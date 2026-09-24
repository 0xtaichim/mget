//! Collects the files a fetch should produce, then writes them in one pass.
//!
//! Separating planning from execution keeps the TMDB-to-file mapping free of
//! I/O concerns, lets `--dry-run` report exactly what would happen (including
//! skips), and lets all image downloads run concurrently.

use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

use futures_util::{StreamExt, stream};
use reqwest::Client;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::{fsutil, http};

enum Payload {
    Nfo(String),
    Image { url: String },
}

struct Item {
    path: PathBuf,
    label: String,
    payload: Payload,
}

#[derive(Default)]
pub struct Plan {
    items: Vec<Item>,
    failures: Vec<Failure>,
}

pub struct ExecOptions {
    pub overwrite: bool,
    pub dry_run: bool,
    pub concurrency: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub files_written: Vec<String>,
    pub files_skipped: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files_planned: Vec<String>,
    pub errors: Vec<Failure>,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub file: String,
    pub error: String,
}

impl Plan {
    pub fn nfo(&mut self, path: PathBuf, content: String) {
        self.items.push(Item {
            path,
            label: "nfo".into(),
            payload: Payload::Nfo(content),
        });
    }

    pub fn image(&mut self, path: PathBuf, url: String, label: impl Into<String>) {
        self.items.push(Item {
            path,
            label: label.into(),
            payload: Payload::Image { url },
        });
    }

    /// Records a failure that happened while planning (e.g. a season that could not be fetched).
    pub fn fail(&mut self, target: impl Into<String>, error: impl Display) {
        self.failures.push(Failure {
            file: target.into(),
            error: error.to_string(),
        });
    }

    pub async fn execute(self, client: &Client, opts: &ExecOptions) -> Report {
        let mut report = Report {
            errors: self.failures,
            ..Report::default()
        };
        let mut downloads = Vec::new();

        for item in self.items {
            let shown = item.path.display().to_string();
            if !opts.overwrite && item.path.exists() {
                eprintln!("Skipped (exists): {shown}");
                report.files_skipped.push(shown);
                continue;
            }
            if opts.dry_run {
                match &item.payload {
                    Payload::Nfo(_) => eprintln!("[dry-run] Would write: {shown}"),
                    Payload::Image { .. } => {
                        eprintln!("[dry-run] Would download {} -> {shown}", item.label);
                    }
                }
                report.files_planned.push(shown);
                continue;
            }
            match item.payload {
                Payload::Nfo(content) => match fsutil::write_atomic(&item.path, content.as_bytes())
                {
                    Ok(()) => {
                        eprintln!("Written: {shown}");
                        report.files_written.push(shown);
                    }
                    Err(e) => report.record_failure(&item.label, shown, e),
                },
                Payload::Image { url } => downloads.push((item.path, item.label, url)),
            }
        }

        let results: Vec<_> = stream::iter(downloads)
            .map(|(path, label, url)| async move {
                let result = download(client, &url, &path).await;
                (path, label, result)
            })
            .buffered(opts.concurrency.max(1))
            .collect()
            .await;

        for (path, label, result) in results {
            let shown = path.display().to_string();
            match result {
                Ok(()) => {
                    eprintln!("Downloaded: {label} -> {shown}");
                    report.files_written.push(shown);
                }
                Err(e) => report.record_failure(&label, shown, e),
            }
        }
        report
    }
}

impl Report {
    fn record_failure(&mut self, label: &str, file: String, error: impl Display) {
        eprintln!("Failed: {label} - {error}");
        self.errors.push(Failure {
            file,
            error: error.to_string(),
        });
    }

    pub fn print_summary(&self) {
        if self.files_planned.is_empty() {
            eprintln!(
                "\nDone: {} written, {} skipped, {} errors",
                self.files_written.len(),
                self.files_skipped.len(),
                self.errors.len()
            );
        } else {
            eprintln!(
                "\nDry run: {} planned, {} skipped, {} errors",
                self.files_planned.len(),
                self.files_skipped.len(),
                self.errors.len()
            );
        }
    }
}

async fn download(client: &Client, url: &str, path: &Path) -> Result<()> {
    let resp = http::send(client.get(url)).await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Api(format!("HTTP {status}")));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Error::Network(e.without_url()))?;
    if bytes.is_empty() {
        return Err(Error::Api("empty response body".into()));
    }
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || fsutil::write_atomic(&path, &bytes))
        .await
        .map_err(io::Error::other)??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(overwrite: bool, dry_run: bool) -> ExecOptions {
        ExecOptions {
            overwrite,
            dry_run,
            concurrency: 2,
        }
    }

    #[tokio::test]
    async fn writes_skips_and_dry_runs() {
        let dir = std::env::temp_dir().join(format!("mget-plan-{}", std::process::id()));
        let existing = dir.join("old.nfo");
        let fresh = dir.join("sub/new.nfo");
        fsutil::write_atomic(&existing, b"old").unwrap();
        let client = Client::new();

        let make_plan = || {
            let mut plan = Plan::default();
            plan.nfo(existing.clone(), "new".into());
            plan.nfo(fresh.clone(), "new".into());
            plan.fail("Season 9", "boom");
            plan
        };

        let dry = make_plan().execute(&client, &opts(false, true)).await;
        assert_eq!(dry.files_skipped.len(), 1);
        assert_eq!(dry.files_planned.len(), 1);
        assert!(dry.files_written.is_empty());
        assert_eq!(dry.errors.len(), 1);
        assert!(!fresh.exists());

        let real = make_plan().execute(&client, &opts(false, false)).await;
        assert_eq!(real.files_written.len(), 1);
        assert_eq!(std::fs::read_to_string(&fresh).unwrap(), "new");
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "old");

        let forced = make_plan().execute(&client, &opts(true, false)).await;
        assert_eq!(forced.files_written.len(), 2);
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "new");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
