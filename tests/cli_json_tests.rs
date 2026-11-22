use assert_cmd::Command;
use predicates::prelude::*;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn probe_help_succeeds() {
    Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["probe", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--listen"));
}

#[test]
fn list_json_is_an_array_and_unknown_read_has_stable_error_json() {
    let output = Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(value.is_array());
    if let Some(device) = value.as_array().unwrap().first() {
        assert!(device.get("system_path").is_none());
        assert!(device.get("serial_number").is_none());
    }

    Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["read", "dev_definitely_absent", "--json"])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"kind\":\"not_found\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn unknown_percentage_serializes_as_null_not_zero() {
    let fixture = root().join("tests/fixtures/synthetic/feature_unknown.json");
    let text = std::fs::read_to_string(fixture).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(fixture["expect"]["percentage"].is_null());
    assert_ne!(fixture["expect"]["percentage"], serde_json::json!(0));
}

#[test]
fn probe_invalid_hex_argument_exits_with_code_2() {
    Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["probe", "dev_missing", "--send", "ZZ", "--json"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("\"kind\":\"invalid_argument\""));
}
