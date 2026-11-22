//! Application version metadata and semantic version comparison for update checks.

/// Raw semantic version compiled from Cargo workspace metadata (e.g. `"0.1.0"`).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Human-readable display version shown in UI (appends `-dev` when `dev-tools` is enabled).
#[cfg(feature = "dev-tools")]
pub const APP_DISPLAY_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), "-dev");

/// Human-readable display version shown in UI (e.g. `"v0.1.0"`).
#[cfg(not(feature = "dev-tools"))]
pub const APP_DISPLAY_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// GitHub Releases page opened when downloading an update.
pub const RELEASES_URL: &str = "https://github.com/bangbang0103/wireless-battery/releases";

/// GitHub REST API endpoint for querying the latest published release.
pub const LATEST_RELEASE_API_URL: &str =
    "https://api.github.com/repos/bangbang0103/wireless-battery/releases/latest";

/// Result of querying the latest release endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    /// A newer version is available on GitHub Releases.
    Available { latest_version: String, url: String },
    /// Current version is already up to date (or newer than the latest release).
    UpToDate { current_version: String },
    /// Network, TLS, or response parsing error.
    Failed(String),
}

/// Parse a GitHub release JSON payload (`tag_name`, `html_url`) and compare it with `current`.
#[must_use]
pub fn evaluate_release_json(body: &str, current: &str) -> UpdateCheckResult {
    #[derive(serde::Deserialize)]
    struct ReleasePayload {
        tag_name: String,
        html_url: Option<String>,
    }

    let Ok(payload) = serde_json::from_str::<ReleasePayload>(body) else {
        return UpdateCheckResult::Failed("invalid release metadata".to_owned());
    };
    let tag = payload.tag_name.trim();
    if tag.is_empty() {
        return UpdateCheckResult::Failed("empty release tag".to_owned());
    }
    if is_newer_version(current, tag) {
        let normalized = if tag.starts_with('v') || tag.starts_with('V') {
            format!("v{}", &tag[1..])
        } else {
            format!("v{tag}")
        };
        let url = payload
            .html_url
            .filter(|u| u.starts_with("https://"))
            .unwrap_or_else(|| RELEASES_URL.to_owned());
        UpdateCheckResult::Available {
            latest_version: normalized,
            url,
        }
    } else {
        UpdateCheckResult::UpToDate {
            current_version: APP_DISPLAY_VERSION.to_owned(),
        }
    }
}

/// Compare two semantic version strings (accepting optional leading `v` and `-prerelease` suffix).
/// Returns `true` when `candidate` is strictly newer than `current`.
#[must_use]
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    let Some(cur) = parse_semver_tuple(current) else {
        return false;
    };
    let Some(cand) = parse_semver_tuple(candidate) else {
        return false;
    };
    cand > cur
}

fn parse_semver_tuple(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let core = trimmed.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_versions_accurately() {
        assert!(is_newer_version("0.1.0", "v0.2.0"));
        assert!(is_newer_version("v0.1.0-dev", "0.1.1"));
        assert!(is_newer_version("0.1.9", "v0.1.10"));
        assert!(!is_newer_version("0.1.0", "v0.1.0"));
        assert!(!is_newer_version("0.2.0", "v0.1.9"));
        assert!(!is_newer_version("0.1.0", "invalid"));
    }

    #[test]
    fn evaluates_github_release_json_payload() {
        let newer = r#"{"tag_name":"v9.9.9","html_url":"https://github.com/bangbang0103/wireless-battery/releases/tag/v9.9.9"}"#;
        match evaluate_release_json(newer, "0.1.0") {
            UpdateCheckResult::Available {
                latest_version,
                url,
            } => {
                assert_eq!(latest_version, "v9.9.9");
                assert!(url.ends_with("/v9.9.9"));
            }
            other => panic!("expected Available, got {other:?}"),
        }

        let same = r#"{"tag_name":"v0.1.0","html_url":"https://github.com/bangbang0103/wireless-battery/releases/tag/v0.1.0"}"#;
        assert!(matches!(
            evaluate_release_json(same, "0.1.0"),
            UpdateCheckResult::UpToDate { .. }
        ));
    }
}
