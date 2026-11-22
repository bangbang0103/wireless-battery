use crate::error::BatteryError;
use crate::identity;
use crate::model::{
    BatteryReading, Capabilities, ClientConfig, Device, DeviceDiagnostic, DeviceId, DeviceKind,
    Endpoint, EndpointId, Freshness, MatchStatus, Precision, ProviderReading, ReadOptions,
    SupportStatus, TransportKind,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision};
use crate::registry::Registry;
#[cfg(feature = "bluetooth")]
use crate::transport::bluetooth::BluetoothTransport;
use crate::transport::{HidApiTransport, HidSession, Transport, TransportError};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Default)]
struct ClientState {
    readings: HashMap<DeviceId, BatteryReading>,
    known_devices: HashMap<DeviceId, Device>,
    last_errors: HashMap<DeviceId, crate::ErrorKind>,
    last_queries: HashMap<EndpointId, Instant>,
}

/// Synchronous peripheral battery client.
///
/// Endpoint I/O for a given endpoint is serialized. Independent endpoints can
/// proceed concurrently, and no permanent per-device threads are created.
pub struct BatteryClient {
    config: ClientConfig,
    transport: Arc<dyn Transport>,
    registry: Registry,
    state: Mutex<ClientState>,
    endpoint_locks: Mutex<HashMap<EndpointId, Arc<EndpointLock>>>,
}

impl BatteryClient {
    /// Construct a client using real HIDAPI, XInput, and optional BLE BAS transports.
    pub fn new(config: ClientConfig) -> Result<Self, BatteryError> {
        Self::with_registry(config, Registry::builtin()?)
    }

    /// Construct a client using real system transports and a custom provider registry.
    pub fn with_registry(config: ClientConfig, registry: Registry) -> Result<Self, BatteryError> {
        let transport = SystemTransport::new()
            .map_err(|error| error.into_battery(&EndpointId("system".to_owned())))?;
        Self::with_transport(config, Arc::new(transport), registry)
    }

    /// Construct a client with an explicit transport and registry.
    ///
    /// This is the injection point for deterministic replay and mock testing.
    pub fn with_transport(
        config: ClientConfig,
        transport: Arc<dyn Transport>,
        registry: Registry,
    ) -> Result<Self, BatteryError> {
        Ok(Self {
            config,
            transport,
            registry,
            state: Mutex::new(ClientState::default()),
            endpoint_locks: Mutex::new(HashMap::new()),
        })
    }

    /// Access the immutable provider registry backing this client.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Enumerate every endpoint supplied by the operating system.
    ///
    /// Unmatched and ambiguous endpoints are returned with unknown support.
    /// This method never opens a HID session or starts a Bluetooth scan.
    pub fn list_devices(&self) -> Result<Vec<Device>, BatteryError> {
        let endpoints = self
            .transport
            .enumerate()
            .map_err(|error| error.into_battery(&EndpointId("enumeration".to_owned())))?;
        let devices = endpoints
            .iter()
            .map(|endpoint| self.device_from_endpoint(endpoint))
            .collect::<Vec<_>>();
        self.remember_devices(&devices)?;
        Ok(devices)
    }

    /// Read with client defaults and no stale fallback.
    pub fn read_battery(&self, device_id: &str) -> Result<BatteryReading, BatteryError> {
        self.read_battery_with_options(
            device_id,
            ReadOptions {
                timeout: self.config.default_timeout,
                allow_stale: false,
            },
        )
    }

    /// Explicitly read a device, optionally returning its last successful value on failure.
    pub fn read_battery_with_options(
        &self,
        device_id: &str,
        options: ReadOptions,
    ) -> Result<BatteryReading, BatteryError> {
        if options.timeout.is_zero() {
            return Err(BatteryError::timeout("battery read"));
        }
        let deadline = Deadline::after(options.timeout);
        let id = DeviceId(device_id.to_owned());
        let device = self.find_device(&id)?;
        deadline.check("battery read")?;
        let result = self.perform_read(&device, deadline);
        match result {
            Ok(reading) => {
                if self.config.enable_cache {
                    self.lock(&self.state, "client state")?
                        .readings
                        .insert(reading.device_id.clone(), reading.clone());
                }
                Ok(reading)
            }
            Err(error) => {
                let mut state = self.lock(&self.state, "client state")?;
                state.last_errors.insert(device.id.clone(), error.kind());
                if options.allow_stale && self.config.enable_cache {
                    if let Some(mut reading) = state.readings.get(&device.id).cloned() {
                        reading.freshness = Freshness::Stale;
                        return Ok(reading);
                    }
                }
                Err(error)
            }
        }
    }

    /// Collect passive diagnostics without protocol I/O.
    pub fn diagnose(&self, device_id: &str) -> Result<DeviceDiagnostic, BatteryError> {
        let id = DeviceId(device_id.to_owned());
        let device = self.find_device(&id)?;
        let matches = self
            .registry
            .matches(&device.endpoint)
            .into_iter()
            .map(|(provider, decision)| {
                let kind = match decision {
                    MatchDecision::Exact => "exact",
                    MatchDecision::Family => "family",
                    MatchDecision::NoMatch => "no_match",
                };
                format!("{}:{kind}", provider.id())
            })
            .collect();
        let state = self.lock(&self.state, "client state")?;
        let cached_observed_at = state
            .readings
            .get(&device.id)
            .map(|reading| reading.observed_at);
        let last_error = state.last_errors.get(&device.id).copied();
        drop(state);
        Ok(DeviceDiagnostic {
            device_id: device.id,
            provider_id: device.provider_id,
            provider_version: device.provider_version,
            support_status: device.support_status,
            match_status: device.match_status,
            matches,
            capabilities: device.capabilities,
            transport: device.endpoint.transport,
            platform: std::env::consts::OS.to_owned(),
            last_error,
            cached_observed_at,
            permission_hint: permission_hint(),
        })
    }

    fn perform_read(
        &self,
        device: &Device,
        deadline: Deadline,
    ) -> Result<BatteryReading, BatteryError> {
        if device.match_status != MatchStatus::Exact {
            return Err(BatteryError::Unsupported {
                reason: match device.match_status {
                    MatchStatus::Conflict => "multiple providers matched the same priority tier",
                    MatchStatus::Unmatched => "no provider matched",
                    MatchStatus::Exact => "provider match error",
                }
                .to_owned(),
            });
        }
        let provider_id = device
            .provider_id
            .as_ref()
            .ok_or_else(|| BatteryError::Unsupported {
                reason: "matched provider is absent".to_owned(),
            })?;
        let provider = self
            .registry
            .find(provider_id)
            .ok_or_else(|| BatteryError::Internal {
                message: "selected provider disappeared from the immutable registry".to_owned(),
            })?;
        match device.support_status {
            SupportStatus::Verified | SupportStatus::Experimental => {}
            SupportStatus::Unknown => {
                return Err(BatteryError::Unsupported {
                    reason: "provider is not enabled for reads".to_owned(),
                });
            }
        }
        let _permit = self.endpoint_lock(&device.endpoint_id)?.acquire(deadline)?;
        let metadata = provider.metadata();
        self.wait_for_cooldown(&device.endpoint_id, metadata.cooldown, deadline)?;
        let context = DeviceContext {
            device,
            endpoint: &device.endpoint,
        };
        let reading = provider.execute(&context, self.transport.as_ref(), deadline);
        if !metadata.cooldown.is_zero() {
            if let Ok(mut state) = self.state.lock() {
                state
                    .last_queries
                    .insert(device.endpoint_id.clone(), Instant::now());
            }
        }
        self.normalize_reading(device, reading?)
    }

    fn normalize_reading(
        &self,
        device: &Device,
        provider: ProviderReading,
    ) -> Result<BatteryReading, BatteryError> {
        if provider.percentage > Some(100) {
            return Err(BatteryError::invalid_response(
                "provider returned a percentage above 100",
            ));
        }
        let precision = if provider.percentage.is_some() {
            Precision::Percentage
        } else if provider.level.is_some() {
            Precision::DiscreteLevel
        } else if provider.voltage_mv.is_some() {
            Precision::VoltageOnly
        } else {
            Precision::None
        };
        let reachability = if provider.reachability == crate::Reachability::Unknown {
            crate::Reachability::Online
        } else {
            provider.reachability
        };
        Ok(BatteryReading {
            device_id: device.id.clone(),
            endpoint_id: device.endpoint_id.clone(),
            percentage: provider.percentage,
            level: provider.level,
            voltage_mv: provider.voltage_mv,
            charge_state: provider.charge_state,
            reachability,
            precision,
            source: provider.source,
            observed_at: Utc::now(),
            freshness: Freshness::Live,
        })
    }

    fn device_from_endpoint(&self, endpoint: &Endpoint) -> Device {
        let id = identity::device_id(&endpoint.id);
        let matches = self.registry.matches(endpoint);
        let exact = matches
            .iter()
            .filter(|(_, decision)| *decision == MatchDecision::Exact)
            .collect::<Vec<_>>();
        let family = matches
            .iter()
            .filter(|(_, decision)| *decision == MatchDecision::Family)
            .collect::<Vec<_>>();
        let (selected, match_status) = if exact.len() == 1 {
            (Some(exact[0].0.clone()), MatchStatus::Exact)
        } else if exact.len() > 1 {
            (None, MatchStatus::Conflict)
        } else if family.len() == 1 {
            (Some(family[0].0.clone()), MatchStatus::Exact)
        } else if family.len() > 1 {
            (None, MatchStatus::Conflict)
        } else {
            (None, MatchStatus::Unmatched)
        };
        let metadata = selected.as_ref().map(|provider| provider.metadata());
        let support_status = metadata
            .as_ref()
            .map_or(SupportStatus::Unknown, |metadata| metadata.support_status);
        let device_kind = selected
            .as_ref()
            .map_or(DeviceKind::Peripheral, |provider| {
                provider.device_kind(endpoint)
            });
        let readable =
            match_status == MatchStatus::Exact && support_status != SupportStatus::Unknown;
        let default_name = match endpoint.transport {
            TransportKind::BluetoothLe => "Unknown Bluetooth LE device",
            TransportKind::XInput => "XInput controller",
            TransportKind::Hid | TransportKind::Mock => "Unknown HID device",
        };
        let display_name = selected
            .as_ref()
            .and_then(|provider| provider.display_name(endpoint))
            .or_else(|| endpoint.product_name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| default_name.to_owned());
        Device {
            id,
            endpoint_id: endpoint.id.clone(),
            endpoint: endpoint.clone(),
            name: display_name,
            vendor: endpoint.manufacturer.clone(),
            model: endpoint.product_name.clone(),
            provider_id: selected.as_ref().map(|provider| provider.id().to_owned()),
            provider_version: metadata.as_ref().map(|metadata| metadata.version.clone()),
            support_status,
            match_status,
            device_kind,
            capabilities: metadata
                .map_or(Capabilities::default(), |metadata| metadata.capabilities),
            readable,
        }
    }

    fn find_device(&self, id: &DeviceId) -> Result<Device, BatteryError> {
        if let Some(device) = self
            .lock(&self.state, "client state")?
            .known_devices
            .get(id)
            .cloned()
        {
            return Ok(device);
        }
        let devices = self.list_devices()?;
        devices
            .into_iter()
            .find(|device| &device.id == id)
            .ok_or_else(|| BatteryError::NotFound {
                device_id: id.to_string(),
            })
    }

    fn remember_devices(&self, devices: &[Device]) -> Result<(), BatteryError> {
        let current_ids = devices
            .iter()
            .map(|device| device.id.clone())
            .collect::<HashSet<_>>();
        let current_endpoints = devices
            .iter()
            .map(|device| device.endpoint_id.clone())
            .collect::<HashSet<_>>();
        let mut state = self.lock(&self.state, "client state")?;
        let removed_ids = state
            .known_devices
            .keys()
            .filter(|id| !current_ids.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in &removed_ids {
            state.known_devices.remove(id);
            state.readings.remove(id);
            state.last_errors.remove(id);
        }
        state
            .last_queries
            .retain(|endpoint, _| current_endpoints.contains(endpoint));
        for device in devices {
            state
                .known_devices
                .insert(device.id.clone(), device.clone());
        }
        drop(state);
        let mut locks = self.lock(&self.endpoint_locks, "endpoint locks")?;
        locks.retain(|endpoint, lock| {
            current_endpoints.contains(endpoint) || Arc::strong_count(lock) > 1
        });
        Ok(())
    }

    fn endpoint_lock(&self, id: &EndpointId) -> Result<Arc<EndpointLock>, BatteryError> {
        let mut locks = self.lock(&self.endpoint_locks, "endpoint locks")?;
        Ok(locks
            .entry(id.clone())
            .or_insert_with(|| Arc::new(EndpointLock::default()))
            .clone())
    }

    fn wait_for_cooldown(
        &self,
        id: &EndpointId,
        cooldown: Duration,
        deadline: Deadline,
    ) -> Result<(), BatteryError> {
        if cooldown.is_zero() {
            return Ok(());
        }
        let last = self
            .lock(&self.state, "client state")?
            .last_queries
            .get(id)
            .copied();
        if let Some(last) = last {
            let target = last + cooldown;
            loop {
                let now = Instant::now();
                if now >= target {
                    break;
                }
                let remaining = deadline.remaining("endpoint cooldown")?;
                std::thread::sleep((target - now).min(remaining));
            }
        }
        Ok(())
    }

    fn lock<'a, T>(
        &self,
        mutex: &'a Mutex<T>,
        name: &str,
    ) -> Result<MutexGuard<'a, T>, BatteryError> {
        mutex.lock().map_err(|_| BatteryError::Internal {
            message: format!("{name} lock poisoned"),
        })
    }
}

struct SystemTransport {
    hid: HidApiTransport,
    xinput: crate::transport::xinput::XInputTransport,
    #[cfg(feature = "bluetooth")]
    bluetooth: Option<BluetoothTransport>,
}

impl SystemTransport {
    fn new() -> Result<Self, TransportError> {
        Ok(Self {
            hid: HidApiTransport::new()?,
            xinput: crate::transport::xinput::XInputTransport::new(),
            // Degrade gracefully when Bluetooth hardware or permissions are absent
            // so USB/2.4G HID devices remain usable.
            #[cfg(feature = "bluetooth")]
            bluetooth: BluetoothTransport::new().ok(),
        })
    }
}

impl Transport for SystemTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        let mut endpoints = self.hid.enumerate()?;
        endpoints.extend(self.xinput.enumerate_slots()?);
        #[cfg(feature = "bluetooth")]
        if let Some(bluetooth) = &self.bluetooth {
            if let Ok(ble_endpoints) = bluetooth.enumerate_known() {
                endpoints.extend(ble_endpoints);
            }
        }
        Ok(endpoints)
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        self.hid.open(endpoint)
    }

    fn read_bas(
        &self,
        endpoint: &EndpointId,
        timeout: Duration,
    ) -> Result<Option<u8>, TransportError> {
        #[cfg(feature = "bluetooth")]
        return match &self.bluetooth {
            Some(bluetooth) => bluetooth.read_level(endpoint, timeout),
            None => Ok(None),
        };
        #[cfg(not(feature = "bluetooth"))]
        {
            let _ = (endpoint, timeout);
            Ok(None)
        }
    }

    fn read_xinput(
        &self,
        endpoint: &EndpointId,
    ) -> Result<Option<crate::transport::XInputReport>, TransportError> {
        self.xinput.read_slot(endpoint)
    }
}

#[derive(Default)]
struct EndpointLock {
    locked: Mutex<bool>,
    available: Condvar,
}

impl EndpointLock {
    fn acquire(self: &Arc<Self>, deadline: Deadline) -> Result<EndpointPermit, BatteryError> {
        let mut locked = self.locked.lock().map_err(|_| BatteryError::Internal {
            message: "endpoint session lock poisoned".to_owned(),
        })?;
        while *locked {
            let remaining = deadline.remaining("endpoint session lock")?;
            let (guard, wait) = self
                .available
                .wait_timeout(locked, remaining)
                .map_err(|_| BatteryError::Internal {
                    message: "endpoint session lock poisoned".to_owned(),
                })?;
            locked = guard;
            if wait.timed_out() && *locked {
                return Err(BatteryError::timeout("endpoint session lock"));
            }
        }
        *locked = true;
        Ok(EndpointPermit {
            lock: Arc::clone(self),
        })
    }
}

struct EndpointPermit {
    lock: Arc<EndpointLock>,
}

impl Drop for EndpointPermit {
    fn drop(&mut self) {
        if let Ok(mut locked) = self.lock.locked.lock() {
            *locked = false;
            self.lock.available.notify_one();
        }
    }
}

fn permission_hint() -> Option<String> {
    match std::env::consts::OS {
        "linux" => Some(
            "Check HIDRAW permissions and udev rules; BLE may also require BlueZ access.".to_owned(),
        ),
        "macos" => Some(
            "Check Input Monitoring and Bluetooth permissions when the operating system requests them."
                .to_owned(),
        ),
        "windows" => Some(
            "Some HID collections are reserved by Windows; try the vendor usage-page endpoint.".to_owned(),
        ),
        _ => None,
    }
}
