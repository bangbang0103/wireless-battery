//! Standard OS and cross-vendor protocol providers (BLE BAS, Windows XInput).

mod bluetooth_bas;
mod xinput;

use super::Provider;
use std::sync::Arc;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(bluetooth_bas::BluetoothBasProvider),
        Arc::new(xinput::XInputProvider),
    ]
}
