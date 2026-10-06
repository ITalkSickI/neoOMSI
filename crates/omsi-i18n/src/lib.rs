mod legacy;

use hashbrown::HashMap;
use std::borrow::Cow;
use std::sync::{OnceLock, RwLock};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

const DEFAULT: &str = "en";

pub type Lookup = fn(&str, &str) -> Option<String>;

type Table = HashMap<String, String>;

static TABLES: OnceLock<HashMap<String, Table>> = OnceLock::new();
static LANGUAGE: RwLock<String> = RwLock::new(String::new());
static FALLBACK: RwLock<Option<Lookup>> = RwLock::new(None);

fn flatten(prefix: &str, value: &serde_json::Value, out: &mut Table) {
    match value {
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        _ => {}
    }
}

fn tables() -> &'static HashMap<String, Table> {
    TABLES.get_or_init(|| {
        let mut all = HashMap::new();
        for (code, json) in LOCALES {
            let mut table = Table::new();
            match serde_json::from_str::<serde_json::Value>(json) {
                Ok(v) => flatten("", &v, &mut table),
                Err(e) => eprintln!("i18n: {code}.json is not valid: {e}"),
            }
            all.insert(code.to_ascii_lowercase(), table);
        }
        all
    })
}

fn find_modern(lang: &str, key: &str) -> Option<&'static str> {
    tables()
        .get(&lang.to_ascii_lowercase())
        .and_then(|t| t.get(key))
        .map(String::as_str)
}

fn find(lang: &str, key: &str) -> Option<&'static str> {
    find_modern(lang, key).or_else(|| legacy::lookup(lang, key))
}

pub fn set_fallback(f: Option<Lookup>) {
    if let Ok(mut s) = FALLBACK.write() {
        *s = f;
    }
}

pub fn set_language(code: &str) {
    if let Ok(mut s) = LANGUAGE.write() {
        *s = if code.eq_ignore_ascii_case(DEFAULT) {
            String::new()
        } else {
            code.to_ascii_lowercase()
        };
    }
}

pub fn language() -> String {
    LANGUAGE.read().map(|s| s.clone()).unwrap_or_default()
}

pub fn languages() -> Vec<&'static str> {
    LOCALES.iter().map(|(c, _)| *c).collect()
}

pub fn keys() -> Vec<String> {
    let mut all: Vec<String> = tables()
        .values()
        .flat_map(|t| t.keys().cloned())
        .collect();
    all.sort();
    all.dedup();
    all
}

pub fn lookup(lang: &str, key: &str) -> Option<String> {
    find(lang, key).map(str::to_string)
}

pub fn tr(key: &str) -> Cow<'_, str> {
    if key.is_empty() {
        return Cow::Borrowed(key);
    }
    let lang = language();
    if !lang.is_empty() {
        if let Some(t) = legacy::lookup(&lang, key) {
            return Cow::Borrowed(t);
        }
        if let Some(t) = FALLBACK
            .read()
            .ok()
            .and_then(|g| *g)
            .and_then(|g| g(&lang, key))
        {
            return Cow::Owned(t);
        }
    }
    match legacy::lookup(DEFAULT, key) {
        Some(t) => Cow::Borrowed(t),
        None => Cow::Borrowed(key),
    }
}

/// We will only translate text when we use this, the legacy way is shit.
pub fn translate(key: &str, params: &[(&str, &dyn std::fmt::Display)]) -> String {
    let lang = language();
    let mut text = if lang.is_empty() {
        None
    } else {
        find_modern(&lang, key)
    }
        .or_else(|| find_modern(DEFAULT, key))
        .unwrap_or(key)
        .to_string();
    for (name, value) in params {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_nested_keys() {
        let mut t = Table::new();
        let v: serde_json::Value = serde_json::from_str(r#"{"ui":{"menu":{"title":"Menu"}}}"#).unwrap();
        flatten("", &v, &mut t);
        assert_eq!(t["ui.menu.title"], "Menu");
    }

    #[test]
    fn unknown_key_is_returned_as_it_is() {
        assert_eq!(tr("no.such.key"), "no.such.key");
    }
}