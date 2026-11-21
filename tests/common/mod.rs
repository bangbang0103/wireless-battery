#![allow(dead_code)]

use std::path::{Path, PathBuf};
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::{
    assert_expectation, run_fixture as run_replay_fixture, Fixture, ReplayTransport,
};
use wireless_battery::{BatteryError, BatteryReading};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn fixture(name: &str) -> Fixture {
    load_fixture(&root().join("tests/fixtures/synthetic").join(name))
}

pub fn load_fixture(path: &Path) -> Fixture {
    let text = std::fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&text).expect("fixture should be valid JSON")
}

pub fn run_fixture(fixture: &Fixture) -> (Result<BatteryReading, BatteryError>, ReplayTransport) {
    run_replay_fixture(
        fixture,
        Registry::with_synthetic().expect("synthetic registry"),
    )
    .expect("fixture setup should match target provider")
}

pub fn assert_expected(fixture: &Fixture, result: &Result<BatteryReading, BatteryError>) {
    assert_expectation(&fixture.expect, result, &fixture.case)
        .unwrap_or_else(|error| panic!("{}: {error}", fixture.case));
}
