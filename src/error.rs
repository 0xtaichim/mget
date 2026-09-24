use std::process::ExitCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("File system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0} item(s) failed")]
    Incomplete(usize),
}

impl Error {
    pub fn exit_code(&self) -> ExitCode {
        let code = match self {
            Error::NotFound(_) => 1,
            Error::Network(_) | Error::Api(_) => 2,
            Error::Config(_) => 3,
            Error::Io(_) | Error::Json(_) => 4,
            Error::Incomplete(_) => 5,
        };
        ExitCode::from(code)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
