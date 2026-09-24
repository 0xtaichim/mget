#[derive(Debug)]
#[allow(dead_code)]
pub enum ExitCode {
    Success = 0,
    NotFound = 1,
    NetworkError = 2,
    ArgumentError = 3,
    FileSystemError = 4,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::NotFound(_) => ExitCode::NotFound as i32,
            Error::Network(_) | Error::Api(_) => ExitCode::NetworkError as i32,
            Error::Parse(_) | Error::Config(_) => ExitCode::ArgumentError as i32,
            Error::FileSystem(_) | Error::Xml(_) => ExitCode::FileSystemError as i32,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
