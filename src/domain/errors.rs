use crate::input::tx_parser::ParseTransactionsError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Parse(ParseTransactionsError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Parse(err) => write!(f, "{err}"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Parse(err) => Some(err),
        }
    }
}

impl From<ParseTransactionsError> for AppError {
    fn from(value: ParseTransactionsError) -> Self {
        AppError::Parse(value)
    }
}
