//! The structure side of the rules of `fragrules/1`: which rules of the table the structure of the user satisfies (required and excluded groups, the
//! named `site`) and the products they expect (`regole_site`). The core (`mzlab_core::msngrafo`) matches those products against the graph; used by
//! `mzlab-cli msn-graph --regole` and the `.wasm` `msngrafo`. Evidence with a reason, never a verdict; `regole_tp/2` does not use it.

use super::grafo::Mol;
use super::regole_site::{NEW_GROUPS, SITES};
use mzlab_core::error::Result;
use mzlab_core::msngrafo::{tabella_regole, Expectation, Options, Rule, Table, Tipo};

/// The rules whose structure conditions hold for `smiles`, with the products they expect: `Expectation`s for the option `attese` of the graph.
/// A rule with no structure condition is not listed (the core decides it by the formula alone); a rule that reads a structure but has no product to
/// compute (`remote_fixed_charge`) is not listed either.
pub fn attese(smiles: &str, t: &Table) -> Result<Vec<Expectation>> {
    Ok(attese_mol(&Mol::parse(smiles)?, t))
}

/// `attese` of a structure already read (the decoys of `mzlab-cli msn-rules` are built as graphs).
pub fn attese_mol(m: &Mol, t: &Table) -> Vec<Expectation> {
    let groups = m.groups_ext();
    let mut out = vec![];
    for r in t
        .rules
        .iter()
        .filter(|r| r.structure_law() && (r.tipo != Tipo::Cleavage || r.site.is_some()))
    {
        if !soddisfa(m, &groups, r) {
            continue;
        }
        let products = match r.site.as_deref().and_then(|s| m.site(s)) {
            Some(p) => p,
            None => vec![(None, false)],
        };
        for (f, ione) in products {
            let produce = f.is_some() || r.site.as_deref().is_none_or(|s| !product_of_site(s));
            if produce {
                out.push(Expectation {
                    rule: r.id.clone(),
                    formula: f.unwrap_or_else(|| r.formula.clone()),
                    ione: ione || r.tipo == Tipo::Ion,
                });
            }
        }
    }
    out
}

/// The sites whose products are computed from the structure (a site of these with no product gives nothing, not the formula of the table).
fn product_of_site(s: &str) -> bool {
    matches!(
        s,
        "gamma_h_carbonyl" | "cyclohexene_ring" | "benzyl" | "alpha_c_n" | "acyl"
    )
}

fn soddisfa(m: &Mol, groups: &[&str], r: &Rule) -> bool {
    let ha = |g: &String| groups.contains(&g.as_str());
    r.all_groups.iter().all(ha)
        && (r.any_groups.is_empty() || r.any_groups.iter().any(ha))
        && !r.excludes.iter().any(ha)
        && r.site
            .as_deref()
            .is_none_or(|s| m.site(s).is_some_and(|p| !p.is_empty()))
}

/// Every group and site name that the table uses must be one this module knows (the test of the table).
pub fn unknown_names(t: &Table) -> Vec<String> {
    let mut v = vec![];
    for r in &t.rules {
        for g in r.all_groups.iter().chain(&r.any_groups).chain(&r.excludes) {
            let known = NEW_GROUPS.contains(&g.as_str()) || BASE_GROUPS.contains(&g.as_str());
            if !known {
                v.push(format!("{}:{g}", r.id));
            }
        }
        if let Some(s) = &r.site
            && !SITES.contains(&s.as_str())
        {
            v.push(format!("{}:{s}", r.id));
        }
    }
    v
}

/// The names of `Mol::groups()` (listed in `docs/agenti/mzfeat-tp.md`).
const BASE_GROUPS: [&str; 31] = [
    "halogen",
    "fluoro",
    "chloro",
    "bromo",
    "iodo",
    "aromatic_ring",
    "carboxylic_acid",
    "ester",
    "amide",
    "aldehyde",
    "ketone",
    "alpha_ch_hetero",
    "nitro",
    "amine_primary",
    "amine_secondary",
    "amine_tertiary",
    "amine_aromatic",
    "n_alkyl",
    "n_methyl",
    "n_isopropyl",
    "n_tbutyl",
    "sulfonamide",
    "heteroaromatic",
    "phenol",
    "alcohol",
    "ether",
    "o_alkyl",
    "sulfonyl",
    "sulfoxide",
    "sulfide",
    "ring_hetero_saturated",
];

/// `o` with the expectations of its structure (`o.smiles`) when the rules are on; the options of a graph without rules or without a SMILES (or with one
/// that does not parse: the page and the command report that where they read it) come back unchanged.
pub fn with_expected(o: &Options) -> Options {
    let mut o = o.clone();
    if o.regole.is_some()
        && let Some(Ok(a)) = o.smiles.as_deref().map(|s| attese(s, tabella_regole()))
    {
        o.attese = a;
    }
    o
}
