use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn cmd() -> Command {
    let mut command = Command::cargo_bin("utmd").expect("binary should build");
    command.env("UTMD_UTMCTL_PATH", "/usr/bin/true");
    command.env("UTMD_STATE_PATH", "/tmp/utmd-test-state.json");
    command
}

#[test]
fn delete_all_invalid_older_than_returns_exit_2_and_json_error() {
    let output = cmd()
        .args(["--json", "delete-all", "--older-than", "abc"])
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["command"], "delete-all");
    assert_eq!(payload["ok"], false);
    assert!(payload["error"].as_str().is_some());
}

#[test]
fn status_not_found_returns_exit_4_and_json_error() {
    let output = cmd()
        .args(["--json", "status", "missing-vm"])
        .assert()
        .code(4)
        .stdout(predicate::str::contains("\"command\": \"status\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], false);
}

#[test]
fn list_returns_wrapped_json_schema() {
    let output = cmd()
        .args(["--json", "list"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"command\": \"list\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], true);
    assert!(payload["data"].is_array());
}
