//! Project repository, issue tracker, and community contact links.
//!
//! Update `QQ_GROUP_NUMBER`, `QQ_GROUP_URL`, and `DISCORD_INVITE_URL` here
//! to propagate them to both the macOS and Windows tray applications.

use serde::Serialize;

/// GitHub source code repository URL.
pub const REPO_URL: &str = "https://github.com/bangbang0103/wireless-battery";

/// GitHub Issues URL for bug reports.
pub const ISSUES_URL: &str = "https://github.com/bangbang0103/wireless-battery/issues";

/// QQ group number displayed on hover (empty until configured).
pub const QQ_GROUP_NUMBER: &str = "";

/// QQ group share / invite URL opened on click (empty until configured).
pub const QQ_GROUP_URL: &str = "";

/// Discord server invite URL (empty until configured).
pub const DISCORD_INVITE_URL: &str = "";

/// Serializable bundle of project and community links exposed over FFI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityLinks {
    pub repo_url: &'static str,
    pub issues_url: &'static str,
    pub qq_group: &'static str,
    pub qq_url: &'static str,
    pub discord_url: &'static str,
}

/// Return the compiled-in project and community links.
#[must_use]
pub const fn community_links() -> CommunityLinks {
    CommunityLinks {
        repo_url: REPO_URL,
        issues_url: ISSUES_URL,
        qq_group: QQ_GROUP_NUMBER,
        qq_url: QQ_GROUP_URL,
        discord_url: DISCORD_INVITE_URL,
    }
}

/// Check whether an external community/repository URL uses `https://` or `http://`.
#[must_use]
pub fn is_valid_http_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}
