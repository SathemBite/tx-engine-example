use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_csv_path(test_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tx_engine_{test_name}_{nanos}.csv"))
}

fn run_engine_with_csv(test_name: &str, csv_input: &str) -> (String, String) {
    let path = unique_csv_path(test_name);
    fs::write(&path, csv_input).expect("must write input csv");

    let output = Command::new(env!("CARGO_BIN_EXE_tx-engine-example"))
        .arg(&path)
        .output()
        .expect("must run tx-engine-example binary");

    fs::remove_file(&path).expect("must remove temp csv");

    assert!(output.status.success(), "binary should exit successfully");
    (
        String::from_utf8(output.stdout).expect("stdout must be utf8"),
        String::from_utf8(output.stderr).expect("stderr must be utf8"),
    )
}

#[test]
fn e2e_single_client_happy_flow() {
    let input = "\
type,client,tx,amount
deposit,1,1,5.0
withdrawal,1,2,1.5
dispute,1,1,
resolve,1,1,
";

    let (stdout, _stderr) = run_engine_with_csv("happy_flow", input);
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines[0], "client,available,held,total,locked");
    assert!(lines.contains(&"1,3.5000,0.0000,3.5000,false"));
    assert_eq!(lines.len(), 2);
}

#[test]
fn e2e_corner_cases_duplicate_unknown_client_and_frozen_account() {
    let input = "\
type,client,tx,amount
deposit,1,1,2.0
deposit,2,1,3.0
dispute,1,1,
chargeback,1,1,
deposit,1,2,1.0
resolve,77,1,
";

    let (stdout, _stderr) = run_engine_with_csv("corner_cases", input);

    assert!(stdout.contains("client,available,held,total,locked"));
    assert!(stdout.contains("1,0.0000,0.0000,0.0000,true"));
    assert!(!stdout.contains("\n2,"));
    assert!(!stdout.contains("\n77,"));
}
