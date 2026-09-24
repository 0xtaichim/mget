use crate::config::Config;
use crate::error::{Error, Result};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct Downloader {
    client: Client,
    semaphore: Arc<Semaphore>,
}

#[derive(Debug)]
pub struct DownloadTask {
    pub url: String,
    pub path: PathBuf,
    pub description: String,
}

#[derive(Debug)]
pub struct DownloadResult {
    pub path: PathBuf,
    pub description: String,
    pub success: bool,
    pub error: Option<String>,
}

impl Downloader {
    pub fn new(config: &Config) -> Result<Self> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(config.network.timeout));
        if let Some(proxy) = &config.network.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(Error::Network)?);
        }
        let client = builder.build().map_err(Error::Network)?;
        let semaphore = Arc::new(Semaphore::new(config.network.concurrent_downloads as usize));
        Ok(Self { client, semaphore })
    }

    pub async fn download_all(&self, tasks: Vec<DownloadTask>, overwrite: bool) -> Vec<DownloadResult> {
        let mut handles = Vec::new();

        for task in tasks {
            if !overwrite && task.path.exists() {
                handles.push(tokio::spawn(async move {
                    DownloadResult {
                        path: task.path,
                        description: task.description,
                        success: true,
                        error: Some("skipped (already exists)".into()),
                    }
                }));
                continue;
            }

            let client = self.client.clone();
            let semaphore = self.semaphore.clone();

            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await
                    .expect("semaphore should never be closed during download execution");
                match download_file(&client, &task.url, &task.path).await {
                    Ok(()) => DownloadResult {
                        path: task.path,
                        description: task.description,
                        success: true,
                        error: None,
                    },
                    Err(e) => DownloadResult {
                        path: task.path,
                        description: task.description,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        results
    }
}

async fn download_file(client: &Client, url: &str, path: &Path) -> Result<()> {
    // Create parent directories
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut retries = 0;
    loop {
        match client.get(url).send().await {
            Ok(resp) => {
                if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    if retries < 3 {
                        retries += 1;
                        tokio::time::sleep(Duration::from_millis(500 * retries as u64)).await;
                        continue;
                    }
                    return Err(Error::Api("Rate limited".into()));
                }
                if !resp.status().is_success() {
                    return Err(Error::Api(format!("HTTP {}", resp.status())));
                }
                let bytes = resp.bytes().await.map_err(Error::Network)?;
                tokio::fs::write(path, &bytes).await.map_err(|e| Error::FileSystem(e))?;
                return Ok(());
            }
            Err(e) => {
                if retries < 3 {
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(retries as u64)).await;
                    continue;
                }
                return Err(Error::Network(e));
            }
        }
    }
}
