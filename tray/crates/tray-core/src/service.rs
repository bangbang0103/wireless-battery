//! Background polling coordinator and immutable snapshot builder.

use crate::config::{default_config_path, load_config, save_config, TrayConfig};
use crate::device_kind::classify_device;
use crate::notify::LowBatteryTracker;
use crate::state::{format_tooltip, DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use wireless_battery::{
    BatteryClient, BatteryError, BatteryReading, ChargeState, ClientConfig, Device, ErrorKind,
    Freshness, HidBusType, MatchStatus, Precision, Reachability, ReadOptions, SupportStatus,
    TransportKind,
};

/// Event emitted to the native UI shell whenever the snapshot changes or an alert triggers.
#[derive(Debug, Clone)]
pub struct TrayUpdate {
    pub snapshot: TraySnapshot,
    pub alerts: Vec<LowBatteryAlert>,
}

struct ServiceState {
    config: TrayConfig,
    devices: Vec<Device>,
    readings: HashMap<String, BatteryReading>,
    errors: HashMap<String, ErrorKind>,
    last_sync_at: Option<chrono::DateTime<Utc>>,
    refreshing: bool,
    tracker: LowBatteryTracker,
}

/// Thread-safe core coordinator shared by every native platform tray shell.
pub struct TrayService {
    client: Arc<BatteryClient>,
    config_path: Option<PathBuf>,
    state: Mutex<ServiceState>,
    refresh_guard: Mutex<()>,
}

impl TrayService {
    /// Create a service backed by the real OS `BatteryClient` and default config path.
    pub fn new() -> Result<Self, BatteryError> {
        let config_path = default_config_path();
        let config = config_path
            .as_deref()
            .and_then(|path| load_config(path).ok())
            .unwrap_or_default();
        let client_config = ClientConfig {
            default_timeout: config.read_timeout(),
            // Keep per-read experimental gating controlled dynamically by TrayConfig.
            allow_experimental: false,
            enable_cache: true,
        };
        let client = BatteryClient::new(client_config)?;
        Ok(Self::with_client(Arc::new(client), config, config_path))
    }

    /// Construct a service with an injected `BatteryClient` (for deterministic tests).
    #[must_use]
    pub fn with_client(
        client: Arc<BatteryClient>,
        config: TrayConfig,
        config_path: Option<PathBuf>,
    ) -> Self {
        Self {
            client,
            config_path,
            state: Mutex::new(ServiceState {
                config: config.normalized(),
                devices: Vec::new(),
                readings: HashMap::new(),
                errors: HashMap::new(),
                last_sync_at: None,
                refreshing: false,
                tracker: LowBatteryTracker::default(),
            }),
            refresh_guard: Mutex::new(()),
        }
    }

    /// Return the current immutable UI snapshot without performing device I/O.
    #[must_use]
    pub fn snapshot(&self) -> TraySnapshot {
        let guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        build_snapshot(&guard)
    }

    /// Return a copy of the active configuration.
    #[must_use]
    pub fn config(&self) -> TrayConfig {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .config
            .clone()
    }

    /// Update configuration, persist to disk (best effort), and return the updated snapshot.
    pub fn update_config(&self, config: TrayConfig) -> TraySnapshot {
        let normalized = config.normalized();
        if let Some(path) = &self.config_path {
            let _ = save_config(path, &normalized);
        }
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.config = normalized;
        build_snapshot(&guard)
    }

    /// Perform passive enumeration followed by conservative concurrent reads of eligible devices.
    pub fn refresh_all(&self) -> TrayUpdate {
        self.refresh_all_with_progress(|_| {})
    }

    /// Same as `refresh_all`, invoking `on_progress` immediately after passive discovery
    /// and again as each device finishes reading, so fast awake devices render right away
    /// without waiting behind a sleeping peripheral's timeout.
    pub fn refresh_all_with_progress<F>(&self, on_progress: F) -> TrayUpdate
    where
        F: Fn(TraySnapshot),
    {
        let _refresh_lock = self.refresh_guard.lock().unwrap_or_else(|e| e.into_inner());

        let config = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            guard.refreshing = true;
            guard.config.clone()
        };

        let listed = self.client.list_devices().unwrap_or_default();
        let eligible_ids: Vec<String> = listed
            .iter()
            .filter(|device| is_device_eligible(device, config.allow_experimental))
            .map(|device| device.id.0.clone())
            .collect();

        // Publish discovered devices immediately (keeping any cached readings for still-present IDs).
        let discovered_snapshot = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let present: std::collections::HashSet<&str> =
                listed.iter().map(|d| d.id.0.as_str()).collect();
            guard.readings.retain(|id, _| present.contains(id.as_str()));
            guard.errors.retain(|id, _| present.contains(id.as_str()));
            guard.devices = listed;
            build_snapshot(&guard)
        };
        on_progress(discovered_snapshot);

        let read_options = ReadOptions {
            timeout: config.read_timeout(),
            allow_stale: config.allow_stale,
            allow_experimental: config.allow_experimental,
        };

        // Read independent endpoints concurrently and publish each device as soon as it responds.
        let (tx, rx) = std::sync::mpsc::channel::<(String, Result<BatteryReading, BatteryError>)>();
        thread::scope(|scope| {
            for id in &eligible_ids {
                let client = Arc::clone(&self.client);
                let opts = read_options.clone();
                let dev_id = id.clone();
                let sender = tx.clone();
                scope.spawn(move || {
                    let res = client.read_battery_with_options(&dev_id, opts);
                    let _ = sender.send((dev_id, res));
                });
            }
            drop(tx);

            for (id, result) in rx {
                let partial_snapshot = {
                    let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    match result {
                        Ok(reading) => {
                            guard.errors.remove(&id);
                            guard.readings.insert(id, reading);
                        }
                        Err(error) => {
                            guard.errors.insert(id.clone(), error.kind());
                            if !guard.config.allow_stale {
                                guard.readings.remove(&id);
                            }
                        }
                    }
                    build_snapshot(&guard)
                };
                on_progress(partial_snapshot);
            }
        });

        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.last_sync_at = Some(Utc::now());
        guard.refreshing = false;

        let snapshot = build_snapshot(&guard);
        let notify = guard.config.low_battery_notify;
        let threshold = guard.config.low_battery_threshold;
        let alerts = guard.tracker.evaluate(&snapshot.devices, notify, threshold);
        TrayUpdate { snapshot, alerts }
    }

    /// Refresh a single device on demand (e.g., from the expanded device drawer).
    pub fn refresh_device(&self, device_id: &str) -> TrayUpdate {
        self.refresh_device_with_progress(device_id, |_| {})
    }

    /// Refresh a single device on demand, emitting an immediate `refreshing = true` snapshot first.
    pub fn refresh_device_with_progress<F>(&self, device_id: &str, on_start: F) -> TrayUpdate
    where
        F: FnOnce(TraySnapshot),
    {
        let _refresh_lock = self.refresh_guard.lock().unwrap_or_else(|e| e.into_inner());

        let (config, starting_snapshot) = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            guard.refreshing = true;
            (guard.config.clone(), build_snapshot(&guard))
        };
        on_start(starting_snapshot);

        let read_options = ReadOptions {
            timeout: config.read_timeout(),
            allow_stale: config.allow_stale,
            allow_experimental: config.allow_experimental,
        };
        let result = self
            .client
            .read_battery_with_options(device_id, read_options);

        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match result {
            Ok(reading) => {
                guard.errors.remove(device_id);
                guard.readings.insert(device_id.to_owned(), reading);
            }
            Err(error) => {
                guard.errors.insert(device_id.to_owned(), error.kind());
                if !guard.config.allow_stale {
                    guard.readings.remove(device_id);
                }
            }
        }
        guard.last_sync_at = Some(Utc::now());
        guard.refreshing = false;

        let snapshot = build_snapshot(&guard);
        let notify = guard.config.low_battery_notify;
        let threshold = guard.config.low_battery_threshold;
        let alerts = guard.tracker.evaluate(&snapshot.devices, notify, threshold);
        TrayUpdate { snapshot, alerts }
    }

    /// Produce sanitized passive diagnostic JSON for clipboard copy.
    pub fn diagnose_json(&self, device_id: &str) -> Result<String, BatteryError> {
        let diag = self.client.diagnose(device_id)?;
        serde_json::to_string_pretty(&diag).map_err(|error| BatteryError::Internal {
            message: format!("failed to serialize diagnostic: {error}"),
        })
    }
}

#[derive(Default)]
struct PollerSignal {
    shutdown: bool,
    refresh_requested: bool,
    single_device: Option<String>,
    config_changed: bool,
}

/// Handle controlling the single lightweight background polling thread (no async runtime).
pub struct PollerHandle {
    service: Arc<TrayService>,
    signal: Arc<(Mutex<PollerSignal>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl PollerHandle {
    /// Start background polling and notify `on_update` on every snapshot change.
    pub fn start<F>(service: Arc<TrayService>, on_update: F) -> Self
    where
        F: Fn(TrayUpdate) + Send + 'static,
    {
        let signal = Arc::new((Mutex::new(PollerSignal::default()), Condvar::new()));
        let thread_service = Arc::clone(&service);
        let thread_signal = Arc::clone(&signal);

        let handle = thread::Builder::new()
            .name("wb-tray-poller".to_owned())
            .spawn(move || {
                // Perform an initial progressive sync on startup.
                let initial = thread_service.refresh_all_with_progress(|snap| {
                    on_update(TrayUpdate {
                        snapshot: snap,
                        alerts: Vec::new(),
                    });
                });
                on_update(initial);

                loop {
                    let poll_interval = thread_service.config().poll_interval();
                    let action = {
                        let (lock, cvar) = &*thread_signal;
                        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());

                        loop {
                            if sig.shutdown {
                                break PollerAction::Shutdown;
                            }
                            if let Some(id) = sig.single_device.take() {
                                break PollerAction::RefreshDevice(id);
                            }
                            if sig.refresh_requested {
                                sig.refresh_requested = false;
                                break PollerAction::RefreshAll;
                            }
                            if sig.config_changed {
                                sig.config_changed = false;
                                break PollerAction::ConfigChanged;
                            }

                            match poll_interval {
                                Some(duration) => {
                                    let (next, timeout) = cvar
                                        .wait_timeout(sig, duration)
                                        .unwrap_or_else(|e| e.into_inner());
                                    sig = next;
                                    if timeout.timed_out() {
                                        break PollerAction::RefreshAll;
                                    }
                                }
                                None => {
                                    sig = cvar.wait(sig).unwrap_or_else(|e| e.into_inner());
                                }
                            }
                        }
                    };

                    match action {
                        PollerAction::Shutdown => break,
                        PollerAction::RefreshAll | PollerAction::ConfigChanged => {
                            let update = thread_service.refresh_all_with_progress(|snap| {
                                on_update(TrayUpdate {
                                    snapshot: snap,
                                    alerts: Vec::new(),
                                });
                            });
                            on_update(update);
                        }
                        PollerAction::RefreshDevice(id) => {
                            let update = thread_service.refresh_device_with_progress(&id, |snap| {
                                on_update(TrayUpdate {
                                    snapshot: snap,
                                    alerts: Vec::new(),
                                });
                            });
                            on_update(update);
                        }
                    }
                }
            })
            .expect("failed to spawn tray poller thread");

        Self {
            service,
            signal,
            thread: Some(handle),
        }
    }

    /// Access the underlying `TrayService`.
    #[must_use]
    pub fn service(&self) -> &Arc<TrayService> {
        &self.service
    }

    /// Request an immediate full refresh on the background thread.
    pub fn request_refresh(&self) {
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.refresh_requested = true;
        cvar.notify_one();
    }

    /// Request an immediate single-device refresh on the background thread.
    pub fn request_device_refresh(&self, device_id: impl Into<String>) {
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.single_device = Some(device_id.into());
        cvar.notify_one();
    }

    /// Update configuration and wake the poller so new intervals/policies take effect immediately.
    pub fn update_config(&self, config: TrayConfig) -> TraySnapshot {
        let snapshot = self.service.update_config(config);
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.config_changed = true;
        cvar.notify_one();
        snapshot
    }

    /// Gracefully stop the background poller thread.
    pub fn shutdown(&mut self) {
        {
            let (lock, cvar) = &*self.signal;
            let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
            sig.shutdown = true;
            cvar.notify_one();
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

enum PollerAction {
    Shutdown,
    RefreshAll,
    RefreshDevice(String),
    ConfigChanged,
}

fn is_device_eligible(device: &Device, allow_experimental: bool) -> bool {
    if device.match_status != MatchStatus::Exact {
        return false;
    }
    match device.support_status {
        SupportStatus::Verified | SupportStatus::Deprecated => true,
        SupportStatus::Experimental => allow_experimental,
        SupportStatus::Draft | SupportStatus::Unknown => false,
    }
}

fn transport_label(device: &Device) -> String {
    match device.endpoint.transport {
        TransportKind::BluetoothLe => "BLE BAS".to_owned(),
        TransportKind::XInput => "XInput".to_owned(),
        TransportKind::Hid => match device.endpoint.bus_type {
            Some(HidBusType::Bluetooth) => "BT HID".to_owned(),
            Some(HidBusType::Usb) | None => "2.4G HID".to_owned(),
            Some(HidBusType::I2c) => "I2C HID".to_owned(),
            Some(HidBusType::Spi) => "SPI HID".to_owned(),
        },
        TransportKind::Mock => "Mock".to_owned(),
    }
}

fn build_snapshot(state: &ServiceState) -> TraySnapshot {
    let mut hidden_experimental_count = 0usize;
    let mut views = Vec::new();

    for device in &state.devices {
        if device.match_status != MatchStatus::Exact {
            continue;
        }
        if device.support_status == SupportStatus::Experimental && !state.config.allow_experimental
        {
            hidden_experimental_count += 1;
            continue;
        }
        if !is_device_eligible(device, state.config.allow_experimental) {
            continue;
        }

        let reading = state.readings.get(&device.id.0);
        let reading = match reading {
            Some(r) if r.freshness == Freshness::Stale && !state.config.allow_stale => None,
            other => other,
        };
        let last_error = state.errors.get(&device.id.0).copied();

        let (
            precision,
            percentage,
            level,
            voltage_mv,
            charge_state,
            reachability,
            freshness,
            observed_at,
        ) = match reading {
            Some(r) => (
                r.precision,
                r.percentage,
                r.level,
                r.voltage_mv,
                r.charge_state,
                r.reachability,
                Some(r.freshness),
                Some(r.observed_at),
            ),
            None => (
                Precision::None,
                None,
                None,
                None,
                ChargeState::Unknown,
                // Conservative semantics: never infer Offline from a timeout or read error.
                Reachability::Unknown,
                None,
                None,
            ),
        };

        let is_low = precision == Precision::Percentage
            && percentage.is_some_and(|pct| pct <= state.config.low_battery_threshold)
            && !matches!(charge_state, ChargeState::Charging | ChargeState::Full);

        views.push(DeviceView {
            id: device.id.0.clone(),
            name: device.name.clone(),
            kind: classify_device(device),
            transport: device.endpoint.transport,
            transport_label: transport_label(device),
            provider_id: device.provider_id.clone(),
            support_status: device.support_status,
            precision,
            percentage,
            level,
            voltage_mv,
            charge_state,
            reachability,
            freshness,
            observed_at,
            last_error,
            is_low,
        });
    }

    views.sort_by(|a, b| {
        device_sort_rank(a).cmp(&device_sort_rank(b)).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });

    let icon_state = if views.is_empty() {
        TrayIconState::Idle
    } else if state.config.low_battery_notify && views.iter().any(|d| d.is_low) {
        TrayIconState::LowBattery
    } else {
        TrayIconState::Normal
    };

    let tooltip = format_tooltip(&views);

    TraySnapshot {
        devices: views,
        hidden_experimental_count,
        icon_state,
        tooltip,
        last_sync_at: state.last_sync_at,
        refreshing: state.refreshing,
        config: state.config.clone(),
    }
}

fn device_sort_rank(device: &DeviceView) -> u8 {
    if device.is_low && device.freshness == Some(Freshness::Live) {
        0
    } else if device.freshness == Some(Freshness::Live)
        && device.reachability == Reachability::Online
    {
        1
    } else if device.freshness == Some(Freshness::Stale)
        || device.reachability == Reachability::Sleeping
    {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use wireless_battery::registry::Registry;
    use wireless_battery::{
        Capabilities, Deadline, DeviceContext, Endpoint, EndpointId, HidSession, MatchDecision,
        Provider, ProviderMetadata, ProviderReading, ReadingSource, Transport, TransportError,
    };

    struct StaticProvider {
        id: &'static str,
        vid: u16,
        pid: u16,
        status: SupportStatus,
        percentage: u8,
        fail: Arc<AtomicBool>,
    }

    impl Provider for StaticProvider {
        fn id(&self) -> &str {
            self.id
        }

        fn metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                version: "0.1.0".to_owned(),
                support_status: self.status,
                capabilities: Capabilities {
                    percentage: true,
                    charge_state: true,
                    ..Capabilities::default()
                },
                io_timeout: Duration::from_millis(500),
                cooldown: Duration::ZERO,
            }
        }

        fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
            if endpoint.vid == Some(self.vid) && endpoint.pid == Some(self.pid) {
                MatchDecision::Exact
            } else {
                MatchDecision::NoMatch
            }
        }

        fn execute(
            &self,
            _context: &DeviceContext<'_>,
            _transport: &dyn Transport,
            _deadline: Deadline,
        ) -> Result<ProviderReading, BatteryError> {
            if self.fail.load(Ordering::SeqCst) {
                return Err(BatteryError::Timeout {
                    operation: "mock device asleep".to_owned(),
                });
            }
            Ok(ProviderReading {
                percentage: Some(self.percentage),
                level: None,
                voltage_mv: None,
                charge_state: ChargeState::Discharging,
                reachability: Reachability::Online,
                source: ReadingSource::Mock,
            })
        }
    }

    struct MockTransport {
        endpoints: Vec<Endpoint>,
    }

    impl Transport for MockTransport {
        fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
            Ok(self.endpoints.clone())
        }

        fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
            Err(TransportError::Timeout)
        }
    }

    #[test]
    fn respects_experimental_gating_stale_cache_and_low_battery_badge() {
        let fail_verified = Arc::new(AtomicBool::new(false));
        let fail_exp = Arc::new(AtomicBool::new(false));

        let mut ep_mouse = Endpoint::synthetic("ep-mouse", 0x046D, 0xC547);
        ep_mouse.product_name = Some("Logitech GPW3".to_owned());
        let mut ep_headset = Endpoint::synthetic("ep-headset", 0x0B05, 0x1ACE);
        ep_headset.product_name = Some("ROG Delta II".to_owned());

        let registry = Registry::new(vec![
            Arc::new(StaticProvider {
                id: "logitech.hidpp",
                vid: 0x046D,
                pid: 0xC547,
                status: SupportStatus::Verified,
                percentage: 12,
                fail: Arc::clone(&fail_verified),
            }),
            Arc::new(StaticProvider {
                id: "asus.headset",
                vid: 0x0B05,
                pid: 0x1ACE,
                status: SupportStatus::Experimental,
                percentage: 80,
                fail: Arc::clone(&fail_exp),
            }),
        ])
        .unwrap();

        let client = BatteryClient::with_transport(
            ClientConfig {
                default_timeout: Duration::from_secs(1),
                allow_experimental: false,
                enable_cache: true,
            },
            Arc::new(MockTransport {
                endpoints: vec![ep_mouse, ep_headset],
            }),
            registry,
        )
        .unwrap();

        let service = TrayService::with_client(Arc::new(client), TrayConfig::default(), None);

        // Default config hides experimental device, reads verified GPW3 (12% -> low alert).
        let update = service.refresh_all();
        assert_eq!(update.snapshot.devices.len(), 1);
        assert_eq!(update.snapshot.hidden_experimental_count, 1);
        assert_eq!(update.snapshot.icon_state, TrayIconState::LowBattery);
        assert_eq!(update.alerts.len(), 1);
        assert_eq!(update.alerts[0].percentage, 12);

        // When verified device goes to sleep (timeout), stale cache keeps 12% marked Stale.
        fail_verified.store(true, Ordering::SeqCst);
        let stale_update = service.refresh_all();
        assert_eq!(stale_update.snapshot.devices[0].percentage, Some(12));
        assert_eq!(
            stale_update.snapshot.devices[0].freshness,
            Some(Freshness::Stale)
        );
        assert!(stale_update.alerts.is_empty());

        // Enabling experimental reveals the second device without rebuilding BatteryClient.
        let mut next_cfg = service.config();
        next_cfg.allow_experimental = true;
        service.update_config(next_cfg);
        let exp_update = service.refresh_all();
        assert_eq!(exp_update.snapshot.devices.len(), 2);
        assert_eq!(exp_update.snapshot.hidden_experimental_count, 0);

        // Diagnostics JSON is sanitized and valid JSON.
        let diag = service
            .diagnose_json(&exp_update.snapshot.devices[0].id)
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&diag).unwrap();
        assert!(parsed.get("provider_id").is_some());
    }
}
