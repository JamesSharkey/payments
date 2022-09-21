use assert_cmd::Command;
use std::process::Output;

#[test]
fn missing_file_arg() {
    let mut cmd = Command::cargo_bin("payments").unwrap();
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("Missing filepath argument"));
}

#[test]
fn missing_file() {
    let output = run("./tests/this_file_does_not_exist.csv");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("The system cannot find the file specified."));
}

#[test]
fn empty_file() {
    let output = run("./tests/empty.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{stdout}");
    assert!(output.status.success());
    assert_eq!(stdout, expect(&[]));
}

#[test]
fn junk_file() {
    let output = run("./tests/junk.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&[]));
}

#[test]
fn some_junk() {
    let output = run("./tests/some_junk.csv");
    let stderr = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stderr, expect(&["1,1.5000,0.0000,1.5000,false"]));
}

#[test]
fn headers() {
    let output = run("./tests/headers.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&[]));
}

#[test]
fn deposit_and_withdraw() {
    let output = run("./tests/deposit_and_withdraw.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&["1,1.5000,0.0000,1.5000,false"]));
}

#[test]
fn whitespace() {
    let output = run("./tests/whitespace.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&["1,1.5000,0.0000,1.5000,false"]));
}

#[test]
fn precision() {
    let output = run("./tests/precision.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&["1,2.2099,0.0000,2.2099,false"]));
}

#[test]
fn chargeback() {
    let output = run("./tests/chargeback.csv");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert_eq!(stdout, expect(&["0,5.0000,0.0000,5.0000,true"]));
}

fn run(file: &str) -> Output {
    let mut cmd = Command::cargo_bin("payments").unwrap();
    cmd.arg(file).output().unwrap()
}

fn expect(expected_accounts: &[&str]) -> String {
    let mut expect = String::from("client,available,held,total,locked\n");
    for i in expected_accounts {
        expect.push_str(i);
        expect.push('\n');
    }

    expect
}
