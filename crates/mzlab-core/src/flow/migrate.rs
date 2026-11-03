//! Step migrations: a plan, a `--set` or an `mzrun/1` preset that names an older version of a step (or the old names of its parameters) is rewritten to
//! the current version before it is checked or run, so saved plans keep running and only the current names are printed. Applied by `parse` (and so by
//! `check`, `run`, `line`, `command` and the scheduler), by `apply_set` and by `mzrun::migrate`.

use super::Plan;
use crate::json::Value;

/// The parameters of an instance, in the order written.
pub type Params = Vec<(String, Value)>;

/// One migration: `step@from` becomes `step@to` and its parameters go through `apply`. `apply` must be idempotent, because an instance already at `to`
/// that still carries old names (a hand edit, an older page) goes through it too.
pub struct Migration {
    pub step: &'static str,
    pub from: u32,
    pub to: u32,
    pub apply: fn(Params) -> Params,
}

/// `spectra.msn_graph@1 → @2`: the Italian parameters take the English names of the `msn-graph` flags.
const MSN_GRAPH_1_2: &[(&str, &str)] = &[
    ("mz_basso", "low_mz"),
    ("ppm_basso", "wide_ppm"),
    ("prove_min", "min_evidence"),
    ("attivazione", "activation"),
    ("isotopi", "isotopes"),
    ("strutture_nodo", "structures"),
    ("ppm_struttura", "struct_ppm"),
    ("interpreta", "interpret"),
    ("ppm_perdita", "loss_ppm"),
    ("regole", "rules"),
    ("toll_perdita_mda", "loss_tol_mda"),
    ("scelte", "pick"),
    ("rotture", "breaks"),
    ("max_legami", "max_bonds"),
];

/// Renames the keys of `table`; when the new name is given too, the new one wins and the old one is dropped.
fn rename(p: Params, table: &[(&str, &str)]) -> Params {
    let new_of = |k: &str| table.iter().find(|(old, _)| *old == k).map(|(_, new)| *new);
    let mut out: Params = Vec::with_capacity(p.len());
    let given: Vec<String> = p.iter().map(|(k, _)| k.clone()).collect();
    for (k, v) in p {
        match new_of(&k) {
            Some(new) if given.iter().any(|g| g == new) => {}
            Some(new) => out.push((new.to_string(), v)),
            None => out.push((k, v)),
        }
    }
    out
}

fn msn_graph_1_2(p: Params) -> Params {
    rename(p, MSN_GRAPH_1_2)
}

/// Every migration of the engine, one row per step and version.
pub const MIGRATIONS: &[Migration] = &[Migration {
    step: "spectra.msn_graph",
    from: 1,
    to: 2,
    apply: msn_graph_1_2,
}];

/// What a migration changed in one instance: the versions and each parameter renamed (old, new).
#[derive(Debug, Clone, PartialEq)]
pub struct Migrated {
    pub from: u32,
    pub to: u32,
    pub renamed: Vec<(String, String)>,
}

/// The newest version `step@version` migrates to, its parameters and what changed; `None` when no migration applies and nothing was renamed.
pub fn migrate_params(
    step: &str,
    version: u32,
    params: &Params,
) -> Option<(u32, Params, Migrated)> {
    let mut v = version;
    let mut p = params.clone();
    let mut any = false;
    while let Some(m) = MIGRATIONS.iter().find(|m| m.step == step && m.from == v) {
        p = (m.apply)(p);
        v = m.to;
        any = true;
    }
    // an instance already at the newest version with old names left in it
    if let Some(m) = MIGRATIONS.iter().find(|m| m.step == step && m.to == v) {
        let again = (m.apply)(p.clone());
        any |= again != p;
        p = again;
    }
    if !any {
        return None;
    }
    // a dropped old name (its new name was given too) is not a rename
    let renamed = params
        .iter()
        .filter_map(|(k, _)| Some((k.clone(), migrate_name(step, version, k)?)))
        .filter(|(_, new)| !params.iter().any(|(g, _)| g == new))
        .collect();
    Some((
        v,
        p,
        Migrated {
            from: version,
            to: v,
            renamed,
        },
    ))
}

/// The current name of the parameter `name` of `step@version` (`--set g.mz_basso=` names `low_mz`); `None` when it is not renamed.
pub fn migrate_name(step: &str, version: u32, name: &str) -> Option<String> {
    let one = vec![(name.to_string(), Value::Null)];
    let mut v = version;
    let mut p = one.clone();
    while let Some(m) = MIGRATIONS.iter().find(|m| m.step == step && m.from == v) {
        p = (m.apply)(p);
        v = m.to;
    }
    if let Some(m) = MIGRATIONS.iter().find(|m| m.step == step && m.to == v) {
        p = (m.apply)(p);
    }
    let new = p.into_iter().next()?.0;
    (new != name).then_some(new)
}

/// Migrates every instance of `plan` in place; returns (instance, what changed) for each instance that changed.
pub fn migrate(plan: &mut Plan) -> Vec<(String, Migrated)> {
    let mut out = Vec::new();
    for inst in &mut plan.steps {
        if let Some((v, p, m)) = migrate_params(&inst.step, inst.version, &inst.params) {
            inst.version = v;
            inst.params = p;
            out.push((inst.id.clone(), m));
        }
    }
    out
}

#[cfg(test)]
#[path = "migrate_tests.rs"]
mod tests;
