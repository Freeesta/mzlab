// Ported from Attributio identity.py (coherence_report).
//! Invariants of an annotated table, independent of any expected number: a logic bug that happens to give the right count still fails them.

use super::annotate::{Curation, Feature};
use super::identity::{identity_table, load_confident_inventory};
use std::collections::BTreeMap;

/// `hard`: invariants that must hold (name, holds); `info`: diagnostics (name, text).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoherenceReport {
    pub hard: Vec<(String, bool)>,
    pub info: Vec<(String, String)>,
}

impl CoherenceReport {
    /// The invariants that are violated.
    pub fn failed(&self) -> Vec<&str> {
        self.hard
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(n, _)| n.as_str())
            .collect()
    }
}

const L3_OTHER_BASES: [&str; 5] = [
    "name conflict",
    "expert review",
    "isomers not discriminated",
    "RT isomer",
    "diagnostic evidence",
];

fn chemist_wrote(f: &Feature) -> bool {
    !f.decision.feature_decision.trim().is_empty()
}

pub fn coherence_report(
    feats: &[Feature],
    sites: &[String],
    cur: &dyn Curation,
) -> CoherenceReport {
    let mut r = CoherenceReport::default();
    let mut hard = |name: &str, ok: bool| r.hard.push((name.to_string(), ok));
    let at = |lv: i32| feats.iter().filter(move |f| f.level == lv);
    let mut by_level: BTreeMap<i32, usize> = BTreeMap::new();
    for f in feats {
        *by_level.entry(f.level).or_default() += 1;
    }
    hard(
        "levels in {0..4}",
        feats.iter().all(|f| (0..=4).contains(&f.level)),
    );
    hard("no missing levels (NaN)", true);
    hard(
        "sum of levels == n features",
        by_level.values().sum::<usize>() == feats.len(),
    );
    hard(
        "every Lvl1 has std_name",
        at(1).all(|f| f.std.as_ref().is_some_and(|s| s.name.is_some())),
    );
    hard(
        "every Lvl1 has a confirmed MS/MS",
        at(1).all(|f| {
            f.std
                .as_ref()
                .is_some_and(|s| s.status == "confirmed" || s.status == "confirmed_manual")
        }),
    );
    hard(
        "every Lvl3 has a CSI name or a reference standard",
        at(3).all(|f| {
            f.name().is_some()
                || f.std.as_ref().is_some_and(|s| s.name.is_some())
                || L3_OTHER_BASES.contains(&f.level_basis.as_str())
        }),
    );
    let is2b = |f: &&Feature| f.level_sub == "b";
    hard(
        "every Lvl2b comes from a decision of the chemist",
        feats.iter().filter(is2b).all(chemist_wrote),
    );
    hard(
        "every Lvl2 has level_sub a or b",
        at(2).all(|f| f.level_sub == "a" || f.level_sub == "b"),
    );
    let mut info = |name: &str, text: String| r.info.push((name.to_string(), text));
    let counts: Vec<String> = by_level.iter().map(|(k, v)| format!("{k}: {v}")).collect();
    info("features per level", format!("{{{}}}", counts.join(", ")));
    info(
        "features Lvl2b (chemist's decisions)",
        feats.iter().filter(is2b).count().to_string(),
    );
    let only_mzlib = at(2)
        .filter(|f| !f.sources.is_empty() && f.sources.iter().all(|s| *s == "mzMine-library"))
        .count();
    info(
        "Lvl2a mzMine-library only (uncorroborated)",
        only_mzlib.to_string(),
    );
    let mut status: BTreeMap<&str, usize> = BTreeMap::new();
    for s in feats.iter().filter_map(|f| f.std.as_ref()) {
        *status.entry(s.status.as_str()).or_default() += 1;
    }
    let st: Vec<String> = status.iter().map(|(k, v)| format!("'{k}': {v}")).collect();
    info(
        "reference-standard candidates by status",
        format!("{{{}}}", st.join(", ")),
    );
    let (it, inv) = (
        identity_table(feats, sites, cur, &[]).len(),
        load_confident_inventory(feats, sites, cur, &[1, 2, 3]).len(),
    );
    info(
        "identities: identity_table vs inventory",
        format!("({it}, {inv})"),
    );
    r
}
