use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;


#[derive(Debug, Deserialize)]
struct Transaction {
    id: u32,
    description: String,
    amount: Decimal,
    currency: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "data/transactions.csv".to_string());

    let file = File::open(&input_path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);

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
