// Ported from Attributio src/attributio/review/rules.py.
//! The decision rules of the Banco 2b: how the level of a feature is shown, what a decision changes and when a note is required. A note (`motivazione`) is
//! required only when the decision CHANGES the assignment the cascade gives the feature now (the level, the name or the group); keeping things as they are is
//! saved without a note. The page and the saving of the decisions both ask these functions.

/// The level of `feature_decisions.csv` (`livello`) as the page shows it; `None` for no level decision.
pub fn livello_to_level(livello: &str) -> Option<&'static str> {
    match livello.trim() {
        "2b" => Some("2b"),
        "3" | "3a" | "3b" => Some("3"),
        "2" => Some("2a"),
        _ => None,
    }
}

/// `1`, `2a`, `2b` or `3` as the page shows the feature. A 2b counted as 3 (`count_2b_as_2 = false`) stays `2b`; a `3b` (a generic class, also `level_sub` b) is a 3.
pub fn page_level(level: i32, level_sub: &str, decision_2b: bool) -> String {
    match level {
        2 if level_sub == "b" => "2b".to_string(),
        2 => "2a".to_string(),
        3 if level_sub == "b" && decision_2b => "2b".to_string(),
        l => l.to_string(),
    }
}

/// What the cascade gives the feature now.
#[derive(Clone, Debug, Default)]
pub struct Current<'a> {
    pub level: i32,
    pub level_sub: &'a str,
    /// The chemist's 2b decision is in force (`Feature::decision.decision_2b`).
    pub decision_2b: bool,
    pub name: Option<&'a str>,
    pub group: Option<&'a str>,
}

/// The fields of a decision that the rules read.
#[derive(Clone, Debug, Default)]
pub struct Rec<'a> {
    pub livello: &'a str,
    pub group: &'a str,
    pub nome: &'a str,
}

/// What the decision `rec` changes for `feature`; empty: nothing changes and the note is optional.
pub fn decision_changes(feature: &Current, rec: &Rec) -> Vec<String> {
    let cur = page_level(feature.level, feature.level_sub, feature.decision_2b);
    let mut out = Vec::new();
    if let Some(to) = livello_to_level(rec.livello).filter(|to| *to != cur) {
        out.push(format!("level {cur} -> {to}"));
    }
    if rec.group.trim() == "ambiguous" && feature.group.unwrap_or("") != "ambiguous" {
        out.push("identity rejected".to_string());
    }
    let nome = rec.nome.trim();
    if !nome.is_empty() && nome != feature.name.unwrap_or("") {
        out.push(format!("name -> {nome}"));
    }
    out
}

/// A note is required when the decision changes something.
pub fn note_required(feature: &Current, rec: &Rec) -> bool {
    !decision_changes(feature, rec).is_empty()
}
