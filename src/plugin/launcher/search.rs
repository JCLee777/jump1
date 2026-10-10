//! Pure launcher search model and matching rules.

use std::path::PathBuf;

use crate::{platform::home_dir, plugin::protocol::*, text_search};

pub fn search(query: &str, apps: &[ApplicationEntry]) -> Vec<SearchResult> {
    let query = query.trim();
    let mut results = Vec::new();

    if query.is_empty() {
        if let Some(home) = home_dir() {
            results.push(SearchResult {
                title: ResultTitle::OpenHome,
                detail: ResultDetail::Literal(home.display().to_string()),
                kind: ResultKind::QuickAction,
                icon_path: None,
                action: CommandAction::Path(home.clone()),
            });
            let downloads = home.join("Downloads");
            if downloads.exists() {
                results.push(SearchResult {
                    title: ResultTitle::OpenDownloads,
                    detail: ResultDetail::Literal(downloads.display().to_string()),
                    kind: ResultKind::QuickAction,
                    icon_path: None,
                    action: CommandAction::Path(downloads),
                });
            }
        }
    }

    let lowered = query.to_lowercase();
    let normalized = text_search::normalize(query);
    let mut matching_apps: Vec<_> = if query.is_empty() {
        Vec::new()
    } else {
        apps.iter()
            .filter_map(|app| {
                app.search_terms
                    .iter()
                    .filter_map(|term| {
                        let candidate = if term.contains(&lowered) {
                            lowered.as_str()
                        } else if !normalized.is_empty() && term.contains(&normalized) {
                            normalized.as_str()
                        } else {
                            return None;
                        };
                        Some(if term == candidate {
                            0
                        } else if term.starts_with(candidate) {
                            1
                        } else {
                            2
                        })
                    })
                    .min()
                    .map(|score| (score, app))
            })
            .collect()
    };
    matching_apps.sort_by(|(left_score, left), (right_score, right)| {
        left_score
            .cmp(right_score)
            .then_with(|| left.name.cmp(&right.name))
    });
    results.extend(
        matching_apps
            .into_iter()
            .take(20)
            .map(|(_, app)| SearchResult {
                title: ResultTitle::Application(app.name.clone()),
                detail: ResultDetail::Literal(app.path.display().to_string()),
                kind: ResultKind::Application,
                icon_path: app.icon_path.clone(),
                action: CommandAction::Application(app.clone()),
            }),
    );

    if !query.is_empty() {
        if let Some(path) = expand_path(query) {
            if path.exists() {
                results.push(SearchResult {
                    title: ResultTitle::OpenPath(
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    detail: ResultDetail::Literal(path.display().to_string()),
                    kind: ResultKind::Path,
                    icon_path: None,
                    action: CommandAction::Path(path),
                });
            }
        }
        let url = if query.starts_with("https://") || query.starts_with("http://") {
            query.to_owned()
        } else {
            format!("https://www.google.com/search?q={}", percent_encode(query))
        };
        results.push(SearchResult {
            title: if query.starts_with("https://") || query.starts_with("http://") {
                ResultTitle::OpenUrl
            } else {
                ResultTitle::SearchWeb(query.to_owned())
            },
            detail: ResultDetail::Literal(url.clone()),
            kind: ResultKind::Web,
            icon_path: None,
            action: CommandAction::Url(url),
        });
    }

    results
}

fn expand_path(query: &str) -> Option<PathBuf> {
    if let Some(rest) = query.strip_prefix("~/") {
        return home_dir().map(|home| home.join(rest));
    }
    let path = PathBuf::from(query);
    path.is_absolute().then_some(path)
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
