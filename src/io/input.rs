use serde::Deserialize;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{BufReader, Read};

use crate::domain::types::{Amount, ClientId, TransactionType, TxID};

#[derive(Debug, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub op_type: TransactionType,
    pub client: ClientId,
    #[serde(rename = "tx")]
    pub tx_id: TxID,
    pub amount: Option<Amount>,
}

pub type TransactionRecords = csv::DeserializeRecordsIntoIter<BufReader<File>, Transaction>;
pub type TransactionRecordsFromReader<R> = csv::DeserializeRecordsIntoIter<R, Transaction>;

#[derive(Debug)]
pub enum ParseTransactionsError {
    Io(std::io::Error),
    Csv(csv::Error),
}

impl Display for ParseTransactionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseTransactionsError::Io(err) => write!(f, "{err}"),
            ParseTransactionsError::Csv(err) => write!(f, "{err}"),
        }
    }
}

impl Error for ParseTransactionsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseTransactionsError::Io(err) => Some(err),
            ParseTransactionsError::Csv(err) => Some(err),
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

pub fn parse_transactions_from_reader<R: Read>(reader: R) -> TransactionRecordsFromReader<R> {
    let csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    csv_reader.into_deserialize::<Transaction>()
}

pub fn parse_transactions(input_path: &str) -> Result<TransactionRecords, ParseTransactionsError> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);

    Ok(parse_transactions_from_reader(reader))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::io::Cursor;

    #[test]
    fn parses_row_with_whitespace_and_amount() {
        let csv = "\
type, client, tx, amount
deposit, 1, 10, 1.2345
";
        let cursor = Cursor::new(csv.as_bytes());

        let mut iter = parse_transactions_from_reader(cursor);
        let tx = iter
            .next()
            .expect("one row is expected")
            .expect("row must parse");

        assert_eq!(tx.op_type, TransactionType::Deposit);
        assert_eq!(tx.client, ClientId(1));
        assert_eq!(tx.tx_id, TxID(10));
        assert_eq!(tx.amount, Some(Amount::new(dec!(1.2345))));
    }

    #[test]
    fn parses_dispute_with_empty_amount_as_none() {
        let csv = "\
type,client,tx,amount
dispute,5,42,
";
        let cursor = Cursor::new(csv.as_bytes());

        let mut iter = parse_transactions_from_reader(cursor);
        let tx = iter
            .next()
            .expect("one row is expected")
            .expect("row must parse");

        assert_eq!(tx.op_type, TransactionType::Dispute);
        assert_eq!(tx.client, ClientId(5));
        assert_eq!(tx.tx_id, TxID(42));
        assert_eq!(tx.amount, None);
    }

    #[test]
    fn returns_io_error_for_missing_file() {
        let missing_path = std::env::temp_dir()
            .join("definitely_missing_transactions_file.csv")
            .to_string_lossy()
            .into_owned();

        let result = parse_transactions(&missing_path);
        match result {
            Err(ParseTransactionsError::Io(_)) => {}
            Err(ParseTransactionsError::Csv(_)) => panic!("expected io error, got csv error"),
            Ok(_) => panic!("expected io error, got success"),
        }
    }

    #[test]
    fn yields_csv_error_on_invalid_record() {
        let csv = "\
type,client,tx,amount
deposit,abc,1,1.0
";
        let cursor = Cursor::new(csv.as_bytes());

        let mut iter = parse_transactions_from_reader(cursor);
        let row_result = iter.next().expect("one row is expected");

        assert!(row_result.is_err());
    }
}
