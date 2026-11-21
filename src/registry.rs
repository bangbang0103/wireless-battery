//! Static provider registry and embedded package loading.

use crate::error::BatteryError;
use crate::package::schema::PackageSource;
use crate::providers::declarative::DeclarativeProvider;
use crate::providers::{MatchDecision, Provider};
use include_dir::{include_dir, Dir};
use std::collections::HashSet;
use std::sync::Arc;

static REAL_PACKAGES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/packages/vendors");
static SYNTHETIC_PACKAGES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/fixtures/synthetic/packages");

/// Immutable provider collection.
#[derive(Default, Clone)]
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    /// Build a registry from explicit providers, rejecting duplicate IDs.
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Result<Self, BatteryError> {
        let mut ids = HashSet::new();
        for provider in &providers {
            if !ids.insert(provider.id().to_owned()) {
                return Err(BatteryError::Internal {
                    message: format!("duplicate provider ID: {}", provider.id()),
                });
            }
        }
        Ok(Self { providers })
    }

    /// Load only real embedded packages and built-in providers. Synthetic fixtures are never real support.
    pub fn builtin() -> Result<Self, BatteryError> {
        let mut registry = Self::from_directory(&REAL_PACKAGES, PackageSource::Real)?;
        registry.providers.extend(crate::providers::os::providers());
        registry
            .providers
            .extend(crate::providers::vendors::providers());
        Self::new(registry.providers)
    }

    /// Load real and explicitly synthetic packages for replay and tests.
    pub fn with_synthetic() -> Result<Self, BatteryError> {
        let real = Self::builtin()?;
        let synthetic = Self::from_directory(&SYNTHETIC_PACKAGES, PackageSource::Synthetic)?;
        let mut providers = real.providers;
        providers.extend(synthetic.providers);
        Self::new(providers)
    }

    /// Return all registered providers.
    #[must_use]
    pub fn providers(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    /// Find a provider by stable ID.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|provider| provider.id() == id)
            .cloned()
    }

    pub(crate) fn matches(
        &self,
        endpoint: &crate::model::Endpoint,
    ) -> Vec<(Arc<dyn Provider>, MatchDecision)> {
        self.providers
            .iter()
            .filter_map(|provider| {
                let decision = provider.matches(endpoint);
                (decision != MatchDecision::NoMatch).then(|| (Arc::clone(provider), decision))
            })
            .collect()
    }

    fn from_directory(
        directory: &Dir<'_>,
        expected_source: PackageSource,
    ) -> Result<Self, BatteryError> {
        let mut files = Vec::new();
        collect_yaml_files(directory, &mut files);
        files.sort_by(|left, right| left.path().cmp(right.path()));
        let mut providers: Vec<Arc<dyn Provider>> = Vec::new();
        for file in files {
            let name = file.path().display().to_string();
            let yaml = file.contents_utf8().ok_or_else(|| BatteryError::Internal {
                message: format!("embedded package is not UTF-8: {name}"),
            })?;
            let package = crate::package::parse_package(yaml, &name)?;
            if package.source != expected_source {
                return Err(BatteryError::invalid_package(
                    name,
                    "package source is not valid in this directory",
                ));
            }
            providers.push(Arc::new(DeclarativeProvider::new(package)));
        }
        Self::new(providers)
    }
}

fn collect_yaml_files<'a>(directory: &'a Dir<'a>, out: &mut Vec<&'a include_dir::File<'a>>) {
    for file in directory.files() {
        if file
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("yaml")
        {
            out.push(file);
        }
    }
    for subdir in directory.dirs() {
        collect_yaml_files(subdir, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use include_dir::{DirEntry, File};

    #[test]
    fn from_directory_loads_nested_vendor_packages() {
        const YAML: &[u8] = include_bytes!("../fixtures/synthetic/packages/feature_mouse.yaml");
        static LEAF: &[DirEntry<'static>] = &[DirEntry::File(File::new(
            "vendor_a/feature_mouse.yaml",
            YAML,
        ))];
        static ROOT: &[DirEntry<'static>] = &[DirEntry::Dir(Dir::new("vendor_a", LEAF))];
        let dir = Dir::new("", ROOT);
        let registry = Registry::from_directory(&dir, PackageSource::Synthetic).unwrap();
        assert!(registry.find("synthetic.feature.mouse").is_some());
    }
}
