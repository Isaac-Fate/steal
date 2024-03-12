#[derive(Debug)]
pub enum Error {
    ReqwestError(reqwest::Error),
    IOError(std::io::Error),
    TokioError(tokio::task::JoinError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReqwestError(err) => write!(f, "ReqwestError: {}", err),
            Error::IOError(err) => write!(f, "IOError: {}", err),
            Error::TokioError(err) => write!(f, "TokioError: {}", err),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::ReqwestError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::TokioError(err)
    }
}
