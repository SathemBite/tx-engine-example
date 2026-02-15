use crate::io::input::ParseTransactionsError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Parse(ParseTransactionsError),
    TxProcessing(String),
    TxProcessingNonCritical(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Parse(err) => write!(f, "{err}"),
            AppError::TxProcessing(err) => write!(f, "{err}"),
            AppError::TxProcessingNonCritical(err) => write!(f, "{err}, skipping"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Parse(err) => Some(err),
            AppError::TxProcessing(_) | AppError::TxProcessingNonCritical(_) => None,
        }
    }
}

impl From<ParseTransactionsError> for AppError {
    fn from(value: ParseTransactionsError) -> Self {
        AppError::Parse(value)
    }
}
