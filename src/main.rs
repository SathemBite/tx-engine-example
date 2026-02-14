use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::BufReader;


#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
struct UserId(u32);

impl Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
struct Currency(String);

impl Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
struct Description(String);

impl Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize)]
struct Transaction {
    id: UserId,
    description: Description,
    amount: Decimal,
    currency: Currency,
}

const REQUIRED_HEADERS: [&str; 4] = ["type", "client", "tx", "amount"];

#[derive(Debug)]
enum AppError {
    Io(std::io::Error),
    Csv(csv::Error),
    InvalidHeaders { expected: String, actual: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "{err}"),
            AppError::Csv(err) => write!(f, "{err}"),
            AppError::InvalidHeaders { expected, actual } => write!(
                f,
                "invalid CSV headers. expected: [{expected}], actual: [{actual}]"
            ),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Io(err) => Some(err),
            AppError::Csv(err) => Some(err),
            AppError::InvalidHeaders { .. } => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value)
    }
}

impl From<csv::Error> for AppError {
    fn from(value: csv::Error) -> Self {
        AppError::Csv(value)
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "data/transactions.csv".to_string());

    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

    let headers = csv_reader.headers()?.clone();
    validate_headers(&headers)?;

    let mut total = dec!(0);

    for record in csv_reader.deserialize::<Transaction>() {
        let tx = record?;
        total += tx.amount;
        println!(
            "#{} | {} | {} {}",
            tx.id, tx.description, tx.amount, tx.currency
        );
    }

    println!("------------------------------");
    println!("Total: {}", total.round_dp(2));

    Ok(())
}

fn validate_headers(headers: &csv::StringRecord) -> Result<(), AppError> {
    if !headers.iter().eq(REQUIRED_HEADERS.iter().copied()) {
        return Err(AppError::InvalidHeaders {
            expected: REQUIRED_HEADERS.join(", "),
            actual: headers.iter().collect::<Vec<_>>().join(", "),
        });
    }

    Ok(())
}
