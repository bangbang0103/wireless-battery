use super::schema::{
    Checksum, Constant, DevicePackage, FieldSpec, FieldType, Fields, PackageSource,
};
use crate::error::BatteryError;
use crate::model::SupportStatus;
use jsonschema::JSONSchema;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path};
use std::sync::OnceLock;

const PACKAGE_SCHEMA: &str = include_str!("../../schemas/device-package.schema.json");
const MAX_REPORT_PAYLOAD: usize = 4_096;
const MAX_QUERY_ROUNDS: usize = 4;

fn compiled_package_schema() -> Result<&'static JSONSchema, BatteryError> {
    static SCHEMA: OnceLock<Result<JSONSchema, String>> = OnceLock::new();
    let cached = SCHEMA.get_or_init(|| {
        let schema: serde_json::Value = serde_json::from_str(PACKAGE_SCHEMA)
            .map_err(|error| format!("embedded package schema is invalid: {error}"))?;
        JSONSchema::compile(&schema)
            .map_err(|error| format!("embedded package schema cannot compile: {error}"))
    });
    match cached {
        Ok(schema) => Ok(schema),
        Err(message) => Err(BatteryError::Internal {
            message: message.clone(),
        }),
    }
}

/// Parse a YAML package, apply JSON Schema, then apply semantic validation.
pub fn parse_package(yaml: &str, package_name: &str) -> Result<DevicePackage, BatteryError> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)
        .map_err(|error| BatteryError::invalid_package(package_name, error.to_string()))?;
    let json = serde_json::to_value(value)
        .map_err(|error| BatteryError::invalid_package(package_name, error.to_string()))?;
    let compiled = compiled_package_schema()?;
    if let Err(errors) = compiled.validate(&json) {
        let reason = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(BatteryError::invalid_package(package_name, reason));
    }
    let package: DevicePackage = serde_json::from_value(json)
        .map_err(|error| BatteryError::invalid_package(package_name, error.to_string()))?;
    validate_package(&package, package_name, None)?;
    Ok(package)
}

/// Load a package from disk and validate repository-relative evidence references.
pub fn load_package(path: &Path, repository_root: &Path) -> Result<DevicePackage, BatteryError> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("package.yaml");
    let yaml = std::fs::read_to_string(path)
        .map_err(|error| BatteryError::invalid_package(name, error.to_string()))?;
    let package = parse_package(&yaml, name)?;
    validate_package(&package, name, Some(repository_root))?;
    Ok(package)
}

/// Validate semantic constraints that JSON Schema cannot express.
pub fn validate_package(
    package: &DevicePackage,
    package_name: &str,
    repository_root: Option<&Path>,
) -> Result<(), BatteryError> {
    let fail = |reason: String| BatteryError::invalid_package(package_name, reason);
    if package.schema_version != 1 {
        return Err(fail("schema_version must be 1".to_owned()));
    }
    if package.id.is_empty()
        || !package.id.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || "._-".contains(character)
        })
    {
        return Err(fail(
            "id must contain only lowercase ASCII letters, digits, '.', '_' or '-'".to_owned(),
        ));
    }
    semver::Version::parse(&package.version)
        .map_err(|error| fail(format!("version is not semantic: {error}")))?;
    if package.status == SupportStatus::Unknown {
        return Err(fail("unknown is not a valid package status".to_owned()));
    }
    if package.source == PackageSource::Synthetic && package.status == SupportStatus::Verified {
        return Err(fail("synthetic packages cannot be verified".to_owned()));
    }
    if package.status == SupportStatus::Verified && package.evidence.provenance.is_none() {
        return Err(fail("verified packages require provenance".to_owned()));
    }
    if package.r#match.all == Default::default() {
        return Err(fail("match.all cannot be empty".to_owned()));
    }
    if package.transport.timeout_ms == 0 || package.transport.timeout_ms > 10_000 {
        return Err(fail("timeout_ms must be in 1..=10000".to_owned()));
    }
    if package.transport.cooldown_ms > 60_000 {
        return Err(fail("cooldown_ms cannot exceed 60000".to_owned()));
    }
    if package.query.steps.is_empty() || package.query.steps.len() > MAX_QUERY_ROUNDS {
        return Err(fail(
            "query must contain between one and four rounds".to_owned(),
        ));
    }

    let mut step_ids = HashSet::new();
    for (index, step) in package.query.steps.iter().enumerate() {
        if let Some(id) = &step.id {
            if id.is_empty() || !step_ids.insert(id) {
                return Err(fail(format!(
                    "duplicate or empty step ID at round {}",
                    index + 1
                )));
            }
        }
        let (request, receive) = match package.transport.kind {
            super::schema::TransportKind::HidFeature => {
                if step.write_output.is_some() || step.read_input.is_some() {
                    return Err(fail(format!(
                        "output operations in feature round {}",
                        index + 1
                    )));
                }
                (step.send_feature.as_ref(), step.receive_feature.as_ref())
            }
            super::schema::TransportKind::HidOutput => {
                if step.send_feature.is_some() || step.receive_feature.is_some() {
                    return Err(fail(format!(
                        "feature operations in output round {}",
                        index + 1
                    )));
                }
                (step.write_output.as_ref(), step.read_input.as_ref())
            }
        };
        let request =
            request.ok_or_else(|| fail(format!("missing request in round {}", index + 1)))?;
        let receive =
            receive.ok_or_else(|| fail(format!("missing response in round {}", index + 1)))?;
        let payload = crate::codec::decode_hex(&request.payload_hex)
            .map_err(|error| fail(format!("invalid request in round {}: {error}", index + 1)))?;
        if payload.len() > MAX_REPORT_PAYLOAD {
            return Err(fail(format!("request too large in round {}", index + 1)));
        }
        validate_response_parts(
            receive.length,
            &receive.constants,
            receive.checksum.as_ref(),
        )
        .map_err(|error| fail(format!("round {}: {error}", index + 1)))?;
    }

    let final_receive = package
        .query
        .steps
        .last()
        .and_then(|step| step.receive_feature.as_ref().or(step.read_input.as_ref()))
        .ok_or_else(|| fail("final response is absent".to_owned()))?;
    if final_receive.report_id != package.response.report_id
        || final_receive.length != package.response.length
    {
        return Err(fail(
            "final response report ID and length must match the final query round".to_owned(),
        ));
    }
    validate_response_parts(
        package.response.length,
        &package.response.constants,
        package.response.checksum.as_ref(),
    )
    .map_err(fail)?;
    validate_fields(&package.fields, package.response.length).map_err(fail)?;
    validate_capability_fields(package).map_err(fail)?;

    for (capability, status) in &package.evidence.capability_status {
        if !matches!(
            status.as_str(),
            "verified" | "experimental" | "unverified" | "unsupported"
        ) {
            return Err(fail(format!("invalid evidence status for {capability}")));
        }
    }
    if let Some(root) = repository_root {
        for reference in package
            .evidence
            .fixtures
            .iter()
            .chain(package.evidence.provenance.iter())
        {
            validate_reference(root, reference).map_err(fail)?;
        }
    }
    Ok(())
}

fn validate_response_parts(
    length: usize,
    constants: &[Constant],
    checksum: Option<&Checksum>,
) -> Result<(), String> {
    if length == 0 || length > MAX_REPORT_PAYLOAD {
        return Err(format!(
            "response length must be in 1..={MAX_REPORT_PAYLOAD}"
        ));
    }
    for constant in constants {
        let bytes = crate::codec::decode_hex(&constant.hex)?;
        if bytes.is_empty() || constant.offset.saturating_add(bytes.len()) > length {
            return Err(format!(
                "constant at offset {} exceeds response",
                constant.offset
            ));
        }
    }
    if let Some(checksum) = checksum {
        let [start, end] = checksum.range;
        if start > end || end >= length || checksum.offset >= length {
            return Err("checksum range or offset exceeds response".to_owned());
        }
        if (start..=end).contains(&checksum.offset) {
            return Err("checksum range cannot include its own offset".to_owned());
        }
    }
    Ok(())
}

enum FieldOccupancy<'a> {
    WholeByte(&'a str),
    Bits(HashMap<u8, &'a str>),
}

fn validate_fields(fields: &Fields, response_length: usize) -> Result<(), String> {
    let entries = [
        ("percentage", fields.percentage.as_ref()),
        ("level", fields.level.as_ref()),
        ("voltage_mv", fields.voltage_mv.as_ref()),
        ("charge_state", fields.charge_state.as_ref()),
        ("reachability", fields.reachability.as_ref()),
    ];
    let mut offsets = HashMap::<usize, FieldOccupancy<'_>>::new();
    for (name, field) in entries
        .into_iter()
        .filter_map(|(name, field)| field.map(|field| (name, field)))
    {
        validate_field(name, field, response_length)?;
        match field.r#type {
            FieldType::U8 => {
                if let Some(existing) = offsets.get(&field.offset) {
                    let previous_name = match existing {
                        FieldOccupancy::WholeByte(previous) => *previous,
                        FieldOccupancy::Bits(bits) => {
                            bits.values().copied().next().unwrap_or("unknown")
                        }
                    };
                    return Err(format!(
                        "fields {previous_name} and {name} overlap at offset {}",
                        field.offset
                    ));
                }
                offsets.insert(field.offset, FieldOccupancy::WholeByte(name));
            }
            FieldType::BoolBit => {
                let bit = field.bit.unwrap_or(0);
                match offsets.get_mut(&field.offset) {
                    Some(FieldOccupancy::WholeByte(previous_name)) => {
                        return Err(format!(
                            "fields {previous_name} and {name} overlap at offset {}",
                            field.offset
                        ));
                    }
                    Some(FieldOccupancy::Bits(bits)) => {
                        if let Some(previous_name) = bits.insert(bit, name) {
                            return Err(format!(
                                "fields {previous_name} and {name} overlap at offset {} bit {bit}",
                                field.offset
                            ));
                        }
                    }
                    None => {
                        let mut bits = HashMap::new();
                        bits.insert(bit, name);
                        offsets.insert(field.offset, FieldOccupancy::Bits(bits));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_field(name: &str, field: &FieldSpec, response_length: usize) -> Result<(), String> {
    if field.offset >= response_length {
        return Err(format!("field {name} exceeds response"));
    }
    if field.scale == 0 {
        return Err(format!("field {name} has zero scale"));
    }
    match field.r#type {
        FieldType::U8 if field.bit.is_some() => {
            return Err(format!("u8 field {name} cannot specify bit"));
        }
        FieldType::BoolBit if field.bit.is_none() || field.bit > Some(7) => {
            return Err(format!("field {name} bit must be specified in 0..=7"));
        }
        _ => {}
    }
    if name == "percentage" {
        if field.r#type != FieldType::U8 || field.scale != 1 {
            return Err("percentage must be an unscaled u8".to_owned());
        }
        let [minimum, maximum] = field.valid_range.unwrap_or([0, 100]);
        if minimum > maximum || maximum > 100 {
            return Err("percentage valid_range must be within 0..=100".to_owned());
        }
    }
    if matches!(name, "level" | "charge_state" | "reachability") && field.map.is_empty() {
        return Err(format!("field {name} requires a value map"));
    }
    let allowed_targets: &[&str] = match name {
        "level" => &["critical", "low", "medium", "high", "full"],
        "charge_state" => &["charging", "not_charging", "full", "discharging", "unknown"],
        "reachability" => &["online", "sleeping", "offline", "unknown"],
        _ => &[],
    };
    if field
        .map
        .values()
        .any(|target| !allowed_targets.is_empty() && !allowed_targets.contains(&target.as_str()))
    {
        return Err(format!("field {name} contains an unknown enum target"));
    }
    if field.r#type == FieldType::BoolBit
        && field.map.keys().any(|key| key != "true" && key != "false")
    {
        return Err(format!(
            "bool_bit field {name} map keys must be true or false"
        ));
    }
    Ok(())
}

fn validate_capability_fields(package: &DevicePackage) -> Result<(), String> {
    let capabilities = package.capabilities;
    let fields = &package.fields;
    let checks = [
        (
            capabilities.percentage,
            fields.percentage.is_some(),
            "percentage",
        ),
        (
            capabilities.discrete_level,
            fields.level.is_some(),
            "discrete_level",
        ),
        (capabilities.voltage, fields.voltage_mv.is_some(), "voltage"),
        (
            capabilities.charge_state,
            fields.charge_state.is_some(),
            "charge_state",
        ),
        (
            capabilities.reachability,
            fields.reachability.is_some(),
            "reachability",
        ),
    ];
    for (capability, field, name) in checks {
        if capability != field {
            return Err(format!(
                "capability {name} and its field definition disagree"
            ));
        }
    }
    Ok(())
}

fn validate_reference(root: &Path, reference: &str) -> Result<(), String> {
    let path = Path::new(reference);
    if reference.is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe evidence path: {reference}"));
    }
    if !root.join(path).is_file() {
        return Err(format!("evidence path does not exist: {reference}"));
    }
    Ok(())
}
