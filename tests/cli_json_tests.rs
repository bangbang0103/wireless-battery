use assert_cmd::Command;
use predicates::prelude::*;

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn replay_command_runs_without_hid_and_validate_succeeds() {
    let fixture = root().join("fixtures/synthetic/feature_normal.json");
    Command::cargo_bin("wireless-battery")
        .unwrap()
        .current_dir(root())
        .args(["replay", "--fixture"])
        .arg(fixture)
        .assert()
        .success()
        .stdout(predicate::str::contains("feature_charging_72_percent"));

    let output = Command::cargo_bin("wireless-battery")
        .unwrap()
        .current_dir(root())
        .arg("validate")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let fixture_count = text
        .split_whitespace()
        .zip(text.split_whitespace().skip(1))
        .find_map(|(count, label)| (label == "fixture(s)").then(|| count.parse::<usize>().ok())?)
        .expect("validate output should include fixture count");
    assert!(
        fixture_count >= 35,
        "expected at least 35 fixtures, got {fixture_count}"
    );
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
    let fixture = root().join("fixtures/synthetic/feature_unknown.json");
    let text = std::fs::read_to_string(fixture).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(fixture["expect"]["percentage"].is_null());
    assert_ne!(fixture["expect"]["percentage"], serde_json::json!(0));
}

#[test]
fn malformed_replay_exits_with_validation_code() {
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"{\"schema_version\":1}").unwrap();
    Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["replay", "--fixture"])
        .arg(file.path())
        .assert()
        .code(6)
        .stderr(predicate::str::contains("invalid fixture"));
}

#[test]
fn replay_rejects_fixture_whose_endpoint_matches_different_provider() {
    use std::io::Write;
    let fixture_path = root().join("fixtures/synthetic/feature_normal.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_path).unwrap()).unwrap();
    value["target"]["kind"] = serde_json::json!("provider");
    value["target"]["id"] = serde_json::json!("wob.rainy75");
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(serde_json::to_string(&value).unwrap().as_bytes())
        .unwrap();
    Command::cargo_bin("wireless-battery")
        .unwrap()
        .args(["replay", "--fixture"])
        .arg(file.path())
        .assert()
        .code(6)
        .stderr(predicate::str::contains("expected target provider"));
}
