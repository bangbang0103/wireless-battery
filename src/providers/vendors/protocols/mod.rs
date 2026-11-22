//! Shared chipset and OEM HID battery protocol families.

pub(crate) mod bolutek;
pub(crate) mod compx;
mod drivercore;
mod evision;

use crate::providers::Provider;
use std::sync::Arc;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    let mut providers = Vec::new();
    providers.extend(bolutek::providers());
    providers.extend(compx::providers());
    providers.extend(drivercore::providers());
    providers.extend(evision::providers());
    providers
}
