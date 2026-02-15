pub mod domain;
pub mod io;
pub mod tx_engine;

use domain::errors::AppError;
use io::input::{parse_transactions, ParseTransactionsError};
use io::output::print_clients_snapshot;
use std::env;
use tx_engine::TxEngine;

fn main() {
    env_logger::init();
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), AppError> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(AppError::TxProcessing(
            "Usage: cargo run -- <transactions.csv>".to_string(),
        ));
    }
    let input_path = &args[1];

    let mut tx_engine = TxEngine::new();

    for tx_result in parse_transactions(input_path)? {
        let tx = tx_result.map_err(ParseTransactionsError::from)?;
        if let Err(err) = tx_engine.process_transaction(&tx) {
            match err {
                AppError::TxProcessingNonCritical(_) => {
                    log::warn!("{err}");
                    continue;
                }
                _ => return Err(err),
            }
        }
    }

    let snapshots = tx_engine.clients_snapshot();
    print_clients_snapshot(&snapshots);

    Ok(())
}
