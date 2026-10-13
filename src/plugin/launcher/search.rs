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
            let documents = home.join("Documents");
            if documents.exists() {
                results.push(SearchResult {
                    title: ResultTitle::OpenDocuments,
                    detail: ResultDetail::Literal(documents.display().to_string()),
                    kind: ResultKind::QuickAction,
                    icon_path: None,
                    action: CommandAction::Path(documents),
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
                    .filter_map(|term| application_match_score(term, &lowered, &normalized))
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
                detail: ResultDetail::Literal(String::new()),
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

fn application_match_score(term: &str, lowered_query: &str, normalized_query: &str) -> Option<u8> {
    if lowered_query.is_empty() {
        return None;
    }

    let normalized_term = text_search::normalize(term);
    if term == lowered_query
        || (!normalized_query.is_empty() && normalized_term == normalized_query)
    {
        return Some(0);
    }
    if term.starts_with(lowered_query)
        || (!normalized_query.is_empty() && normalized_term.starts_with(normalized_query))
    {
        return Some(1);
    }

    let token_prefix_match = term
        .split(|character: char| !character.is_alphanumeric())
        .any(|token| token.starts_with(lowered_query));
    if token_prefix_match {
        return Some(2);
    }

    // Short queries are common launcher prefixes. Matching them in the middle
    // of words makes unrelated shortcuts such as “Administrative Tools” noisy.
    if lowered_query.chars().count() <= 2 {
        return None;
    }

    if term.contains(lowered_query)
        || (!normalized_query.is_empty() && normalized_term.contains(normalized_query))
    {
        Some(3)
    } else {
        None
    }
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
