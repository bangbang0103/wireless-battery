//! Package, fixture, JSON Schema, and privacy validation.

use jsonschema::JSONSchema;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;
use wireless_battery::package::schema::DevicePackage;
use wireless_battery::registry::Registry;
use wireless_battery::transport::replay::Fixture;
use wireless_battery::BatteryError;

const FIXTURE_SCHEMA: &str = include_str!("../../schemas/fixture.schema.json");
const PACKAGE_SCHEMA: &str = include_str!("../../schemas/device-package.schema.json");

pub(super) fn run(
    packages: Option<PathBuf>,
    fixtures: Option<PathBuf>,
) -> Result<(), BatteryError> {
    Registry::builtin()?;
    Registry::with_synthetic()?;
    compile_schema(PACKAGE_SCHEMA)?;
    compile_schema(FIXTURE_SCHEMA)?;
    let root = std::env::current_dir().map_err(|error| BatteryError::Internal {
        message: format!("cannot determine repository root: {error}"),
    })?;
    let package_dir = packages.unwrap_or_else(|| root.join("packages/vendors"));
    let fixture_dir = fixtures.unwrap_or_else(|| root.join("fixtures"));
    let mut package_count = 0;
    let mut loaded_packages = Vec::new();
    if package_dir.is_dir() {
        for path in files_with_extension(&package_dir, "yaml")? {
            let package = wireless_battery::package::load_package(&path, &root)?;
            loaded_packages.push(package);
            package_count += 1;
        }
    }
    reject_duplicate_matches(&loaded_packages)?;
    let synthetic_dir = root.join("fixtures/synthetic/packages");
    if synthetic_dir.is_dir() {
        for path in files_with_extension(&synthetic_dir, "yaml")? {
            wireless_battery::package::load_package(&path, &root)?;
        }
    }
    let mut fixture_count = 0;
    if fixture_dir.is_dir() {
        for path in files_with_extension(&fixture_dir, "json")? {
            if path
                .components()
                .any(|component| component.as_os_str() == "schema")
            {
                continue;
            }
            load_valid_fixture(&path)?;
            fixture_count += 1;
        }
    }
    println!("validated {package_count} package(s) and {fixture_count} fixture(s)");
    Ok(())
}

pub(super) fn load_valid_fixture(path: &Path) -> Result<Fixture, BatteryError> {
    let text = std::fs::read_to_string(path).map_err(|error| BatteryError::InvalidFixture {
        fixture: path.display().to_string(),
        reason: error.to_string(),
    })?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| BatteryError::InvalidFixture {
            fixture: path.display().to_string(),
            reason: error.to_string(),
        })?;
    validate_fixture_schema(&value, &path.display().to_string())?;
    reject_sensitive_fields(&value)?;
    serde_json::from_value(value).map_err(|error| BatteryError::InvalidFixture {
        fixture: path.display().to_string(),
        reason: error.to_string(),
    })
}

pub(super) fn validate_fixture_schema(
    value: &serde_json::Value,
    name: &str,
) -> Result<(), BatteryError> {
    let schema = compiled_fixture_schema()?;
    if let Err(errors) = schema.validate(value) {
        let reason = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(BatteryError::InvalidFixture {
            fixture: name.to_owned(),
            reason,
        });
    }
    Ok(())
}

pub(super) fn reject_sensitive_fields(value: &serde_json::Value) -> Result<(), BatteryError> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "serial" | "serial_number" | "system_path" | "username" | "pcap"
                ) {
                    return Err(BatteryError::InvalidFixture {
                        fixture: "privacy validation".to_owned(),
                        reason: format!("sensitive field is forbidden: {key}"),
                    });
                }
                reject_sensitive_fields(value)?;
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                reject_sensitive_fields(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn files_with_extension(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, BatteryError> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(directory).follow_links(false) {
        let entry = entry.map_err(|error| BatteryError::Internal {
            message: format!("directory traversal failed: {error}"),
        })?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|value| value.to_str()) == Some(extension)
        {
            paths.push(entry.path().to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
}

fn reject_duplicate_matches(packages: &[DevicePackage]) -> Result<(), BatteryError> {
    let mut matches = HashSet::new();
    for package in packages {
        if package.source == wireless_battery::package::schema::PackageSource::Synthetic {
            continue;
        }
        let fingerprint = format!("{:?}", package.r#match.all);
        if !matches.insert(fingerprint) {
            return Err(BatteryError::InvalidPackage {
                package: package.id.clone(),
                reason: "duplicate static match rule".to_owned(),
            });
        }
    }
    Ok(())
}

fn compiled_fixture_schema() -> Result<&'static JSONSchema, BatteryError> {
    static SCHEMA: OnceLock<Result<JSONSchema, String>> = OnceLock::new();
    let cached = SCHEMA.get_or_init(|| {
        let schema: serde_json::Value =
            serde_json::from_str(FIXTURE_SCHEMA).map_err(|error| error.to_string())?;
        JSONSchema::compile(&schema)
            .map_err(|error| format!("fixture schema cannot compile: {error}"))
    });
    match cached {
        Ok(schema) => Ok(schema),
        Err(message) => Err(BatteryError::Internal {
            message: message.clone(),
        }),
    }
}

fn compile_schema(text: &str) -> Result<(), BatteryError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| BatteryError::Internal {
            message: format!("embedded schema is invalid JSON: {error}"),
        })?;
    JSONSchema::compile(&value)
        .map(|_| ())
        .map_err(|error| BatteryError::Internal {
            message: format!("embedded schema is invalid: {error}"),
        })
}
