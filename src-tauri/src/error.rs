use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IMAP error: {0}")]
    Imap(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Account limit reached (max 5 accounts)")]
    AccountLimit,

    #[error("Account not found")]
    AccountNotFound,

    #[error("Email not found")]
    EmailNotFound,

    #[error("Tag not found")]
    TagNotFound,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<imap::Error> for AppError {
    fn from(e: imap::Error) -> Self {
        AppError::Imap(e.to_string())
    }
}

impl From<mail_parser::Error> for AppError {
    fn from(e: mail_parser::Error) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl From<chrono::ParseError> for AppError {
    fn from(e: chrono::ParseError) -> Self {
        AppError::Parse(e.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
