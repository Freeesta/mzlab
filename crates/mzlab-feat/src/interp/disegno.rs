//! The drawing of the TP (`drawing` of `interp/1`): ONE form per row, the simplest that tells the truth, never a structure per site. `structure` only when
//! the position is `site`; `multicenter` when every candidate lies in one aromatic ring; otherwise `bracket` around a zone or the whole parent. The page
//! draws it over the coordinates of the parent that Ketcher gives; without them `without_molfile` turns `structure` into `bracket`.

use super::sito::SiteReport;
use crate::chem::grafo::Mol;
use crate::legacy::fragtree::Graph;

#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub atoms: Vec<usize>,
    pub support: f64,
    pub class: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// `structure`, `multicenter` or `bracket`.
    pub mode: &'static str,
    /// Atoms of the structure change, of the bracket or of the ring.
    pub atoms: Vec<usize>,
    /// The candidate atoms a multicenter bond ends on.
    pub ends: Vec<usize>,
    /// Atoms drawn faint and dashed: the ones a loss removes.
    pub lost: Vec<usize>,
    /// Element of the atom the structure adds on the outer bisector of the site (None for a loss or a dehydrogenation).
    pub added: Option<&'static str>,
    /// Atoms of the zone the fragments support most (empty when none).
    pub highlight: Vec<usize>,
    /// The candidate classes with their support, for `uncertain` only.
    pub zones: Vec<Zone>,
    /// For a loss, the atoms that are kept (the rest is drawn faint).
    pub kept: Vec<usize>,
    pub kind: Option<String>,
    pub delta: String,
    pub label: String,
    /// `none` (position known), `zone` or `whole`.
    pub uncertainty: &'static str,
    pub fallback: Option<&'static str>,
}

/// The label of a transformation with the true minus sign (U+2212).
pub fn delta_label(transform: &str) -> String {
    transform.replace('-', "\u{2212}")
}

fn added_of(kind: &str) -> Option<&'static str> {
    match kind {
        "aromatic_cl" => Some("Cl"),
        "aromatic_br" => Some("Br"),
        "demethyl" | "deethyl" | "dehydro" => None,
        _ => Some("O"),
    }
}

fn whole(mol: &Mol, delta: &str, label: &str, uncertainty: &'static str) -> Drawing {
    Drawing {
        mode: "bracket",
        atoms: (0..mol.atoms.len()).collect(),
        ends: vec![],
        lost: vec![],
        added: None,
        highlight: vec![],
        zones: vec![],
        kept: vec![],
        kind: None,
        delta: delta.to_string(),
        label: label.to_string(),
        uncertainty,
        fallback: None,
    }
}

fn one_ring(smiles: &str, atoms: &[usize]) -> bool {
    Graph::parse(smiles).is_some_and(|g| {
        g.rings()
            .iter()
            .any(|r| atoms.iter().all(|a| r.contains(a)))
    })
}

/// The form of the drawing of a row; `smiles` = the parent.
pub fn drawing(smiles: &str, site: &SiteReport) -> Drawing {
    let delta = delta_label(&site.transform);
    let Ok(mol) = Mol::parse(smiles) else {
        return Drawing {
            mode: "bracket",
            atoms: vec![],
            uncertainty: "whole",
            ..empty(&delta)
        };
    };
    by_confidence(&mol, smiles, site, &delta)
}

fn by_confidence(mol: &Mol, smiles: &str, site: &SiteReport, delta: &str) -> Drawing {
    match site.confidence {
        "site" => structure(mol, site, delta),
        "region" => zone(mol, smiles, site, delta),
        "uncertain" => whole(mol, delta, &format!("{delta}: position uncertain"), "whole"),
        _ => whole(
            mol,
            delta,
            &format!("{delta}: position not determined"),
            "whole",
        ),
    }
}

/// The atoms of the class with the most support among those not excluded.
fn top_atoms(site: &SiteReport) -> Vec<usize> {
    site.votes
        .iter()
        .filter(|v| v.status != "excluded")
        .max_by(|a, b| a.support.total_cmp(&b.support))
        .map_or(vec![], |v| v.atoms.clone())
}

fn empty(delta: &str) -> Drawing {
    Drawing {
        mode: "bracket",
        atoms: vec![],
        ends: vec![],
        lost: vec![],
        added: None,
        highlight: vec![],
        zones: vec![],
        kept: vec![],
        kind: None,
        delta: delta.to_string(),
        label: delta.to_string(),
        uncertainty: "whole",
        fallback: None,
    }
}

/// The structure of the TP: the top class, its first site; a loss draws what is lost faint.
fn structure(mol: &Mol, site: &SiteReport, delta: &str) -> Drawing {
    let top = site.votes.iter().find(|v| v.status != "excluded");
    let s = top.and_then(|v| site.sites.iter().find(|s| v.atoms.contains(&s.atom)));
    let Some(s) = s else {
        return whole(
            mol,
            delta,
            &format!("{delta}: position not determined"),
            "whole",
        );
    };
    let mut atoms = vec![s.atom];
    atoms.extend(s.partner);
    let lost = if matches!(s.kind, "demethyl" | "deethyl") {
        atoms.clone()
    } else {
        vec![]
    };
    let kept = match (&site.retained, lost.is_empty()) {
        (Some((a, _)), _) => a.clone(),
        (None, false) => (0..mol.atoms.len()).filter(|a| !lost.contains(a)).collect(),
        _ => vec![],
    };
    Drawing {
        mode: "structure",
        atoms,
        lost,
        added: added_of(s.kind),
        kept,
        kind: Some(s.kind.to_string()),
        label: delta.to_string(),
        uncertainty: "none",
        fallback: Some("bracket"),
        ..empty(delta)
    }
}

/// A zone: all the candidates in one aromatic ring give the multicenter bond, any other zone a bracket on the group.
fn zone(mol: &Mol, smiles: &str, site: &SiteReport, delta: &str) -> Drawing {
    let Some((name, group)) = &site.region else {
        return whole(mol, delta, &format!("{delta}: position uncertain"), "whole");
    };
    let cands: Vec<usize> = site
        .votes
        .iter()
        .filter(|v| v.status != "excluded")
        .flat_map(|v| v.atoms.clone())
        .collect();
    let aromatic = !cands.is_empty()
        && site
            .votes
            .iter()
            .filter(|v| v.status != "excluded")
            .all(|v| v.kind.starts_with("aromatic"))
        && cands.iter().all(|&a| mol.atoms[a].arom)
        && one_ring(smiles, &cands);
    let label = format!("{delta}: position uncertain within {name}");
    if aromatic {
        return Drawing {
            mode: "multicenter",
            atoms: group.clone(),
            ends: cands,
            highlight: top_atoms(site),
            label,
            uncertainty: "zone",
            ..empty(delta)
        };
    }
    Drawing {
        atoms: group.clone(),
        ends: vec![],
        highlight: top_atoms(site),
        label,
        uncertainty: "zone",
        ..empty(delta)
    }
}

impl Drawing {
    /// The page has no molfile of the parent: a structure cannot be drawn, so it becomes a bracket around the same atoms.
    pub fn without_molfile(mut self) -> Drawing {
        if self.mode == "structure" {
            self.mode = "bracket";
            self.uncertainty = "zone";
            self.fallback = None;
        }
        self
    }
}

/// `drawing` for a parent given as a `mol/1` record (feature `record`); `smiles` is the text the rings are read from.
#[cfg(feature = "record")]
pub fn drawing_record(rec: &mzlab_mol::Mol, smiles: &str, site: &SiteReport) -> Drawing {
    let delta = delta_label(&site.transform);
    match Mol::from_record(rec, true) {
        Ok(mol) => by_confidence(&mol, smiles, site, &delta),
        Err(_) => Drawing {
            mode: "bracket",
            atoms: vec![],
            uncertainty: "whole",
            ..empty(&delta)
        },
    }
}
