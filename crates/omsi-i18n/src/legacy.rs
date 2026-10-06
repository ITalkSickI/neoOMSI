use hashbrown::HashMap;
use std::sync::OnceLock;

const SOURCE: &str = include_str!("../../omsi-app/locales/app.yml");

type Table = HashMap<String, HashMap<String, String>>;

static TABLE: OnceLock<Table> = OnceLock::new();

fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.strip_suffix('"').unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

fn parse(source: &str) -> Table {
    let mut table = Table::new();
    let mut key: Option<String> = None;
    for line in source.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() || line.starts_with('#') || line.starts_with("_version") {
            continue;
        }
        if line.starts_with('"') {
            key = line.strip_suffix(':').map(unquote);
        } else if line.starts_with(' ') {
            let (Some(key), Some((lang, text))) = (&key, line.trim_start().split_once(": ")) else {
                continue;
            };
            table
                .entry(lang.to_ascii_lowercase())
                .or_default()
                .insert(key.clone(), unquote(text));
        }
    }
    table
}

pub fn lookup(lang: &str, key: &str) -> Option<&'static str> {
    TABLE
        .get_or_init(|| parse(SOURCE))
        .get(&lang.to_ascii_lowercase())
        .and_then(|t| t.get(key))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_app_yml() {
        let t = parse("_version: 2\n\n\"Start test\":\n  de: \"Test starten\"\n");
        assert_eq!(t["de"]["Start test"], "Test starten");
        assert!(lookup("de", "Start test").is_some());
    }
}