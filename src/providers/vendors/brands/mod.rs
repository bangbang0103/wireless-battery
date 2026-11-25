//! Brand- and device-category-organized vendor HID battery providers.

mod asus_omni_headset;
mod asus_peripherals;
mod corsair;
mod dareu;
mod eweadn;
mod gamepads_console;
mod gamepads_pc;
mod headsets_airoha;
mod headsets_audio;
mod hyperx;
mod keyboards_vendors;
mod keychron;
mod logitech;
mod logitech_centurion;
mod mchose_keyboards;
mod mchose_mice;
mod mice_esports;
mod mice_gaming;
mod mice_nordic;
mod motospeed_rongyuan;
mod rapoo;
mod razer_headsets;
mod razer_mice;
mod sony;
mod steelseries_headsets;
mod steelseries_mice;

use crate::providers::Provider;
use std::sync::Arc;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    let mut providers = Vec::new();
    providers.extend(asus_omni_headset::providers());
    providers.extend(asus_peripherals::providers());
    providers.extend(corsair::providers());
    providers.extend(dareu::providers());
    providers.extend(eweadn::providers());
    providers.extend(gamepads_console::providers());
    providers.extend(gamepads_pc::providers());
    providers.extend(headsets_airoha::providers());
    providers.extend(headsets_audio::providers());
    providers.extend(hyperx::providers());
    providers.extend(keyboards_vendors::providers());
    providers.extend(keychron::providers());
    providers.extend(logitech::providers());
    providers.extend(logitech_centurion::providers());
    providers.extend(mchose_keyboards::providers());
    providers.extend(mchose_mice::providers());
    providers.extend(mice_esports::providers());
    providers.extend(mice_gaming::providers());
    providers.extend(mice_nordic::providers());
    providers.extend(motospeed_rongyuan::providers());
    providers.extend(rapoo::providers());
    providers.extend(razer_headsets::providers());
    providers.extend(razer_mice::providers());
    providers.extend(sony::providers());
    providers.extend(steelseries_headsets::providers());
    providers.extend(steelseries_mice::providers());
    providers
}
