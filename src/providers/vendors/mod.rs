//! Built-in vendor HID protocol providers, organized by shared chipset protocol or brand.
//!
//! See `provenance/builtin-providers.md` and `NOTICE` for protocol references and attribution.

mod brands;
mod protocols;

use super::Provider;
use std::sync::Arc;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    let mut providers = Vec::new();
    providers.extend(protocols::providers());
    providers.extend(brands::providers());
    providers
}
