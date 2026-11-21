//! Built-in vendor HID protocol providers, organized by vendor or shared chipset protocol.
//!
//! See `provenance/builtin-providers.md` and `NOTICE` for protocol references and attribution.

mod asus;
mod bolutek;
mod compx;
mod corsair;
mod dareu;
mod eweadn;
mod gamepads;
mod headsets;
mod hyperx;
mod keyboards;
mod logitech;
mod mchose;
mod mice_drivercore;
mod mice_enthusiast;
mod misc;
mod razer;
mod sony;
mod steelseries;

use super::Provider;
use std::sync::Arc;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    let mut providers: Vec<Arc<dyn Provider>> = Vec::new();
    providers.extend(asus::providers());
    providers.extend(bolutek::providers());
    providers.extend(compx::providers());
    providers.extend(corsair::providers());
    providers.extend(dareu::providers());
    providers.extend(eweadn::providers());
    providers.extend(gamepads::providers());
    providers.extend(headsets::providers());
    providers.extend(hyperx::providers());
    providers.extend(keyboards::providers());
    providers.extend(logitech::providers());
    providers.extend(mchose::providers());
    providers.extend(mice_drivercore::providers());
    providers.extend(mice_enthusiast::providers());
    providers.extend(misc::providers());
    providers.extend(razer::providers());
    providers.extend(sony::providers());
    providers.extend(steelseries::providers());
    providers
}
