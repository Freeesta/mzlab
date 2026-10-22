//! The structure side of the rules of `fragrules/1`: which rules of the table the structure of the user satisfies (required and excluded groups, the
//! named `site`) and the products they expect (`regole_site`). The core (`mzlab_core::msngrafo`) matches those products against the graph; used by
//! `mzlab-cli msn-graph --regole` and the `.wasm` `msngrafo`. Evidence with a reason, never a verdict; `regole_tp/2` does not use it.

use super::grafo::Mol;
use super::regole_site::{GRUPPI_NUOVI, SITI};
use mzlab_core::error::Result;
use mzlab_core::msngrafo::{tabella_regole, Attesa, Opzioni, Regola, Tabella, Tipo};

/// The rules whose structure conditions hold for `smiles`, with the products they expect: `Attesa`s for the option `attese` of the graph.
/// A rule with no structure condition is not listed (the core decides it by the formula alone); a rule that reads a structure but has no product to
/// compute (`remote_fixed_charge`) is not listed either.
pub fn attese(smiles: &str, t: &Tabella) -> Result<Vec<Attesa>> {
    let m = Mol::parse(smiles)?;
    let gruppi = m.groups_ext();
    let mut out = vec![];
    for r in t
        .regole
        .iter()
        .filter(|r| r.legge_struttura() && (r.tipo != Tipo::Cleavage || r.site.is_some()))
    {
        if !soddisfa(&m, &gruppi, r) {
            continue;
        }
        let prodotti = match r.site.as_deref().and_then(|s| m.site(s)) {
            Some(p) => p,
            None => vec![(None, false)],
        };
        for (f, ione) in prodotti {
            let produce = f.is_some() || r.site.as_deref().is_none_or(|s| !prodotto_dal_sito(s));
            if produce {
                out.push(Attesa {
                    regola: r.id.clone(),
                    formula: f.unwrap_or_else(|| r.formula.clone()),
                    ione: ione || r.tipo == Tipo::Ion,
                });
            }
        }
    }
    Ok(out)
}

/// The sites whose products are computed from the structure (a site of these with no product gives nothing, not the formula of the table).
fn prodotto_dal_sito(s: &str) -> bool {
    matches!(
        s,
        "gamma_h_carbonyl" | "cyclohexene_ring" | "benzyl" | "alpha_c_n" | "acyl"
    )
}

fn soddisfa(m: &Mol, gruppi: &[&str], r: &Regola) -> bool {
    let ha = |g: &String| gruppi.contains(&g.as_str());
    r.all_groups.iter().all(ha)
        && (r.any_groups.is_empty() || r.any_groups.iter().any(ha))
        && !r.excludes.iter().any(ha)
        && r.site
            .as_deref()
            .is_none_or(|s| m.site(s).is_some_and(|p| !p.is_empty()))
}

/// Every group and site name that the table uses must be one this module knows (the test of the table).
pub fn nomi_sconosciuti(t: &Tabella) -> Vec<String> {
    let mut v = vec![];
    for r in &t.regole {
        for g in r.all_groups.iter().chain(&r.any_groups).chain(&r.excludes) {
            let noto = GRUPPI_NUOVI.contains(&g.as_str()) || GRUPPI_BASE.contains(&g.as_str());
            if !noto {
                v.push(format!("{}:{g}", r.id));
            }
        }
        if let Some(s) = &r.site {
            if !SITI.contains(&s.as_str()) {
                v.push(format!("{}:{s}", r.id));
            }
        }
    }
    v
}

/// The names of `Mol::groups()` (listed in `docs/agenti/mzfeat-tp.md`).
const GRUPPI_BASE: [&str; 31] = [
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
pub fn con_attese(o: &Opzioni) -> Opzioni {
    let mut o = o.clone();
    if o.regole.is_some() {
        if let Some(Ok(a)) = o.smiles.as_deref().map(|s| attese(s, tabella_regole())) {
            o.attese = a;
        }
    }
    o
}
