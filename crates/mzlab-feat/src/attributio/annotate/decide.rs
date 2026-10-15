// Ported from Attributio annotate.py.
//! The chemist's decisions on single features (`feature_decisions.csv`): `nome` replaces the name, `livello` 3 takes a Level 2 to 3 (`expert review`) and `2b`
//! raises a Level 3 or moves a Level 2a to 2b (`diagnostic evidence`), the only way a feature gets 2b. Also `level_sub` and the `mzlib-only` flag.

use super::curate::Curation;
use super::settings::AnnotationSettings;
use super::types::Feature;
use crate::attributio::campaign::Campaign;
use crate::attributio::readers::feature_decisions::{match_decisions, DecisionTable, FeatureKey};

/// `a` / `b` for the Level 2 features (b = the chemist's 2b decision, also when it counts as Level 3 because `count_2b_as_2 = false`), empty otherwise.
pub fn level_sub(f: &Feature) -> &'static str {
    if f.decision.decision_2b && (f.level == 2 || f.level == 3) {
        "b"
    } else if f.level == 2 {
        "a"
    } else {
        ""
    }
}

/// `mzlib-only` for a Level 2 whose only library source above its gates is mzMine.
pub fn mzlib_only(f: &Feature) -> Option<String> {
    let only = f.sources.contains(&"mzMine-library")
        && !f.sources.contains(&"SIRIUS-spectral")
        && !f.sources.contains(&"GNPS2")
        && !f.decision.decision_2b;
    (f.level == 2 && only).then(|| "mzlib-only".to_string())
}

/// The feature keys of every row of the merged tables: a decision is looked up among all of them, not only the loaded sites.
pub fn feature_keys(feats: &[Feature]) -> Vec<FeatureKey> {
    feats
        .iter()
        .map(|f| FeatureKey {
            pol: f.pol.clone(),
            id: f.id,
            mz: f.mz,
            rt: f.rt,
        })
        .collect()
}

type Before = (i32, String, String);

fn undo_2b(f: &mut Feature, before: &mut Vec<(usize, Before)>, i: usize) {
    if let Some(n) = before.iter().position(|(k, _)| *k == i) {
        let (_, (level, basis, label)) = before.remove(n);
        f.level = level;
        f.level_basis = basis;
        f.level_label = label;
    }
    f.decision.decision_2b = false;
    f.decision.evidence.clear();
}

/// Applies the decisions in the order of the file; a later row supersedes an earlier 2b.
pub fn apply_decisions(
    feats: &mut [Feature],
    table: &DecisionTable,
    every: &[FeatureKey],
    c: &Campaign,
    s: &AnnotationSettings,
    cur: &dyn Curation,
    warnings: &mut Vec<String>,
) {
    let keys = feature_keys(feats);
    let matched = if table.is_empty() {
        Vec::new()
    } else {
        match_decisions(&keys, table, Some(every), warnings)
    };
    let mut before: Vec<(usize, Before)> = Vec::new();
    for rec in matched {
        let i = rec.row;
        let f = &mut feats[i];
        if !rec.group.is_empty() {
            f.decision.group_override = Some(rec.group.clone());
        }
        if !rec.nome.is_empty() {
            f.disp_name = Some(rec.nome.clone());
            f.cec = cur.match_cec(&[Some(&rec.nome)]);
        }
        let l = rec.livello.as_str();
        if (l == "2" || l == "3") && f.decision.decision_2b {
            undo_2b(f, &mut before, i);
        }
        if l == "3" && f.level == 2 {
            f.level = 3;
            f.level_basis = "expert review".into();
            f.level_label = s.label("expert review");
        }
        if l == "2b" {
            if f.level == 2 || f.level == 3 {
                if !f.decision.decision_2b {
                    before.push((i, (f.level, f.level_basis.clone(), f.level_label.clone())));
                }
                f.level = if c.count_2b_as_2 { 2 } else { 3 };
                f.level_basis = "diagnostic evidence".into();
                f.level_label = s.label("diagnostic evidence")
                    + if c.count_2b_as_2 {
                        ""
                    } else {
                        " -- counted as Level 3 (campaign.toml [levels] count_2b_as_2 = false)"
                    };
                f.decision.decision_2b = true;
                f.decision.evidence = rec
                    .evidence
                    .iter()
                    .filter(|(_, v)| !v.is_empty())
                    .cloned()
                    .collect();
            } else {
                warnings.push(format!(
                    "[{}] Level 2b decision not applied to {} {}: it is Level {}, 2b is set only from Level 3 or 2a",
                    c.id, f.pol, f.id, f.level
                ));
            }
        }
        f.decision.feature_decision = rec.motivazione.clone();
    }
    for f in feats.iter_mut() {
        f.cec_headline = f.cec.as_ref().is_some_and(|c| cur.is_headline(&c.0));
        f.level_flag = mzlib_only(f);
        f.level_sub = level_sub(f).into();
    }
}
