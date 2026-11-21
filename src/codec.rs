//! Canonical hexadecimal payload encoding and decoding.

/// Encode payload bytes as uppercase space-separated hexadecimal pairs.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Decode whitespace-separated or contiguous hexadecimal bytes.
pub fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    let compact = value.split_whitespace().collect::<String>();
    if compact.len() % 2 != 0 {
        return Err("hex payload has an odd number of digits".to_owned());
    }
    hex::decode(compact).map_err(|error| format!("invalid hex payload: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, encode_hex};

    #[test]
    fn round_trips_canonical_hex() {
        let bytes = [0x00, 0x0A, 0x48, 0xFF];
        let encoded = encode_hex(&bytes);
        assert_eq!(encoded, "00 0A 48 FF");
        assert_eq!(decode_hex(&encoded).unwrap(), bytes);
        assert_eq!(decode_hex("000a48ff").unwrap(), bytes);
        assert!(decode_hex("ABC").is_err());
    }
}
