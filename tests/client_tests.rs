mod common;

use common::fixture;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wireless_battery::model::{ClientConfig, Endpoint, EndpointId, Freshness, ReadOptions};
use wireless_battery::registry::Registry;
use wireless_battery::transport::record::RecordTransport;
use wireless_battery::transport::replay::ReplayTransport;
use wireless_battery::transport::{HidSession, Transport, TransportError};
use wireless_battery::{BatteryClient, ErrorKind};

struct SequenceTransport {
    normal: ReplayTransport,
    failure: ReplayTransport,
    opens: AtomicUsize,
}

impl Transport for SequenceTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        self.normal.enumerate()
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        if self.opens.fetch_add(1, Ordering::SeqCst) == 0 {
            self.normal.open(endpoint)
        } else {
            self.failure.open(endpoint)
        }
    }
}

#[test]
fn stale_preserves_original_observation_and_is_opt_in() {
    let sequence = Arc::new(SequenceTransport {
        normal: ReplayTransport::new(&fixture("feature_normal.json")),
        failure: ReplayTransport::new(&fixture("feature_timeout.json")),
        opens: AtomicUsize::new(0),
    });
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            ..ClientConfig::default()
        },
        sequence,
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let live = client.read_battery(&device.id.0).unwrap();
    let error = client.read_battery(&device.id.0).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Timeout);
    let stale = client
        .read_battery_with_options(
            &device.id.0,
            ReadOptions {
                allow_stale: true,
                allow_experimental: true,
                ..ReadOptions::default()
            },
        )
        .unwrap();
    assert_eq!(stale.freshness, Freshness::Stale);
    assert_eq!(stale.observed_at, live.observed_at);
    assert_eq!(stale.percentage, Some(72));
    assert!(client
        .diagnose(&device.id.0)
        .unwrap()
        .cached_observed_at
        .is_some());
}

struct DiscoveryTransport {
    endpoints: Vec<Endpoint>,
    opens: AtomicUsize,
}

impl Transport for DiscoveryTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        Ok(self.endpoints.clone())
    }

    fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        Err(TransportError::Message(
            "discovery must not open".to_owned(),
        ))
    }
}

#[test]
fn discovery_lists_unknown_without_any_session_io() {
    let transport = Arc::new(DiscoveryTransport {
        endpoints: vec![
            fixture("feature_normal.json").endpoint,
            Endpoint::synthetic("ep_unknown", 1, 2),
        ],
        opens: AtomicUsize::new(0),
    });
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            ..ClientConfig::default()
        },
        transport.clone(),
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let devices = client.list_devices().unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(transport.opens.load(Ordering::SeqCst), 0);
    assert!(devices
        .iter()
        .any(|device| device.support_status == wireless_battery::SupportStatus::Unknown));
    let unknown = devices
        .iter()
        .find(|device| device.provider_id.is_none())
        .unwrap();
    assert_eq!(
        client.read_battery(&unknown.id.0).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
}

#[test]
fn record_transport_reproduces_exact_provider_session() {
    let replay = Arc::new(ReplayTransport::new(&fixture("feature_normal.json")));
    let recorder = Arc::new(RecordTransport::new(replay));
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            ..ClientConfig::default()
        },
        recorder.clone(),
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    client.read_battery(&device.id.0).unwrap();
    let steps = recorder.steps().unwrap();
    assert_eq!(steps, fixture("feature_normal.json").steps);
}

#[test]
fn matching_conflict_is_diagnostic_and_never_probed() {
    let yaml = std::fs::read_to_string(
        common::root().join("fixtures/synthetic/packages/feature_mouse.yaml"),
    )
    .unwrap();
    let first = wireless_battery::providers::declarative::DeclarativeProvider::from_yaml(
        &yaml,
        "first.yaml",
    )
    .unwrap();
    let mut second_package = first.package().clone();
    second_package.id = "synthetic.feature.conflict".to_owned();
    let second = wireless_battery::providers::declarative::DeclarativeProvider::new(second_package);
    let registry = Registry::new(vec![Arc::new(first), Arc::new(second)]).unwrap();
    let transport = Arc::new(DiscoveryTransport {
        endpoints: vec![fixture("feature_normal.json").endpoint],
        opens: AtomicUsize::new(0),
    });
    let client =
        BatteryClient::with_transport(ClientConfig::default(), transport.clone(), registry)
            .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    assert_eq!(device.match_status, wireless_battery::MatchStatus::Conflict);
    assert_eq!(
        client.read_battery(&device.id.0).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    assert_eq!(transport.opens.load(Ordering::SeqCst), 0);
}

#[test]
fn disabled_cache_cannot_return_stale() {
    let sequence = Arc::new(SequenceTransport {
        normal: ReplayTransport::new(&fixture("feature_normal.json")),
        failure: ReplayTransport::new(&fixture("feature_timeout.json")),
        opens: AtomicUsize::new(0),
    });
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            enable_cache: false,
            ..ClientConfig::default()
        },
        sequence,
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    client.read_battery(&device.id.0).unwrap();
    let error = client
        .read_battery_with_options(
            &device.id.0,
            ReadOptions {
                allow_stale: true,
                allow_experimental: true,
                ..ReadOptions::default()
            },
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Timeout);
}

struct CountingTransport {
    replay: ReplayTransport,
    enumerations: AtomicUsize,
    present: std::sync::atomic::AtomicBool,
}

impl Transport for CountingTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        self.enumerations.fetch_add(1, Ordering::SeqCst);
        if self.present.load(Ordering::SeqCst) {
            self.replay.enumerate()
        } else {
            Ok(Vec::new())
        }
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        self.replay.open(endpoint)
    }
}

#[test]
fn zero_timeout_fails_before_enumerating_and_known_device_avoids_re_enumeration() {
    let transport = Arc::new(CountingTransport {
        replay: ReplayTransport::new(&fixture("feature_normal.json")),
        enumerations: AtomicUsize::new(0),
        present: std::sync::atomic::AtomicBool::new(true),
    });
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            ..ClientConfig::default()
        },
        transport.clone(),
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();

    // Zero timeout must return Timeout immediately without triggering enumeration.
    let err = client
        .read_battery_with_options(
            "dev_unknown",
            ReadOptions {
                timeout: std::time::Duration::ZERO,
                allow_stale: false,
                allow_experimental: true,
            },
        )
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Timeout);
    assert_eq!(transport.enumerations.load(Ordering::SeqCst), 0);

    // After list_devices(), read_battery() reuses known_devices without re-enumerating.
    let device = client.list_devices().unwrap().remove(0);
    assert_eq!(transport.enumerations.load(Ordering::SeqCst), 1);
    let reading = client.read_battery(&device.id.0).unwrap();
    assert_eq!(reading.percentage, Some(72));
    assert_eq!(transport.enumerations.load(Ordering::SeqCst), 1);

    // Once unplugged and re-enumerated, cached readings and known devices are pruned.
    transport.present.store(false, Ordering::SeqCst);
    assert!(client.list_devices().unwrap().is_empty());
    let err = client.diagnose(&device.id.0).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);

    // If the same path reappears later, stale cache from the prior connection is gone.
    transport.present.store(true, Ordering::SeqCst);
    let device_again = client.list_devices().unwrap().remove(0);
    assert!(client
        .diagnose(&device_again.id.0)
        .unwrap()
        .cached_observed_at
        .is_none());
}

#[test]
fn record_transport_delegates_read_xinput() {
    let fixture =
        common::load_fixture(&common::root().join("fixtures/vendors/os/xinput_slot0.json"));
    let replay = Arc::new(ReplayTransport::new(&fixture));
    let recorder = Arc::new(RecordTransport::new(replay.clone()));
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            ..ClientConfig::default()
        },
        recorder,
        Registry::with_synthetic().unwrap(),
    )
    .unwrap();
    let device = client.list_devices().unwrap().remove(0);
    let reading = client.read_battery(&device.id.0).unwrap();
    assert_eq!(reading.level, Some(wireless_battery::BatteryLevel::Medium));
    replay.finish().unwrap();
}
