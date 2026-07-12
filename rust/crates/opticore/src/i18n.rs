//! Translations: the canonical en/da/pl JSON files from the Python app are
//! embedded at compile time, flattened to dot-path maps ("ui.app_title").
//! Lookup: current language → English fallback → "[key]" marker.

use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Da,
    Pl,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Da => "da",
            Lang::Pl => "pl",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Da => "Dansk",
            Lang::Pl => "Polski",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.to_lowercase().as_str() {
            "da" => Lang::Da,
            "pl" => Lang::Pl,
            _ => Lang::En,
        }
    }

    pub const ALL: [Lang; 3] = [Lang::En, Lang::Da, Lang::Pl];
}

fn flatten(prefix: &str, value: &Value, out: &mut HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, sub) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(&path, sub, out);
            }
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

fn load(raw: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        flatten("", &value, &mut map);
    }
    map
}

pub struct Translator {
    pub lang: Lang,
    en: HashMap<String, String>,
    da: HashMap<String, String>,
    pl: HashMap<String, String>,
}

impl Default for Translator {
    fn default() -> Self {
        Self::new(Lang::En)
    }
}

impl Translator {
    pub fn new(lang: Lang) -> Self {
        Self {
            lang,
            en: load(include_str!("../../../../src/translations/en.json")),
            da: load(include_str!("../../../../src/translations/da.json")),
            pl: load(include_str!("../../../../src/translations/pl.json")),
        }
    }

    fn map(&self, lang: Lang) -> &HashMap<String, String> {
        match lang {
            Lang::En => &self.en,
            Lang::Da => &self.da,
            Lang::Pl => &self.pl,
        }
    }

    /// Translate a dot-path key with English fallback and a "[key]" marker
    /// for missing entries (same behavior as the Python `t()` helper).
    pub fn tr(&self, key: &str) -> String {
        self.map(self.lang)
            .get(key)
            .or_else(|| self.en.get(key))
            .cloned()
            .unwrap_or_else(|| format!("[{key}]"))
    }

    /// Keys present in a language but absent from English (should be none).
    pub fn extra_keys(&self, lang: Lang) -> Vec<String> {
        self.map(lang)
            .keys()
            .filter(|k| !self.en.contains_key(*k))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_with_fallback_and_marker() {
        let mut t = Translator::new(Lang::Da);
        assert_eq!(t.tr("ui.games_tab"), "Spil");
        t.lang = Lang::En;
        assert_eq!(t.tr("ui.games_tab"), "Games");
        // missing key → marker
        assert_eq!(t.tr("ui.definitely_not_a_key"), "[ui.definitely_not_a_key]");
    }

    #[test]
    fn no_extra_keys_outside_english() {
        let t = Translator::new(Lang::En);
        assert_eq!(t.extra_keys(Lang::Da), Vec::<String>::new());
        assert_eq!(t.extra_keys(Lang::Pl), Vec::<String>::new());
    }

    #[test]
    fn coverage_report() {
        // Missing keys warn (printed) but don't fail — mirrors the Python
        // discipline where en is canonical and translations may lag.
        let t = Translator::new(Lang::En);
        for lang in [Lang::Da, Lang::Pl] {
            let missing: Vec<&String> =
                t.en.keys()
                    .filter(|k| !t.map(lang).contains_key(*k))
                    .collect();
            if !missing.is_empty() {
                println!(
                    "{} missing {} keys: {missing:?}",
                    lang.code(),
                    missing.len()
                );
            }
        }
    }
}
