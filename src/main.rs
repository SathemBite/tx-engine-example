mod domain;
mod input;

use domain::errors::AppError;
use input::tx_parser::{parse_transactions, ParseTransactionsError};
use rust_decimal_macros::dec;
use std::env;

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

    let mut total = dec!(0);

    for tx in parse_transactions(&input_path)? {
        let tx = tx.map_err(ParseTransactionsError::from)?;
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
