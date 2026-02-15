use serde::Deserialize;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::BufReader;

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

pub fn parse_transactions(input_path: &str) -> Result<TransactionRecords, ParseTransactionsError> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    Ok(csv_reader.into_deserialize::<Transaction>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_csv(test_name: &str, content: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("input_parser_{test_name}_{nanos}.csv"));
        fs::write(&path, content).expect("must write temp csv");
        path
    }

    #[test]
    fn parses_row_with_whitespace_and_amount() {
        let csv = "\
type, client, tx, amount
deposit, 1, 10, 1.2345
";
        let path = write_temp_csv("whitespace", csv);
        let path_str = path.to_string_lossy().into_owned();

        let mut iter = parse_transactions(&path_str).expect("must create csv iterator");
        let tx = iter
            .next()
            .expect("one row is expected")
            .expect("row must parse");
        fs::remove_file(path).expect("must remove temp file");

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
        let path = write_temp_csv("dispute_none_amount", csv);
        let path_str = path.to_string_lossy().into_owned();

        let mut iter = parse_transactions(&path_str).expect("must create csv iterator");
        let tx = iter
            .next()
            .expect("one row is expected")
            .expect("row must parse");
        fs::remove_file(path).expect("must remove temp file");

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
        let path = write_temp_csv("invalid_record", csv);
        let path_str = path.to_string_lossy().into_owned();

        let mut iter = parse_transactions(&path_str).expect("must create csv iterator");
        let row_result = iter.next().expect("one row is expected");
        fs::remove_file(path).expect("must remove temp file");

        assert!(row_result.is_err());
    }
}
