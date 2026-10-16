// Ported from Attributio diagnostics/publication_table.py.
//! The flags, the hazard cells and the sort metadata of a row of the publication table.

use super::publication_ev::*;
use super::publication_ids::PFeat;
use super::publication_text::*;
use crate::attributio::frame::Cell;
use crate::attributio::rt_logp::Z_FLAG;
use std::collections::BTreeSet;

pub(super) fn word_isomers(low: &str) -> bool {
    let w = |c: char| c.is_alphanumeric() || c == '_';
    low.match_indices("isomers").any(|(i, m)| {
        !low[..i].chars().next_back().is_some_and(w)
            && !low[i + m.len()..].chars().next().is_some_and(w)
    })
}

/// A parenthesis without parentheses inside that holds `\w/\w`.
pub(super) fn slash_group(low: &str) -> bool {
    let w = |c: char| c.is_alphanumeric() || c == '_';
    low.match_indices('(').any(|(i, _)| {
        let body = &low[i + 1..];
        let Some(end) = body
            .find(['(', ')'])
            .filter(|&e| body[e..].starts_with(')'))
        else {
            return false;
        };
        let chars: Vec<char> = body[..end].chars().collect();
        chars.windows(3).any(|t| t[1] == '/' && w(t[0]) && w(t[2]))
    })
}

/// `_SERIES`: a name that groups isomers (a polyether series by formula, `isomers`, a curated union such as `(sucrose/palatinose)`).
pub(super) fn is_series(nome: &str) -> bool {
    let low = nome.to_lowercase();
    low.starts_with("peg/polyether [") || word_isomers(&low) || slash_group(&low)
}

pub(super) struct FlagInput<'a, 'b> {
    pub(super) sub: &'b [&'a PFeat<'a>],
    pub(super) rep: &'b PFeat<'a>,
    pub(super) best_feats: &'b [&'a PFeat<'a>],
    pub(super) nome: &'b str,
    pub(super) lv: i32,
    pub(super) lv_sub: &'b str,
    pub(super) watch: &'b [String],
}

pub(super) fn count_mzlib_only(sub: &[&PFeat]) -> usize {
    sub.iter()
        .filter(|p| p.f().level_flag.as_deref() == Some("mzlib-only"))
        .count()
}

pub(super) fn flags(x: &FlagInput, inferred: bool) -> Vec<String> {
    let (sub, rep, lv) = (x.sub, x.rep, x.lv);
    let basis = rep.f().level_basis.as_str();
    let mut fl: Vec<String> = Vec::new();
    if inferred {
        fl.push(FLAG_ADDUCT.into());
    }
    let n_l2 = sub.iter().filter(|p| p.f().level == 2).count();
    let n_mz = count_mzlib_only(sub);
    if n_mz > 0 {
        let more = if n_mz < n_l2 {
            format!(" ({n_mz} of {n_l2} Level 2a features)")
        } else {
            String::new()
        };
        fl.insert(0, format!("{FLAG_MZLIB_ONLY}{more}"));
    }
    if lv == 3 && basis == "name conflict" {
        fl.insert(0, FLAG_NAME_CONFLICT.into());
    }
    let choices: BTreeSet<&str> = sub
        .iter()
        .filter_map(|p| p.f().name_choice.as_deref())
        .collect();
    if !choices.is_empty() {
        let list: Vec<&str> = choices.into_iter().collect();
        fl.push(format!("{FLAG_LIBRARY_CHOICE} ({})", list.join("; ")));
    }
    if lv == 3 && basis == "expert review" {
        fl.insert(0, FLAG_EXPERT.into());
    }
    if lv == 3 && basis == "RT isomer" {
        fl.insert(0, FLAG_RT_ISOMER.into());
    }
    if x.lv_sub == "b" {
        fl.insert(0, FLAG_2B.into());
    }
    tail_flags(x, &mut fl);
    fl
}

pub(super) fn tail_flags(x: &FlagInput, fl: &mut Vec<String>) {
    let sub = x.sub;
    let blocks: BTreeSet<&str> = sub
        .iter()
        .filter(|p| p.f().decision.feature_decision.is_empty())
        .filter_map(|p| p.ev.name_block.as_deref())
        .collect();
    if is_series(x.nome)
        || blocks.len() > 1
        || sub
            .iter()
            .any(|p| p.f().level_basis == "isomers not discriminated")
    {
        fl.push(FLAG_ISOMERS.into());
    }
    if sub
        .iter()
        .any(|p| p.f().rt_info.rt_groups.is_some_and(|g| g > 1))
    {
        fl.push(FLAG_RT_SPLIT.into());
    }
    if sub.iter().any(|p| p.fr.rt_outlier) {
        fl.push(format!(
            "{FLAG_RT_LOGP} (|z| > {})",
            mzlab_core::json::fmt_g(Z_FLAG)
        ));
    }
    let mut names = vec![x.nome.to_string()];
    names.extend(
        sub.iter()
            .map(|p| p.ev.identity.clone())
            .collect::<BTreeSet<_>>(),
    );
    let names = names.join(" ").to_lowercase();
    if x.watch.iter().any(|w| names.contains(w.as_str())) {
        fl.push(FLAG_WATCHLIST.into());
    }
    if x.lv == 1
        && x.best_feats.iter().any(|p| {
            p.f()
                .std
                .as_ref()
                .is_some_and(|s| s.status == "confirmed_manual")
        })
    {
        fl.push(FLAG_L1_MANUAL.into());
    }
}

// ── the table ────────────────────────────────────────────────────────────

/// What the counts and the sort need of a row.
pub(super) struct Meta {
    pub(super) lv: i32,
    pub(super) anthro: bool,
    pub(super) sub_b: bool,
    pub(super) n_mz: usize,
    pub(super) core: bool,
    pub(super) flags: String,
    pub(super) key: String,
}

pub(super) fn is_isomer_name(name: &str) -> bool {
    let Some(i) = name.rfind(" isomer, RT ") else {
        return false;
    };
    let tail = &name[i + 12..];
    tail.strip_suffix(" min")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '.'))
}

/// The hazard columns of a compound that no registry knows: the fixed record with its caveats.
pub(super) fn hazard_cells(nome: &str, lv: i32) -> Vec<Cell> {
    let mut caveats: Vec<&str> = Vec::new();
    if is_isomer_name(nome) {
        caveats.push("RT isomer: hazard of the named structure, the peak may be another isomer");
    }
    if lv == 3 {
        caveats.push("Level 3: hazard of the CSI:FingerID candidate structure");
    }
    caveats.push("no structure in the registry");
    vec![
        text("Not assessable"),
        text("not assessable"),
        text(""),
        text("Not assessable"),
        text("not assessable"),
        text(""),
        text(caveats.join("; ")),
        text(""),
    ]
}
