//! CLI wrapper for strict JSON session replay.

use super::validate::load_valid_fixture;
use std::path::Path;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::verify_fixture;
use wireless_battery::BatteryError;

pub(super) fn run(path: &Path) -> Result<(), BatteryError> {
    let fixture = load_valid_fixture(path)?;
    verify_fixture(&fixture, Registry::with_synthetic()?)?;
    println!("ok\t{}\t{}", fixture.case, path.display());
    Ok(())
}
