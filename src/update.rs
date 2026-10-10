//! GitHub Release lookup for Apple Silicon macOS builds.

use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct ReleaseEntry {
    pub version: String,
    pub name: String,
    pub notes: String,
    pub release_url: String,
    pub download_url: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub enum UpdateStatus {
    #[default]
    NotChecked,
    Checking,
    Ready(Vec<ReleaseEntry>),
    Failed(String),
}

pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    let Some(candidate) = version_parts(candidate) else {
        return false;
    };
    let Some(current) = version_parts(current) else {
        return false;
    };
    candidate.cmp(&current) == Ordering::Greater
}

fn version_parts(version: &str) -> Option<[u64; 3]> {
    let version = version.trim().strip_prefix('v').unwrap_or(version.trim());
    let mut parts = version.split('.');
    Some([
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ])
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn fetch_release_history() -> Result<Vec<ReleaseEntry>, String> {
    let releases = GitHubReleaseClient::new()?.releases()?;

    Ok(releases
        .into_iter()
        .filter(|release| {
            !release
                .get("draft")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
                && !release
                    .get("prerelease")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true)
        })
        .filter_map(|release| {
            let version = release.get("tag_name")?.as_str()?.to_owned();
            let release_url = release.get("html_url")?.as_str()?.to_owned();
            let version_number = version.strip_prefix('v').unwrap_or(&version).to_owned();
            let download_name = format!("Jump1-{version_number}-aarch64.dmg");
            let download_url = release
                .get("assets")?
                .as_array()?
                .iter()
                .find(|asset| {
                    asset.get("name").and_then(serde_json::Value::as_str) == Some(&download_name)
                })
                .and_then(|asset| asset.get("browser_download_url"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            Some(ReleaseEntry {
                version,
                name: release
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or(&version_number)
                    .to_owned(),
                notes: release
                    .get("body")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_owned(),
                release_url,
                download_url,
                published_at: release
                    .get("published_at")
                    .and_then(serde_json::Value::as_str)
                    .map(|value| value.chars().take(10).collect()),
            })
        })
        .collect())
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct GitHubReleaseClient {
    client: reqwest::blocking::Client,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
impl GitHubReleaseClient {
    fn new() -> Result<Self, String> {
        use std::time::Duration;

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("Jump1/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| format!("无法初始化更新客户端：{error}"))?;
        Ok(Self { client })
    }

    fn releases(&self) -> Result<Vec<serde_json::Value>, String> {
        const RELEASES_API: &str =
            "https://api.github.com/repos/JCLee777/jump1/releases?per_page=10";

        let response = self
            .client
            .get(RELEASES_API)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .map_err(|error| format!("GitHub 更新请求失败：{error}"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("GitHub 返回状态码 {status}"));
        }
        serde_json::from_reader(response)
            .map_err(|error| format!("无法解析 GitHub 版本信息：{error}"))
    }
}
