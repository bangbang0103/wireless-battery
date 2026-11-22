//! Background polling coordinator and immutable snapshot builder.

use crate::config::{default_config_path, load_config, save_config, LanguageMode, TrayConfig};
use crate::device_kind::classify_device;
use crate::notify::LowBatteryTracker;
use crate::state::{
    format_tooltip_with_locale, DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot,
};
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

pub(crate) struct ServiceState {
    pub(crate) config: TrayConfig,
    pub(crate) system_is_zh: bool,
    devices: Vec<Device>,
    readings: HashMap<String, BatteryReading>,
    errors: HashMap<String, ErrorKind>,
    pub(crate) last_sync_at: Option<chrono::DateTime<Utc>>,
    refreshing: bool,
    tracker: LowBatteryTracker,
    #[cfg(feature = "dev-tools")]
    pub(crate) dev: crate::dev_tools::DevState,
}

/// Thread-safe core coordinator shared by every native platform tray shell.
pub struct TrayService {
    client: Arc<BatteryClient>,
    config_path: Option<PathBuf>,
    pub(crate) state: Mutex<ServiceState>,
    refresh_guard: Mutex<()>,
    save_guard: Mutex<()>,
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
                system_is_zh: detect_env_is_zh(),
                devices: Vec::new(),
                readings: HashMap::new(),
                errors: HashMap::new(),
                last_sync_at: None,
                refreshing: false,
                tracker: LowBatteryTracker::default(),
                #[cfg(feature = "dev-tools")]
                dev: crate::dev_tools::DevState::default(),
            }),
            refresh_guard: Mutex::new(()),
            save_guard: Mutex::new(()),
        }
    }

    /// Update the detected OS language flag used when `config.language == LanguageMode::System`.
    pub fn set_system_is_zh(&self, is_zh: bool) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.system_is_zh = is_zh;
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
        let snapshot = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            guard.config = normalized;
            build_snapshot(&guard)
        };
        self.persist_current_config();
        snapshot
    }

    /// Update configuration in memory immediately and persist to disk on a background thread
    /// so UI interactions never hitch on synchronous filesystem writes.
    pub fn update_config_async(self: &Arc<Self>, config: TrayConfig) -> TraySnapshot {
        let normalized = config.normalized();
        let snapshot = {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            guard.config = normalized;
            build_snapshot(&guard)
        };
        if self.config_path.is_some() {
            let service = Arc::clone(self);
            let _ = thread::Builder::new()
                .name("wb-tray-config-save".to_owned())
                .spawn(move || {
                    service.persist_current_config();
                });
        }
        snapshot
    }

    fn persist_current_config(&self) {
        let Some(path) = &self.config_path else {
            return;
        };
        let _save_lock = self.save_guard.lock().unwrap_or_else(|e| e.into_inner());
        let latest = self.config();
        let _ = save_config(path, &latest);
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

        let listed = match self.client.list_devices() {
            Ok(devices) => devices,
            Err(_) => {
                // Keep previously discovered devices and cached readings intact on transient
                // OS/bus enumeration errors instead of wiping the UI to empty.
                let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
                guard.refreshing = false;
                let snapshot = build_snapshot(&guard);
                on_progress(snapshot.clone());
                return TrayUpdate {
                    snapshot,
                    alerts: Vec::new(),
                };
            }
        };
        let eligible_ids: Vec<String> = listed
            .iter()
            .filter(|device| is_device_eligible(device))
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
        let notify = guard.config.low_battery_notify && !guard.config.low_battery_silent;
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
        let notify = guard.config.low_battery_notify && !guard.config.low_battery_silent;
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

    /// Produce sanitized error report JSON (combining the latest read error kind and
    /// passive diagnostic info) for clipboard copy when a device fails to read.
    pub fn error_report_json(&self, device_id: &str) -> Result<String, BatteryError> {
        let last_error = {
            let guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            guard.errors.get(device_id).copied()
        };
        let diag = self.client.diagnose(device_id)?;
        let mut value = serde_json::to_value(diag).map_err(|error| BatteryError::Internal {
            message: format!("failed to serialize diagnostic: {error}"),
        })?;
        if let Some(obj) = value.as_object_mut() {
            if let Some(err_kind) = last_error {
                if let Ok(err_val) = serde_json::to_value(err_kind) {
                    obj.insert("last_error".to_owned(), err_val);
                }
            }
        }
        serde_json::to_string_pretty(&value).map_err(|error| BatteryError::Internal {
            message: format!("failed to serialize error report: {error}"),
        })
    }
}

#[derive(Default)]
struct PollerSignal {
    shutdown: bool,
    refresh_requested: bool,
    single_device: Option<String>,
    wake_interval: bool,
    emit_snapshot: bool,
    pending_alerts: Vec<LowBatteryAlert>,
}

/// Handle controlling the single lightweight background polling thread (no async runtime).
pub struct PollerHandle {
    service: Arc<TrayService>,
    signal: Arc<(Mutex<PollerSignal>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl PollerHandle {
    /// Start background polling and notify `on_update` on every snapshot change.
    pub fn start<F>(service: Arc<TrayService>, on_update: F) -> std::io::Result<Self>
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
                            if sig.emit_snapshot || !sig.pending_alerts.is_empty() {
                                sig.emit_snapshot = false;
                                let alerts = std::mem::take(&mut sig.pending_alerts);
                                break PollerAction::EmitSnapshot(alerts);
                            }
                            if let Some(id) = sig.single_device.take() {
                                break PollerAction::RefreshDevice(id);
                            }
                            if sig.refresh_requested {
                                sig.refresh_requested = false;
                                break PollerAction::RefreshAll;
                            }
                            if sig.wake_interval {
                                sig.wake_interval = false;
                                break PollerAction::RecalcInterval;
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
                        PollerAction::RecalcInterval => {}
                        PollerAction::EmitSnapshot(alerts) => {
                            on_update(TrayUpdate {
                                snapshot: thread_service.snapshot(),
                                alerts,
                            });
                        }
                        PollerAction::RefreshAll => {
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
            })?;

        Ok(Self {
            service,
            signal,
            thread: Some(handle),
        })
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

    /// Ask the background poller thread to emit the current snapshot via `on_update`.
    pub fn request_snapshot_emit(&self) {
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.emit_snapshot = true;
        cvar.notify_one();
    }

    /// Queue a low-battery alert and emit the current snapshot from the background poller thread.
    #[cfg(feature = "dev-tools")]
    pub fn request_alert_emit(&self, alert: LowBatteryAlert) {
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.pending_alerts.push(alert);
        sig.emit_snapshot = true;
        cvar.notify_one();
    }

    fn notify_config_diff(&self, prev: &TrayConfig, next: &TrayConfig) {
        if prev.poll_interval_secs == next.poll_interval_secs {
            return;
        }
        let (lock, cvar) = &*self.signal;
        let mut sig = lock.lock().unwrap_or_else(|e| e.into_inner());
        sig.wake_interval = true;
        cvar.notify_one();
    }

    /// Update configuration and wake the poller when the interval changed.
    pub fn update_config(&self, config: TrayConfig) -> TraySnapshot {
        let prev = self.service.config();
        let next = config.normalized();
        let snapshot = self.service.update_config(next.clone());
        self.notify_config_diff(&prev, &next);
        snapshot
    }

    /// Non-blocking variant of `update_config` that persists on a background thread while
    /// waking the poller only when the interval changed.
    pub fn update_config_async(&self, config: TrayConfig) -> TraySnapshot {
        let prev = self.service.config();
        let next = config.normalized();
        let snapshot = self.service.update_config_async(next.clone());
        self.notify_config_diff(&prev, &next);
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
    EmitSnapshot(Vec<LowBatteryAlert>),
    RecalcInterval,
}

fn is_device_eligible(device: &Device) -> bool {
    device.match_status == MatchStatus::Exact && device.support_status != SupportStatus::Unknown
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

pub(crate) fn build_snapshot(state: &ServiceState) -> TraySnapshot {
    let mut views = Vec::new();

    for device in &state.devices {
        if device.match_status != MatchStatus::Exact {
            continue;
        }
        if !is_device_eligible(device) {
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

    let is_zh = resolve_zh(state.config.language, state.system_is_zh);
    let tooltip = format_tooltip_with_locale(&views, is_zh);

    #[cfg_attr(not(feature = "dev-tools"), allow(unused_mut))]
    let mut snapshot = TraySnapshot {
        app_version: crate::version::APP_DISPLAY_VERSION.to_owned(),
        devices: views,
        icon_state,
        tooltip,
        last_sync_at: state.last_sync_at,
        refreshing: state.refreshing,
        config: state.config.clone(),
    };
    #[cfg(feature = "dev-tools")]
    state.dev.apply_to_snapshot(
        &mut snapshot,
        state.config.low_battery_threshold,
        state.config.low_battery_notify,
        is_zh,
    );
    snapshot
}

fn resolve_zh(mode: LanguageMode, system_is_zh: bool) -> bool {
    match mode {
        LanguageMode::ZhCn => true,
        LanguageMode::En => false,
        LanguageMode::System => system_is_zh,
    }
}

fn detect_env_is_zh() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"].iter().any(|key| {
        std::env::var(key)
            .map(|val| val.to_ascii_lowercase().starts_with("zh"))
            .unwrap_or(false)
    })
}

pub(crate) fn device_sort_rank(device: &DeviceView) -> u8 {
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
                device_kind: crate::DeviceKind::Mouse,
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
    fn reads_experimental_stale_cache_and_low_battery_badge() {
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
                enable_cache: true,
            },
            Arc::new(MockTransport {
                endpoints: vec![ep_mouse, ep_headset],
            }),
            registry,
        )
        .unwrap();

        let service = TrayService::with_client(Arc::new(client), TrayConfig::default(), None);

        // Both verified and experimental devices are read; GPW3 at 12% triggers a low alert.
        let update = service.refresh_all();
        assert_eq!(update.snapshot.devices.len(), 2);
        assert_eq!(update.snapshot.icon_state, TrayIconState::LowBattery);
        assert_eq!(update.alerts.len(), 1);
        assert_eq!(update.alerts[0].percentage, 12);

        // When verified device goes to sleep (timeout), stale cache keeps 12% marked Stale.
        fail_verified.store(true, Ordering::SeqCst);
        let stale_update = service.refresh_all();
        let stale_mouse = stale_update
            .snapshot
            .devices
            .iter()
            .find(|device| device.provider_id.as_deref() == Some("logitech.hidpp"))
            .unwrap();
        assert_eq!(stale_mouse.percentage, Some(12));
        assert_eq!(stale_mouse.freshness, Some(Freshness::Stale));
        assert!(stale_update.alerts.is_empty());

        let exp_update = service.refresh_all();
        assert_eq!(exp_update.snapshot.devices.len(), 2);

        // Diagnostics & error report JSON are sanitized and valid JSON.
        let diag = service
            .diagnose_json(&exp_update.snapshot.devices[0].id)
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&diag).unwrap();
        assert!(parsed.get("provider_id").is_some());

        let err_report = service
            .error_report_json(&exp_update.snapshot.devices[0].id)
            .unwrap();
        let err_parsed: serde_json::Value = serde_json::from_str(&err_report).unwrap();
        assert!(err_parsed.get("last_error").is_some());
    }

    #[test]
    fn silent_low_battery_suppresses_os_alerts_while_keeping_tray_badge() {
        let fail = Arc::new(AtomicBool::new(false));
        let mut ep_mouse = Endpoint::synthetic("ep-mouse-silent", 0x046D, 0xC547);
        ep_mouse.product_name = Some("Logitech GPW3".to_owned());

        let registry = Registry::new(vec![Arc::new(StaticProvider {
            id: "logitech.hidpp",
            vid: 0x046D,
            pid: 0xC547,
            status: SupportStatus::Verified,
            percentage: 10,
            fail,
        })])
        .unwrap();

        let client = BatteryClient::with_transport(
            ClientConfig {
                default_timeout: Duration::from_secs(1),
                enable_cache: true,
            },
            Arc::new(MockTransport {
                endpoints: vec![ep_mouse],
            }),
            registry,
        )
        .unwrap();

        let cfg = TrayConfig {
            low_battery_notify: true,
            low_battery_silent: true,
            ..TrayConfig::default()
        };
        let service = TrayService::with_client(Arc::new(client), cfg, None);

        let update = service.refresh_all();
        assert_eq!(update.snapshot.icon_state, TrayIconState::LowBattery);
        assert!(update.alerts.is_empty());
    }

    #[test]
    fn transient_list_error_preserves_cached_devices_and_interval_change_avoids_poll() {
        use std::sync::atomic::AtomicUsize;

        struct FlakyTransport {
            endpoints: Vec<Endpoint>,
            fail_enumerate: Arc<AtomicBool>,
            enumerate_calls: Arc<AtomicUsize>,
        }

        impl Transport for FlakyTransport {
            fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
                self.enumerate_calls.fetch_add(1, Ordering::SeqCst);
                if self.fail_enumerate.load(Ordering::SeqCst) {
                    return Err(TransportError::DeviceBusy);
                }
                Ok(self.endpoints.clone())
            }

            fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
                Err(TransportError::Timeout)
            }
        }

        let fail_read = Arc::new(AtomicBool::new(false));
        let fail_enumerate = Arc::new(AtomicBool::new(false));
        let enumerate_calls = Arc::new(AtomicUsize::new(0));
        let mut ep_mouse = Endpoint::synthetic("ep-mouse-flaky", 0x046D, 0xC547);
        ep_mouse.product_name = Some("Logitech GPW3".to_owned());

        let registry = Registry::new(vec![Arc::new(StaticProvider {
            id: "logitech.hidpp",
            vid: 0x046D,
            pid: 0xC547,
            status: SupportStatus::Verified,
            percentage: 55,
            fail: fail_read,
        })])
        .unwrap();

        let client = BatteryClient::with_transport(
            ClientConfig {
                default_timeout: Duration::from_secs(1),
                enable_cache: true,
            },
            Arc::new(FlakyTransport {
                endpoints: vec![ep_mouse],
                fail_enumerate: Arc::clone(&fail_enumerate),
                enumerate_calls: Arc::clone(&enumerate_calls),
            }),
            registry,
        )
        .unwrap();

        let service = Arc::new(TrayService::with_client(
            Arc::new(client),
            TrayConfig::default(),
            None,
        ));

        let ok_update = service.refresh_all();
        assert_eq!(ok_update.snapshot.devices.len(), 1);
        assert_eq!(ok_update.snapshot.devices[0].percentage, Some(55));

        fail_enumerate.store(true, Ordering::SeqCst);
        let preserved = service.refresh_all();
        assert_eq!(preserved.snapshot.devices.len(), 1);
        assert_eq!(preserved.snapshot.devices[0].percentage, Some(55));

        // Verify PollerHandle interval-only change does not fire a full hardware refresh.
        let calls_before = enumerate_calls.load(Ordering::SeqCst);
        let (tx, rx) = std::sync::mpsc::channel();
        let poller = PollerHandle::start(Arc::clone(&service), move |upd| {
            let _ = tx.send(upd.snapshot);
        })
        .unwrap();
        loop {
            let snap = rx.recv_timeout(Duration::from_secs(2)).unwrap();
            if !snap.refreshing {
                break;
            }
        }
        let after_initial = enumerate_calls.load(Ordering::SeqCst);
        assert!(after_initial > calls_before);

        let mut next = service.config();
        next.poll_interval_secs = 900;
        let _ = poller.update_config(next);
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(enumerate_calls.load(Ordering::SeqCst), after_initial);
    }
}
