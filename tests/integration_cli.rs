use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

fn cmd() -> Command {
    let mut command = Command::cargo_bin("utmd").expect("binary should build");
    command.env("UTMD_UTMCTL_PATH", "/usr/bin/true");
    command.env("UTMD_STATE_PATH", "/tmp/utmd-test-state.json");
    command
}

#[test]
fn delete_all_invalid_older_than_returns_exit_64_and_json_error() {
    let output = cmd()
        .args(["--json", "prune", "--older-than", "abc"])
        .assert()
        .code(64)
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["command"], "prune");
    assert_eq!(payload["ok"], false);
    assert!(payload["error"].as_str().is_some());
}

#[test]
fn status_not_found_returns_exit_66_and_json_error() {
    let output = cmd()
        .args(["--json", "inspect", "missing-vm"])
        .assert()
        .code(66)
        .stdout(predicate::str::contains("\"command\": \"inspect\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], false);
}

#[test]
fn list_returns_wrapped_json_schema() {
    let output = cmd()
        .args(["--json", "ls"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"command\": \"ls\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], true);
    assert!(payload["data"].is_array());
}

#[test]
fn spawn_dry_run_returns_wrapped_json_schema() {
    let output = cmd()
        .args(["--json", "--dry-run", "run", "linux"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"command\": \"run\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["data"]["action"], "run");
    assert!(
        payload["data"]["target"]
            .as_str()
            .is_some_and(|name| name.starts_with("utmd-linux-"))
    );
}

#[test]
fn init_dry_run_returns_wrapped_json_schema() {
    let output = cmd()
        .args(["--json", "--dry-run", "init"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("\"command\": \"init\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["data"]["action"], "init");
}

#[test]
fn init_existing_file_without_force_returns_exit_65() {
    let config_path =
        std::env::temp_dir().join(format!("utmd-test-config-{}.toml", std::process::id()));
    fs::write(&config_path, "default_prefix = \"utmd-\"\n").expect("should create temp config");

    let output = cmd()
        .args([
            "--json",
            "--config",
            config_path.to_string_lossy().as_ref(),
            "init",
        ])
        .assert()
        .code(65)
        .stdout(predicate::str::contains("\"command\": \"init\""))
        .get_output()
        .stdout
        .clone();

    let payload: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(payload["ok"], false);
    assert!(payload["error"].as_str().is_some());

    fs::remove_file(config_path).expect("should remove temp config");
}
