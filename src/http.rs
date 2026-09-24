use std::time::Duration;

use reqwest::header::RETRY_AFTER;
use reqwest::{Client, RequestBuilder, Response, StatusCode};

use crate::config::NetworkConfig;
use crate::error::{Error, Result};

const MAX_RETRIES: u32 = 3;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(10);

/// Builds the single HTTP client shared by TMDB API calls and image downloads,
/// so both reuse the same connection pool.
pub fn build_client(network: &NetworkConfig) -> Result<Client> {
    let mut builder = Client::builder()
        .user_agent(concat!("mget/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(network.timeout.max(1)));
    if let Some(proxy) = network.proxy.as_deref().filter(|p| !p.is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    Ok(builder.build()?)
}

/// Sends a request, retrying transient failures (connect errors, timeouts,
/// 429 and 5xx) with exponential backoff. Honors `Retry-After` when present.
///
/// The final response is returned even if its status is an error; callers
/// decide how to interpret it. Transport errors are stripped of their URL,
/// which may contain the API key.
pub async fn send(request: RequestBuilder) -> Result<Response> {
    let mut attempt = 0;
    loop {
        let req = request
            .try_clone()
            .expect("requests without streaming bodies are always cloneable");
        match req.send().await {
            Ok(resp) if attempt < MAX_RETRIES && is_retryable_status(resp.status()) => {
                let delay = retry_after(&resp).unwrap_or_else(|| backoff(attempt));
                tokio::time::sleep(delay).await;
            }
            Ok(resp) => return Ok(resp),
            Err(e) if attempt < MAX_RETRIES && is_transient(&e) => {
                tokio::time::sleep(backoff(attempt)).await;
            }
            Err(e) => return Err(Error::Network(e.without_url())),
        }
        attempt += 1;
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_transient(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request()
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(500 << attempt)
}

fn retry_after(resp: &Response) -> Option<Duration> {
    let secs: u64 = resp
        .headers()
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(secs).min(MAX_RETRY_AFTER))
}
