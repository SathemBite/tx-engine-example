use rust_decimal::Decimal;
use serde::Deserialize;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::BufReader;

const REQUIRED_HEADERS: [&str; 4] = ["type", "client", "tx", "amount"];

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub u32);

impl Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Currency(pub String);

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Description(pub String);

impl Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub id: UserId,
    pub description: Description,
    pub amount: Decimal,
    pub currency: Currency,
}

pub type TransactionRecords = csv::DeserializeRecordsIntoIter<BufReader<File>, Transaction>;

#[derive(Debug)]
pub enum ParseTransactionsError {
    Io(std::io::Error),
    Csv(csv::Error),
    InvalidHeaders { expected: String, actual: String },
}

impl Display for ParseTransactionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseTransactionsError::Io(err) => write!(f, "{err}"),
            ParseTransactionsError::Csv(err) => write!(f, "{err}"),
            ParseTransactionsError::InvalidHeaders { expected, actual } => write!(
                f,
                "invalid CSV headers. expected: [{expected}], actual: [{actual}]"
            ),
        }
    }
}

impl Error for ParseTransactionsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseTransactionsError::Io(err) => Some(err),
            ParseTransactionsError::Csv(err) => Some(err),
            ParseTransactionsError::InvalidHeaders { .. } => None,
        }
    }
}

impl From<std::io::Error> for ParseTransactionsError {
    fn from(value: std::io::Error) -> Self {
        ParseTransactionsError::Io(value)
    }
}

impl From<csv::Error> for ParseTransactionsError {
    fn from(value: csv::Error) -> Self {
        ParseTransactionsError::Csv(value)
    }
}

pub fn parse_transactions(
    input_path: &str,
) -> Result<TransactionRecords, ParseTransactionsError> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    validate_headers(csv_reader.headers()?)?;

    Ok(csv_reader.into_deserialize::<Transaction>())
}

fn validate_headers(headers: &csv::StringRecord) -> Result<(), ParseTransactionsError> {
    if !headers.iter().eq(REQUIRED_HEADERS.iter().copied()) {
        return Err(ParseTransactionsError::InvalidHeaders {
            expected: REQUIRED_HEADERS.join(", "),
            actual: headers.iter().collect::<Vec<_>>().join(", "),
        });
    }

    Ok(())
}
