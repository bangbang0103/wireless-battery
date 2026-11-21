use crate::model::{DeviceId, EndpointId};
use sha2::{Digest, Sha256};

pub(crate) fn endpoint_id(stable_key: &str) -> EndpointId {
    EndpointId(format!("ep_{}", short_hash(stable_key)))
}

pub(crate) fn device_id(endpoint: &EndpointId) -> DeviceId {
    device_id_for_slot(endpoint, None)
}

/// Derive a deterministic logical device ID, optionally scoped to a receiver slot.
///
/// Keeping slot identity separate from `EndpointId` allows future 1-to-N receiver
/// support while preserving per-endpoint I/O locking.
pub(crate) fn device_id_for_slot(endpoint: &EndpointId, slot: Option<&str>) -> DeviceId {
    match slot {
        Some(slot) => DeviceId(format!(
            "dev_{}",
            short_hash(&format!("{}:{slot}", endpoint.0))
        )),
        None => DeviceId(format!("dev_{}", short_hash(&endpoint.0))),
    }
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(&digest[..16])
}
