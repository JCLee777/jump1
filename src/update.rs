//! GitHub Release lookup for Apple Silicon macOS builds.

use std::cmp::Ordering;

use crate::i18n::Locale;

#[derive(Clone, Debug)]
pub struct ReleaseEntry {
    pub version: String,
    pub name: String,
    pub notes: ReleaseNotes,
    pub release_url: String,
    pub download_url: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ReleaseNotes {
    zh_cn: Option<String>,
    en_us: Option<String>,
    fallback: String,
}

impl ReleaseNotes {
    fn from_markdown(markdown: &str) -> Self {
        #[derive(Clone, Copy)]
        enum Section {
            ZhCn,
            EnUs,
        }

        let mut result = Self::default();
        let mut section = None;
        let mut found_localized_section = false;

        for line in markdown.lines() {
            match line.trim() {
                "### 简体中文" => {
                    section = Some(Section::ZhCn);
                    found_localized_section = true;
                    continue;
                }
                "### English" => {
                    section = Some(Section::EnUs);
                    found_localized_section = true;
                    continue;
                }
                _ => {}
            }

            let Some(section) = section else { continue };
            let content = match section {
                Section::ZhCn => result.zh_cn.get_or_insert_with(String::new),
                Section::EnUs => result.en_us.get_or_insert_with(String::new),
            };
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
        }

        if found_localized_section {
            result.zh_cn = result
                .zh_cn
                .take()
                .map(|notes| notes.trim().to_owned())
                .filter(|notes| !notes.is_empty());
            result.en_us = result
                .en_us
                .take()
                .map(|notes| notes.trim().to_owned())
                .filter(|notes| !notes.is_empty());
        } else {
            result.fallback = markdown.trim().to_owned();
        }

        result
    }

    pub fn for_locale(&self, locale: Locale) -> &str {
        let (preferred, other) = match locale {
            Locale::ZhCn => (self.zh_cn.as_deref(), self.en_us.as_deref()),
            Locale::EnUs => (self.en_us.as_deref(), self.zh_cn.as_deref()),
        };
        preferred
            .filter(|notes| !notes.is_empty())
            .or_else(|| other.filter(|notes| !notes.is_empty()))
            .unwrap_or(&self.fallback)
    }
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
                notes: ReleaseNotes::from_markdown(
                    release
                        .get("body")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default(),
                ),
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
        match self.api_releases() {
            Ok(releases) if !releases.is_empty() => Ok(releases),
            Ok(releases) => self.atom_releases().or(Ok(releases)),
            Err(api_error) => self.atom_releases().map_err(|feed_error| {
                format!("{api_error}；GitHub Releases feed 回退失败：{feed_error}")
            }),
        }
    }

    fn api_releases(&self) -> Result<Vec<serde_json::Value>, String> {
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

    fn atom_releases(&self) -> Result<Vec<serde_json::Value>, String> {
        const RELEASES_FEED: &str = "https://github.com/JCLee777/jump1/releases.atom";
        const RELEASE_PAGE_PREFIX: &str = "https://github.com/JCLee777/jump1/releases/tag/";

        let response = self
            .client
            .get(RELEASES_FEED)
            .header("Accept", "application/atom+xml")
            .send()
            .map_err(|error| format!("无法读取 Releases feed：{error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("Releases feed 返回状态码 {status}"));
        }
        let feed = response
            .text()
            .map_err(|error| format!("无法读取 Releases feed 内容：{error}"))?;
        let document = roxmltree::Document::parse(&feed)
            .map_err(|error| format!("无法解析 Releases feed：{error}"))?;

        let releases = document
            .root_element()
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "entry")
            .filter_map(|entry| {
                let tag = entry
                    .children()
                    .find(|node| {
                        node.is_element()
                            && node.tag_name().name() == "link"
                            && node
                                .attribute("rel")
                                .is_none_or(|relation| relation == "alternate")
                    })
                    .and_then(|node| node.attribute("href"))?
                    .strip_prefix(RELEASE_PAGE_PREFIX)?;
                let version_number = tag.strip_prefix('v').unwrap_or(tag);
                if !is_stable_release_version(version_number) {
                    return None;
                }

                let title = entry
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "title")
                    .and_then(|node| node.text())
                    .unwrap_or(tag);
                let body = entry
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "content")
                    .and_then(|node| node.text())
                    .map(github_html_to_markdown)
                    .unwrap_or_default();
                let published_at = entry
                    .children()
                    .find(|node| node.is_element() && node.tag_name().name() == "updated")
                    .and_then(|node| node.text())
                    .map(|value| value.chars().take(10).collect::<String>());
                let download_name = format!("Jump1-{version_number}-aarch64.dmg");

                Some(serde_json::json!({
                    "tag_name": tag,
                    "name": title,
                    "body": body,
                    "html_url": format!("{RELEASE_PAGE_PREFIX}{tag}"),
                    "published_at": published_at,
                    "draft": false,
                    "prerelease": false,
                    "assets": [{
                        "name": download_name,
                        "browser_download_url": format!(
                            "https://github.com/JCLee777/jump1/releases/download/{tag}/{download_name}"
                        )
                    }]
                }))
            })
            .collect::<Vec<_>>();

        if releases.is_empty() {
            return Err("Releases feed 中没有可用的稳定版本".to_owned());
        }
        Ok(releases)
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn is_stable_release_version(version: &str) -> bool {
    let parts = version.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn github_html_to_markdown(html: &str) -> String {
    let mut markdown = String::new();
    let mut chars = html.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '<' {
            let mut tag = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '>' {
                    closed = true;
                    break;
                }
                tag.push(next);
            }
            if !closed {
                markdown.push('<');
                markdown.push_str(&tag);
                break;
            }

            let tag = tag.trim().to_ascii_lowercase();
            let closing = tag.starts_with('/');
            let tag_name = tag
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_end_matches('/');
            match (tag_name, closing) {
                ("h1", false) => markdown.push_str("\n\n# "),
                ("h2", false) => markdown.push_str("\n\n## "),
                ("h3", false) => markdown.push_str("\n\n### "),
                ("h1" | "h2" | "h3" | "p", true) => markdown.push_str("\n\n"),
                ("p", false) => markdown.push_str("\n\n"),
                ("li", false) => markdown.push_str("\n- "),
                ("li", true) => markdown.push('\n'),
                ("br", _) => markdown.push('\n'),
                _ => {}
            }
        } else {
            markdown.push(character);
        }
    }

    let decoded = decode_html_entities(&markdown);
    decoded
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn decode_html_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(ampersand) = remaining.find('&') {
        decoded.push_str(&remaining[..ampersand]);
        remaining = &remaining[ampersand..];

        let entity_end = remaining
            .find(';')
            .filter(|end| *end <= 16)
            .map(|end| (end, &remaining[1..end]));
        let entity = entity_end.and_then(|(end, entity)| {
            let value = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" | "#39" => Some('\''),
                "nbsp" => Some(' '),
                numeric if numeric.starts_with("#x") || numeric.starts_with("#X") => {
                    u32::from_str_radix(&numeric[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                numeric if numeric.starts_with('#') => {
                    numeric[1..].parse().ok().and_then(char::from_u32)
                }
                _ => None,
            }?;
            Some((end, value))
        });

        if let Some((end, value)) = entity {
            decoded.push(value);
            remaining = &remaining[end + 1..];
        } else {
            decoded.push('&');
            remaining = &remaining[1..];
        }
    }

    decoded.push_str(remaining);
    decoded
}
