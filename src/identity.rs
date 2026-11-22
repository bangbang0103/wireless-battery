use crate::model::{DeviceId, EndpointId};
use sha2::{Digest, Sha256};

pub(crate) fn endpoint_id(stable_key: &str) -> EndpointId {
    EndpointId(format!("ep_{}", short_hash(stable_key)))
}

pub(crate) fn device_id(endpoint: &EndpointId) -> DeviceId {
    DeviceId(format!("dev_{}", short_hash(&endpoint.0)))
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(&digest[..16])
}
