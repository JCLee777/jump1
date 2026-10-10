//! Normalized search terms shared by platform discovery and plugins.

use pinyin::ToPinyin;

pub fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

pub fn application_terms<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut terms = Vec::new();
    for name in names {
        push_unique(&mut terms, name.to_lowercase());
        push_unique(&mut terms, normalize(name));

        let mut full = String::new();
        let mut initials = String::new();
        for character in name.chars() {
            if let Some(pinyin) = character.to_pinyin() {
                full.push_str(pinyin.plain());
                initials.push_str(pinyin.first_letter());
            } else if character.is_alphanumeric() {
                for lowered in character.to_lowercase() {
                    full.push(lowered);
                    initials.push(lowered);
                }
            }
        }
        push_unique(&mut terms, full);
        push_unique(&mut terms, initials);
    }
    terms.retain(|term| !term.is_empty());
    terms
}

fn push_unique(terms: &mut Vec<String>, value: String) {
    if !value.is_empty() && !terms.contains(&value) {
        terms.push(value);
    }
}
