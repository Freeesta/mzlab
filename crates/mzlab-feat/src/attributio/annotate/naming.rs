// Ported from Attributio annotate.py.
//! The name of a feature and the conflicts between sources: which valid sources name it, whether they name different structures (InChIKey first block, or a
//! curated name in common), and, when valid libraries disagree, the best match (cosine, then shared ions, then the order of the sources) with its margin.

use super::curate::Curation;
use super::gates::inchikey_block;
use super::settings::AnnotationSettings;
use super::text::{fmt_g, num_txt};
use super::types::Feature;
use crate::attributio::formula::{structure_fit, STRUCTURE_TOL_PPM};

/// One valid library hit of a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryHit {
    pub source: &'static str,
    pub name: String,
    pub cosine: Option<f64>,
    pub ions: Option<f64>,
    pub block: Option<String>,
    pub order: usize,
}

/// The names that passed their gates, in the order of precedence (reference standard, SIRIUS spectral, GNPS2, mzMine library, putative library, CSI:FingerID,
/// suspect-RT).
pub fn valid_names(f: &Feature) -> [Option<String>; 7] {
    let g = &f.gates;
    let pick = |on: bool, v: &Option<String>| if on { v.clone() } else { None };
    let std_name = f.std.as_ref().and_then(|m| m.name.clone());
    let putative = pick(g.spec_putative, &f.spec.name)
        .or_else(|| pick(g.gnps_putative, &f.gnps.name))
        .or_else(|| pick(g.mzlib_putative, &f.mzlib.name));
    [
        pick(g.l1, &std_name),
        pick(g.sirius_spectral, &f.spec.name),
        pick(g.gnps2, &f.gnps.name),
        pick(g.mzmine_library, &f.mzlib.name),
        putative,
        pick(g.csi, &f.csi.name),
        pick(g.std_rt && !g.l1, &std_name),
    ]
}

/// The first block of the InChIKey of each source, as the source itself gives it with its name: SIRIUS spectral, GNPS2, mzMine library (only when the
/// formula of its structure explains the entry's own precursor: a known bug of the mzMine export) and CSI:FingerID.
fn blocks(f: &Feature) -> [Option<String>; 4] {
    let mz_ok = structure_fit(
        f.mzlib.formula.as_deref(),
        f.mzlib.adduct.as_deref(),
        f.mzlib.precursor_mz,
        &f.pol,
        STRUCTURE_TOL_PPM,
    )
    .1 == Some(true);
    [
        inchikey_block(f.spec.inchikey2d.as_deref()),
        inchikey_block(f.gnps.inchikey.as_deref()),
        inchikey_block(f.mzlib.inchikey.as_deref()).filter(|_| mz_ok),
        inchikey_block(f.csi.inchikey2d.as_deref()),
    ]
}

/// True when two named sources carry different first blocks and the contaminant map does not give both the same curated name.
fn names_conflict(named: &[(Option<&str>, Option<&str>)], cur: &dyn Curation) -> bool {
    let have: Vec<(&str, Option<String>)> = named
        .iter()
        .filter_map(|(n, b)| {
            let n = (*n)?;
            let b = (*b)?;
            (!n.trim().is_empty() && !b.is_empty())
                .then(|| (b, cur.match_cec(&[Some(n)]).map(|c| c.0)))
        })
        .collect();
    for i in 0..have.len() {
        for j in i + 1..have.len() {
            let same_cec = have[i].1.is_some() && have[i].1 == have[j].1;
            if have[i].0 != have[j].0 && !same_cec {
                return true;
            }
        }
    }
    false
}

fn same_structure(a: &LibraryHit, b: &LibraryHit, cur: &dyn Curation) -> bool {
    if let (Some(x), Some(y)) = (&a.block, &b.block) {
        if x == y {
            return true;
        }
    }
    let (ca, cb) = (
        cur.match_cec(&[Some(&a.name)]).map(|c| c.0),
        cur.match_cec(&[Some(&b.name)]).map(|c| c.0),
    );
    ca.is_some() && ca == cb
}

/// The valid library hits of a feature, best first: the highest cosine, then more shared ions, then the order of the sources.
pub fn rank_library_hits(f: &Feature, names: &[Option<String>; 7]) -> Vec<LibraryHit> {
    let b = blocks(f);
    let cands = [
        (
            "SIRIUS spectral",
            &names[1],
            f.spec.sim,
            f.spec.peaks,
            b[0].clone(),
        ),
        ("GNPS2", &names[2], f.gnps.score, f.gnps.peaks, b[1].clone()),
        (
            "mzMine library",
            &names[3],
            f.mzlib.score,
            f.mzlib.peaks,
            b[2].clone(),
        ),
    ];
    let mut hits: Vec<LibraryHit> = cands
        .into_iter()
        .enumerate()
        .filter_map(|(k, (source, n, cosine, ions, block))| {
            let n = n.as_ref().filter(|n| !n.trim().is_empty())?;
            Some(LibraryHit {
                source,
                name: n.clone(),
                cosine,
                ions,
                block,
                order: k,
            })
        })
        .collect();
    let key = |v: Option<f64>| -v.filter(|x| !x.is_nan()).unwrap_or(-1.0);
    hits.sort_by(|a, b| {
        key(a.cosine)
            .total_cmp(&key(b.cosine))
            .then(key(a.ions).total_cmp(&key(b.ions)))
            .then(a.order.cmp(&b.order))
    });
    hits
}

fn choice_text(
    ranked: &[LibraryHit],
    margin: f64,
    near_tie: bool,
    s: &AnnotationSettings,
) -> String {
    let one = |h: &LibraryHit| {
        format!(
            "{} '{}' (cosine {}, {} ions)",
            h.source,
            h.name,
            num_txt(h.cosine, 2),
            num_txt(h.ions, 0)
        )
    };
    let rest: Vec<String> = ranked[1..].iter().map(one).collect();
    format!(
        "best library match: {} over {}; margin {} {}{}",
        one(&ranked[0]),
        rest.join("; "),
        num_txt(Some(margin), 3),
        if near_tie { "< " } else { ">= " },
        fmt_g(s.library_conflict_margin)
    )
}

/// Display name, contaminant, name conflicts and the best library match of every feature; a library conflict can demote a Level 2 to 3 (`name conflict`).
pub fn name_features(feats: &mut [Feature], s: &AnnotationSettings, cur: &dyn Curation) {
    for f in feats.iter_mut() {
        let v = valid_names(f);
        let refs: Vec<Option<&str>> = v.iter().map(|n| n.as_deref()).collect();
        f.disp_name = v.iter().flatten().next().cloned();
        f.cec = cur.match_cec(&refs);
        let b = blocks(f);
        let (sirius, gnps, mzlib, csi) = (&v[1], &v[2], &v[3], &v[5]);
        let named = [
            (sirius.as_deref(), b[0].as_deref()),
            (gnps.as_deref(), b[1].as_deref()),
            (mzlib.as_deref(), b[2].as_deref()),
            (csi.as_deref(), b[3].as_deref()),
        ];
        f.name_conflict = names_conflict(&named, cur);
        f.library_conflict = names_conflict(&named[..3], cur);
        let mut blocks: Vec<&str> = named
            .iter()
            .filter_map(|(n, bl)| n.and(*bl).filter(|x| !x.is_empty()))
            .collect();
        blocks.sort_unstable();
        blocks.dedup();
        f.name_blocks = blocks.join("|");
        let mut near_tie = false;
        let lib_conf = f.library_conflict && !f.gates.l1;
        if s.library_conflict == "best_match" && lib_conf {
            let ranked = rank_library_hits(f, &v);
            let best = &ranked[0];
            let other = ranked[1..].iter().find(|h| !same_structure(h, best, cur));
            let margin = match other {
                Some(o) => best.cosine.unwrap_or(f64::NAN) - o.cosine.unwrap_or(f64::NAN),
                None => f64::INFINITY,
            };
            near_tie = margin < s.library_conflict_margin;
            f.disp_name = Some(best.name.clone());
            f.library_margin = Some(margin);
            f.name_choice = Some(choice_text(&ranked, margin, near_tie, s));
        }
        if f.name_conflict {
            f.cec = cur.match_cec(&[f.disp_name.as_deref()]);
        }
        let demote = f.level == 2
            && (if s.library_conflict == "level3" {
                f.library_conflict
            } else {
                near_tie
            });
        if demote {
            f.level = 3;
            f.level_basis = "name conflict".into();
            f.level_label = s.label("name conflict");
        }
    }
}
