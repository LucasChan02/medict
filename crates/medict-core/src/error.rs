use std::io;

#[derive(Debug, thiserror::Error)]
pub enum MedictError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("dictionary parse error: {0}")]
    Mdict(#[from] mdictlib::Error),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("archive error: {0}")]
    Archive(String),
}

pub type Result<T> = std::result::Result<T, MedictError>;

impl From<toml::de::Error> for MedictError {
    fn from(error: toml::de::Error) -> Self {
        Self::Config(error.to_string())
    }
}

impl From<toml::ser::Error> for MedictError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Config(error.to_string())
    }
}

impl From<serde_json::Error> for MedictError {
    fn from(error: serde_json::Error) -> Self {
        Self::Message(error.to_string())
    }
}

impl From<zip::result::ZipError> for MedictError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Archive(error.to_string())
    }
}

impl From<ureq::Error> for MedictError {
    fn from(error: ureq::Error) -> Self {
        Self::Network(error.to_string())
    }
}
