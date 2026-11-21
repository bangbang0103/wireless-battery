//! Optional read-only Bluetooth LE Battery Service transport.
//!
//! Enumeration never starts a scan. Connection and GATT service discovery occur
//! only for an explicit BAS read.

use super::{classify_hid_error, TransportError};
use crate::identity;
use crate::model::{Endpoint, EndpointId, TransportKind};
use btleplug::api::{Central, Manager as _, Peripheral as _};
use btleplug::platform::{Manager, PeripheralId};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;
use uuid::Uuid;

const BATTERY_LEVEL: Uuid = Uuid::from_u128(0x0000_2A19_0000_1000_8000_0080_5F9B_34FB);

/// Bluetooth transport backed by one bounded worker thread and async runtime.
pub struct BluetoothTransport {
    sender: mpsc::Sender<Command>,
}

enum Command {
    Enumerate(mpsc::Sender<Result<Vec<Endpoint>, TransportError>>),
    Read {
        endpoint: EndpointId,
        timeout: Duration,
        result: mpsc::Sender<Result<Option<u8>, TransportError>>,
    },
}

impl BluetoothTransport {
    /// Start the single Bluetooth worker without scanning for devices.
    pub fn new() -> Result<Self, TransportError> {
        let (sender, receiver) = mpsc::channel();
        let (startup, startup_result) = mpsc::channel();
        std::thread::Builder::new()
            .name("wireless-battery-bluetooth".to_owned())
            .spawn(move || worker(receiver, startup))
            .map_err(|error| {
                TransportError::Message(format!("cannot start Bluetooth worker: {error}"))
            })?;
        startup_result.recv().map_err(|_| {
            TransportError::Message("Bluetooth worker stopped during startup".to_owned())
        })??;
        Ok(Self { sender })
    }

    /// Enumerate peripherals already known to operating-system adapters.
    pub(crate) fn enumerate_known(&self) -> Result<Vec<Endpoint>, TransportError> {
        let (result, receiver) = mpsc::channel();
        self.sender
            .send(Command::Enumerate(result))
            .map_err(|_| TransportError::Message("Bluetooth worker is unavailable".to_owned()))?;
        receiver
            .recv()
            .map_err(|_| TransportError::Message("Bluetooth worker stopped".to_owned()))?
    }

    /// Explicitly connect if necessary and read BAS 0x2A19.
    pub(crate) fn read_level(
        &self,
        endpoint: &EndpointId,
        timeout: Duration,
    ) -> Result<Option<u8>, TransportError> {
        let (result, receiver) = mpsc::channel();
        self.sender
            .send(Command::Read {
                endpoint: endpoint.clone(),
                timeout,
                result,
            })
            .map_err(|_| TransportError::Message("Bluetooth worker is unavailable".to_owned()))?;
        match receiver.recv_timeout(timeout.saturating_add(Duration::from_secs(1))) {
            Ok(outcome) => outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(TransportError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(TransportError::Message(
                "Bluetooth worker stopped".to_owned(),
            )),
        }
    }
}

fn worker(receiver: mpsc::Receiver<Command>, startup: mpsc::Sender<Result<(), TransportError>>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = startup.send(Err(TransportError::Message(format!(
                "cannot initialize Bluetooth runtime: {error}"
            ))));
            return;
        }
    };
    let manager = match runtime.block_on(Manager::new()) {
        Ok(manager) => manager,
        Err(error) => {
            let _ = startup.send(Err(classify_hid_error(error.to_string())));
            return;
        }
    };
    if startup.send(Ok(())).is_err() {
        return;
    }
    let mut endpoints = HashMap::<EndpointId, PeripheralId>::new();
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Enumerate(result) => {
                let response = runtime.block_on(enumerate_async(&manager, &mut endpoints));
                let _ = result.send(response);
            }
            Command::Read {
                endpoint,
                timeout,
                result,
            } => {
                let peripheral_id = endpoints.get(&endpoint).cloned();
                let response = match peripheral_id {
                    Some(id) => runtime.block_on(read_async(&manager, id, timeout)),
                    None => Err(TransportError::DeviceDisconnected),
                };
                let _ = result.send(response);
            }
        }
    }
}

async fn enumerate_async(
    manager: &Manager,
    known: &mut HashMap<EndpointId, PeripheralId>,
) -> Result<Vec<Endpoint>, TransportError> {
    let adapters = manager
        .adapters()
        .await
        .map_err(|error| classify_hid_error(error.to_string()))?;
    let mut endpoints = Vec::new();
    let mut active = HashMap::new();
    for adapter in adapters {
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|error| classify_hid_error(error.to_string()))?;
        for peripheral in peripherals {
            let peripheral_id = peripheral.id();
            let stable_key = format!("bluetooth:{peripheral_id:?}");
            let id = identity::endpoint_id(&stable_key);
            let properties = peripheral
                .properties()
                .await
                .map_err(|error| classify_hid_error(error.to_string()))?;
            active.insert(id.clone(), peripheral_id);
            endpoints.push(Endpoint {
                id,
                transport: TransportKind::BluetoothLe,
                vid: None,
                pid: None,
                usage_page: None,
                usage: None,
                interface_number: None,
                bus_type: None,
                input_report_length: None,
                output_report_length: None,
                feature_report_length: None,
                product_name: properties.and_then(|value| value.local_name),
                manufacturer: None,
                system_path: stable_key,
            });
        }
    }
    *known = active;
    Ok(endpoints)
}

async fn read_async(
    manager: &Manager,
    peripheral_id: PeripheralId,
    timeout: Duration,
) -> Result<Option<u8>, TransportError> {
    let mut connected_by_us = None;
    let operation = async {
        let adapters = manager
            .adapters()
            .await
            .map_err(|error| classify_hid_error(error.to_string()))?;
        let mut selected = None;
        for adapter in adapters {
            for peripheral in adapter
                .peripherals()
                .await
                .map_err(|error| classify_hid_error(error.to_string()))?
            {
                if peripheral.id() == peripheral_id {
                    selected = Some(peripheral);
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let peripheral = selected.ok_or(TransportError::DeviceDisconnected)?;
        let was_connected = peripheral
            .is_connected()
            .await
            .map_err(|error| classify_hid_error(error.to_string()))?;
        if !was_connected {
            peripheral
                .connect()
                .await
                .map_err(|error| classify_hid_error(error.to_string()))?;
            connected_by_us = Some(peripheral.clone());
        }
        peripheral
            .discover_services()
            .await
            .map_err(|error| classify_hid_error(error.to_string()))?;
        let characteristic = peripheral
            .services()
            .into_iter()
            .flat_map(|service| service.characteristics.into_iter())
            .find(|characteristic| characteristic.uuid == BATTERY_LEVEL)
            .ok_or_else(|| TransportError::Unsupported("Battery Service is absent".to_owned()))?;
        let value = peripheral
            .read(&characteristic)
            .await
            .map_err(|error| classify_hid_error(error.to_string()))?;
        match value.as_slice() {
            [percentage @ 0..=100] => Ok(Some(*percentage)),
            [_] => Err(TransportError::InvalidResponse(
                "BAS percentage exceeds 100".to_owned(),
            )),
            _ => Err(TransportError::InvalidResponse(
                "BAS Battery Level must contain exactly one byte".to_owned(),
            )),
        }
    };
    let result = tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| TransportError::Timeout)?;
    if let Some(peripheral) = connected_by_us {
        let _ = tokio::time::timeout(Duration::from_millis(500), peripheral.disconnect()).await;
    }
    result
}
