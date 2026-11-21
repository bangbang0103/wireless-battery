//! Serializable device-package schema types.

use crate::model::{Capabilities, SupportStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Complete declarative device package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePackage {
    /// Package schema version; only 1 is supported.
    pub schema_version: u32,
    /// Stable package ID.
    pub id: String,
    /// Semantic package version.
    pub version: String,
    /// Lifecycle status.
    pub status: SupportStatus,
    /// Display name.
    pub name: String,
    /// Evidence origin.
    #[serde(default)]
    pub source: PackageSource,
    /// Conjunctive static match rule.
    pub r#match: MatchRule,
    /// Coarse runtime capabilities.
    pub capabilities: Capabilities,
    /// Transport policy.
    pub transport: PackageTransport,
    /// Bounded query sequence.
    pub query: Query,
    /// Validation and extraction rules for the final response.
    pub response: ResponseSpec,
    /// Normalized fields extracted from the final response.
    pub fields: Fields,
    /// Evidence references and per-capability verification.
    #[serde(default)]
    pub evidence: Evidence,
}

/// Package evidence origin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageSource {
    /// A package claiming a real protocol.
    #[default]
    Real,
    /// A fictional package usable only for tests and replay.
    Synthetic,
}

/// All supplied fields must match; missing fields are unconstrained.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchRule {
    /// Conjunctive conditions.
    pub all: MatchConditions,
}

/// Static HID match conditions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchConditions {
    /// USB vendor ID.
    pub vid: Option<u16>,
    /// USB product ID.
    pub pid: Option<u16>,
    /// HID usage page.
    pub usage_page: Option<u16>,
    /// HID usage.
    pub usage: Option<u16>,
    /// OS input report length.
    pub input_report_length: Option<usize>,
    /// OS output report length.
    pub output_report_length: Option<usize>,
    /// OS feature report length.
    pub feature_report_length: Option<usize>,
}

/// Declarative transport policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageTransport {
    /// HID operation family.
    pub kind: TransportKind,
    /// Per-I/O timeout in milliseconds.
    pub timeout_ms: u64,
    /// Minimum interval between queries in milliseconds.
    #[serde(default)]
    pub cooldown_ms: u64,
}

/// Supported declarative operation families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    /// Feature report request and response.
    HidFeature,
    /// Output report request and input report response.
    HidOutput,
}

/// Query containing at most four request-response rounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    /// Ordered fixed rounds.
    pub steps: Vec<QueryStep>,
}

/// One request-response round.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryStep {
    /// Optional diagnostic ID; generated positions remain unique.
    pub id: Option<String>,
    /// Feature request for `hid_feature`.
    pub send_feature: Option<RequestSpec>,
    /// Feature response for `hid_feature`.
    pub receive_feature: Option<ReceiveSpec>,
    /// Output request for `hid_output`.
    pub write_output: Option<RequestSpec>,
    /// Input response for `hid_output`.
    pub read_input: Option<ReceiveSpec>,
}

/// Fixed request bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    /// Report ID stored separately from payload.
    pub report_id: u8,
    /// Payload bytes excluding report ID.
    pub payload_hex: String,
}

/// Expected response and optional intermediate validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiveSpec {
    /// Expected report ID.
    pub report_id: u8,
    /// Expected payload length excluding report ID.
    pub length: usize,
    /// Required fixed bytes.
    #[serde(default)]
    pub constants: Vec<Constant>,
    /// Optional checksum.
    pub checksum: Option<Checksum>,
}

/// Final response validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseSpec {
    /// Expected report ID.
    pub report_id: u8,
    /// Expected payload length excluding report ID.
    pub length: usize,
    /// Required fixed bytes.
    #[serde(default)]
    pub constants: Vec<Constant>,
    /// Optional checksum.
    pub checksum: Option<Checksum>,
}

/// Required bytes at a payload offset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constant {
    /// Payload offset excluding report ID.
    pub offset: usize,
    /// Required bytes in hexadecimal.
    pub hex: String,
}

/// `sum8` or `xor8` over an inclusive payload range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checksum {
    /// Checksum algorithm.
    pub algorithm: ChecksumAlgorithm,
    /// Inclusive start and end payload offsets.
    pub range: [usize; 2],
    /// Payload offset containing the expected checksum.
    pub offset: usize,
}

/// Supported checksum algorithms.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumAlgorithm {
    /// Wrapping sum of all bytes in the range.
    Sum8,
    /// Exclusive OR of all bytes in the range.
    Xor8,
}

/// Fields available in the final response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// Exact percentage.
    pub percentage: Option<FieldSpec>,
    /// Discrete level.
    pub level: Option<FieldSpec>,
    /// Voltage before conversion to millivolts.
    pub voltage_mv: Option<FieldSpec>,
    /// Charge state.
    #[serde(alias = "charging")]
    pub charge_state: Option<FieldSpec>,
    /// Explicit reachability state.
    pub reachability: Option<FieldSpec>,
}

/// One `u8` or boolean-bit field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    /// Payload offset.
    pub offset: usize,
    /// Extraction type.
    pub r#type: FieldType,
    /// Bit index for `bool_bit`.
    pub bit: Option<u8>,
    /// Inclusive valid range for percentages.
    pub valid_range: Option<[u8; 2]>,
    /// Explicit values meaning unknown rather than zero.
    #[serde(default)]
    pub unknown_values: Vec<u8>,
    /// Stringified source value to normalized target value.
    #[serde(default)]
    pub map: BTreeMap<String, String>,
    /// Checked multiplier, primarily for voltage millivolts.
    #[serde(default = "default_scale")]
    pub scale: u32,
}

fn default_scale() -> u32 {
    1
}

/// Field extraction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// One unsigned byte.
    U8,
    /// One bit rendered as `true` or `false` before mapping.
    BoolBit,
}

/// Evidence paths and fine-grained verification labels.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Evidence {
    /// Fixture paths relative to the repository root.
    pub fixtures: Vec<String>,
    /// Provenance Markdown path relative to the repository root.
    pub provenance: Option<String>,
    /// Capability or state name to `verified`, `experimental`, `unverified`, or `unsupported`.
    pub capability_status: BTreeMap<String, String>,
}
