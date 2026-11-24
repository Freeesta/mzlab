//! `cip/1` drawing by drawing: only the defined elements on a stereo site are site-tested and labelled, each connected piece alone, within
//! `WORK_LIMIT` per stage and `NODE_CAP` per label digraph. A centre past the cap is dropped and reported (`capped`); a structure past
//! the work limit, or one whose double bond passes the cap (its E/Z feeds the others), is refused, and so is a drawing left with no label.

use mzlab_core::error::{Error, Result};
use mzlab_mol::{Drawing, Mol, Site, Stereo};

use super::cip::{defined, labels, named, CipLabels, Labels, WORK_LIMIT};
use super::cip_rank::NODE_CAP;

/// The atom that places an element in its piece.
fn site_atom(s: &Stereo) -> usize {
    match s {
        Stereo::Tetra { c, .. } => *c as usize,
        Stereo::Db { a, .. } => *a as usize,
    }
}

fn site_of(s: &Stereo) -> Site {
    match s {
        Stereo::Tetra { c, .. } => Site::Tetra(*c),
        Stereo::Db { a, b, .. } => Site::Db(*a, *b),
    }
}

/// The refusal of a structure past `WORK_LIMIT`, with its atoms and defined elements (`stage`: `sites` or `labels`).
fn too_complex(stage: &str, atoms: usize, elements: usize, work: u64, limit: u64) -> Error {
    Error::new("error.cip.tooComplex")
        .with("stage", stage)
        .with("atoms", atoms)
        .with("elements", elements)
        .with("work", work)
        .with("limit", limit)
}

/// The refusal of a structure whose label digraphs reached `NODE_CAP` (`nodes` is the node that passed it).
fn too_big(atoms: usize, elements: usize) -> Error {
    Error::new("error.cip.digraphTooBig")
        .with("atoms", atoms)
        .with("elements", elements)
        .with("nodes", NODE_CAP + 1)
        .with("limit", NODE_CAP)
}

/// The labels of one piece and its capped centres, refused past the work limit or once the digraph of a double bond reaches `NODE_CAP`.
fn piece_labels(p: &Mol, mine: &[&Stereo], ranked: &[bool]) -> Result<(Labels, Vec<u32>)> {
    let (l, work, db_capped, capped) = labels(p, mine, ranked, WORK_LIMIT);
    if work > WORK_LIMIT {
        return Err(too_complex(
            "labels",
            p.atoms.len(),
            mine.len(),
            work,
            WORK_LIMIT,
        ));
    }
    if db_capped {
        return Err(too_big(p.atoms.len(), mine.len()));
    }
    Ok((l, capped))
}

/// Which elements of a piece (atoms `at` of the record) lie on a site: an element off every site keeps no label, so its digraph is never
/// grown (`at` is ascending, so a < b holds).
fn on_sites(mine: &[&Stereo], at: &[u32], sites: &[Site]) -> Vec<bool> {
    mine.iter()
        .map(|s| {
            sites.contains(&match site_of(s) {
                Site::Tetra(c) => Site::Tetra(at[c as usize]),
                Site::Db(a, b) => Site::Db(at[a as usize], at[b as usize]),
            })
        })
        .collect()
}

/// The labels of a drawing structure by structure: only its defined elements are tested as sites, each connected piece is labelled
/// alone (no rule reaches another piece), and the labels come back in the order of the record, as `cip_labels` gives them.
pub fn drawing_labels(d: &Drawing) -> Result<CipLabels> {
    let units = defined(&d.mol);
    let wanted: Vec<Site> = units.iter().map(|s| site_of(s)).collect();
    let members = d.mol.piece_members();
    let mut piece = vec![0; d.mol.atoms.len()];
    for (k, at) in members.iter().enumerate() {
        for &x in at {
            piece[x as usize] = k;
        }
    }
    // the defined elements of the piece of atom `x`, counted the same way at every stage
    let elements = |x: u32| {
        let k = piece.get(x as usize).copied();
        units
            .iter()
            .filter(|s| k == Some(piece[site_atom(s)]))
            .count()
    };
    let sites = d
        .site_record()
        .stereo_sites_among(&wanted, WORK_LIMIT)
        .map_err(|o| too_complex("sites", o.atoms, elements(o.first), o.work, WORK_LIMIT))?;
    let parts = if members.len() > 1 {
        d.mol.pieces()
    } else {
        vec![d.mol.clone()]
    };
    // a piece `pieces` could not cut out would shift every label after it onto the wrong atoms
    if parts.len() != members.len() {
        return Err(Error::new("error.cip.molfile"));
    }
    let mut all: Labels = (Vec::new(), Vec::new());
    // the centres left without a label, and the size of the first structure that holds one
    let (mut capped, mut first) = (Vec::new(), None);
    for (p, at) in parts.iter().zip(&members) {
        let mine = defined(p);
        let ranked = on_sites(&mine, at, &sites);
        if !ranked.contains(&true) {
            continue;
        }
        let ((centres, bonds), cut) = piece_labels(p, &mine, &ranked)?;
        if !cut.is_empty() {
            first = first.or(Some((p.atoms.len(), mine.len())));
        }
        capped.extend(cut.iter().map(|&c| at[c as usize]));
        let g = |x: usize| at[x] as usize;
        all.0.extend(
            centres
                .into_iter()
                .map(|(c, v, low)| (at[c as usize], v, low)),
        );
        all.1
            .extend(bonds.into_iter().map(|(a, b, v)| (g(a), g(b), v)));
    }
    // back in the order of the record's elements
    let mut ordered: Labels = (Vec::new(), Vec::new());
    for w in &wanted {
        match *w {
            Site::Tetra(c) => ordered.0.extend(all.0.iter().find(|l| l.0 == c)),
            Site::Db(a, b) => ordered
                .1
                .extend(all.1.iter().find(|l| (l.0 as u32, l.1 as u32) == (a, b))),
        }
    }
    let mut l = named(ordered, &sites);
    // a centre past the cap is dropped and reported; the drawing is refused only when no label is left
    if let (Some((atoms, elements)), true) = (first, l.atoms.is_empty() && l.bonds.is_empty()) {
        return Err(too_big(atoms, elements));
    }
    l.capped = capped;
    Ok(l)
}
