//! Deterministic, read-only advice. Source text is embedded at compile time.

use crate::hardware::GpuProfile;
use crate::observations::GameObservation;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const RULES: &str = include_str!("../data/advice-rules.json");
const COMPATIBILITY_WIKI: &str = "https://github.com/optiscaler/OptiScaler/wiki/Compatibility-List";
static RULE_BOOK: OnceLock<Option<RuleBook>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdviceStatus {
    Documented,
    Conditional,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub id: String,
    pub title: String,
    pub status: AdviceStatus,
    pub explanation: String,
    pub source_url: String,
    pub checked_at: String,
    pub version_scope: String,
}

#[derive(Deserialize)]
struct RuleBook {
    rules_version: u32,
    checked_at: String,
    general: Vec<Rule>,
    games: Vec<Rule>,
}

#[derive(Deserialize)]
struct Rule {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    steam_ids: Vec<String>,
    #[serde(default)]
    tested_version: Option<String>,
    status: AdviceStatus,
    source_url: String,
    version_scope: String,
    title: Localized,
    explanation: Localized,
}

#[derive(Deserialize)]
struct Localized {
    en: String,
    da: String,
    pl: String,
}

impl Localized {
    fn get(&self, language: &str) -> &str {
        match language.split(['-', '_']).next().unwrap_or("") {
            "da" => &self.da,
            "pl" => &self.pl,
            _ => &self.en,
        }
    }
}

impl Rule {
    fn recommendation(
        &self,
        language: &str,
        checked_at: &str,
        status: AdviceStatus,
    ) -> Recommendation {
        Recommendation {
            id: self.id.clone(),
            title: self.title.get(language).to_string(),
            status,
            explanation: self.explanation.get(language).to_string(),
            source_url: self.source_url.clone(),
            checked_at: checked_at.to_string(),
            version_scope: self.version_scope.clone(),
        }
    }

    fn matches(&self, game: &GameObservation) -> bool {
        let name = normalized(&game.name);
        if game.platform.eq_ignore_ascii_case("steam")
            && game
                .store_id
                .as_ref()
                .is_some_and(|id| !self.steam_ids.is_empty() && !self.steam_ids.contains(id))
        {
            return false;
        }
        self.aliases.iter().any(|alias| normalized(alias) == name)
            || (game.platform.eq_ignore_ascii_case("steam")
                && game
                    .store_id
                    .as_ref()
                    .is_some_and(|id| self.steam_ids.contains(id)))
    }
}

fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn unknown(id: &str, language: &str, checked_at: &str) -> Recommendation {
    let (title, explanation) = match (id, language.split(['-', '_']).next().unwrap_or("")) {
        ("game-unknown", "da") => ("Ukendt spil", "Der findes ingen spilspecifik regel. Brug kun de generelle hardware-råd; kompatibilitet er ikke bekræftet."),
        ("game-unknown", "pl") => ("Nieznana gra", "Brak reguły dla tej gry. Stosuj tylko ogólne porady sprzętowe; zgodność nie jest potwierdzona."),
        ("game-unknown", _) => ("Game support unknown", "No game-specific rule is available. Use only general hardware guidance; compatibility is not established."),
        (_, "da") => ("Ukendt rådgrundlag", "Reglerne mangler eller modsiger hinanden. Ingen spilspecifik kompatibilitet kan udledes."),
        (_, "pl") => ("Nieznana podstawa porady", "Reguł brakuje lub są sprzeczne. Nie można potwierdzić zgodności konkretnej gry."),
        _ => ("Advice evidence unknown", "Rules are missing or contradictory. No game-specific compatibility can be inferred."),
    };
    Recommendation {
        id: id.into(),
        title: title.into(),
        status: AdviceStatus::Unknown,
        explanation: explanation.into(),
        source_url: COMPATIBILITY_WIKI.into(),
        checked_at: checked_at.into(),
        version_scope: "unverified".into(),
    }
}

fn general_status(rule: &Rule, gpu: Option<&GpuProfile>) -> AdviceStatus {
    if rule.id != "dlss-sr-output" {
        return AdviceStatus::Conditional;
    }
    let Some(gpu) = gpu else {
        return AdviceStatus::Unknown;
    };
    let vendor = gpu.vendor.as_deref().unwrap_or("").to_ascii_lowercase();
    let name = gpu.name.as_deref().unwrap_or("").to_ascii_lowercase();
    if vendor.contains("amd") || vendor.contains("intel") {
        AdviceStatus::Unsupported
    } else if vendor.contains("nvidia") {
        if name.contains("rtx") {
            AdviceStatus::Conditional
        } else if name.contains("gtx") {
            AdviceStatus::Unsupported
        } else {
            AdviceStatus::Unknown
        }
    } else {
        AdviceStatus::Unknown
    }
}

/// Evaluate recorded facts; no hardware probe, game file access, mutation or network request.
pub fn evaluate(
    gpu: Option<&GpuProfile>,
    game: &GameObservation,
    language: &str,
) -> Vec<Recommendation> {
    let Some(book) = RULE_BOOK
        .get_or_init(|| serde_json::from_str::<RuleBook>(RULES).ok())
        .as_ref()
    else {
        return vec![unknown("rules-invalid", language, "unknown")];
    };
    evaluate_book(book, gpu, game, language)
}

/// Evaluate a supplied rule fixture without changing the embedded production rule book.
pub fn evaluate_rules(
    rules_json: &str,
    gpu: Option<&GpuProfile>,
    game: &GameObservation,
    language: &str,
) -> Vec<Recommendation> {
    let Ok(book) = serde_json::from_str::<RuleBook>(rules_json) else {
        return vec![unknown("rules-invalid", language, "unknown")];
    };
    evaluate_book(&book, gpu, game, language)
}

fn evaluate_book(
    book: &RuleBook,
    gpu: Option<&GpuProfile>,
    game: &GameObservation,
    language: &str,
) -> Vec<Recommendation> {
    if book.rules_version != 1 {
        return vec![unknown("rules-version-unknown", language, &book.checked_at)];
    }
    let mut result: Vec<_> = book
        .general
        .iter()
        .map(|rule| rule.recommendation(language, &book.checked_at, general_status(rule, gpu)))
        .collect();
    let matches: Vec<_> = book
        .games
        .iter()
        .filter(|rule| rule.matches(game))
        .collect();
    match matches.as_slice() {
        [] => result.push(unknown("game-unknown", language, &book.checked_at)),
        [rule] => {
            // A wiki test does not establish this device's game build, GPU or runtime result.
            let status = if rule.status == AdviceStatus::Documented {
                AdviceStatus::Conditional
            } else {
                rule.status
            };
            let mut recommendation = rule.recommendation(language, &book.checked_at, status);
            if let Some(tested) = &rule.tested_version {
                let same_version = game.optiscaler_version.as_ref().is_some_and(|actual| {
                    actual
                        .trim_start_matches(['v', 'V'])
                        .eq_ignore_ascii_case(tested)
                });
                if !same_version {
                    let caveat = match language.split(['-', '_']).next().unwrap_or("") {
                        "da" => " Den lokale OptiScaler-version matcher ikke den dokumenterede test eller er ukendt.",
                        "pl" => " Lokalna wersja OptiScaler nie odpowiada testowanej wersji lub jest nieznana.",
                        _ => " The local OptiScaler version differs from the documented test or is unknown.",
                    };
                    recommendation.explanation.push_str(caveat);
                }
            }
            result.push(recommendation);
        }
        _ => result.push(unknown("game-rules-conflict", language, &book.checked_at)),
    }
    result
}
