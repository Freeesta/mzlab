//! Alternatives with the same formula (new in mzLab, no counterpart in Attributio): the other structures that the libraries already indexed and the CSI:FingerID
//! candidates of the campaign give for the formula the chemist is about to confirm, with their spectral score when the feature has a spectrum and their logP when
//! the sources give one. They tell the chemist which isomers still stand; they decide nothing. No network: everything comes from the files of the campaign.

use super::explain::{fmt_formula, parse_formula};
use super::isotopes::round_to;
use super::libraries::LibraryIndex;
use super::matching::{compare, Ms2};
use super::sources::StructureCandidate;
use super::structures::Structures;

/// An alternative structure with the formula asked.
#[derive(Clone, Debug, PartialEq)]
pub struct Alt {
    pub name: String,
    /// `library: <file>` or `CSI:FingerID`.
    pub source: String,
    /// Library entry (accession) when the source is a library spectrum.
    pub entry: Option<String>,
    pub ik1: Option<String>,
    pub smiles: Option<String>,
    /// Cosine of the library spectrum against the experimental one (`None` without a spectrum on either side).
    pub score: Option<f64>,
    /// Number of peaks matched when the score exists.
    pub matched: Option<usize>,
    pub logp: Option<f64>,
}

/// What the alternatives are searched in.
pub struct AltSources<'a> {
    /// CSI:FingerID candidates of the feature (five at most).
    pub csi: &'a [StructureCandidate],
    /// The cleaned spectrum of the feature, if it has one.
    pub exp: Option<&'a Ms2>,
    /// m/z tolerance of the spectral comparison (`tol_da`).
    pub tol: f64,
    pub st: &'a dyn Structures,
}

/// The neutral formula of `text` in the canonical order, `None` when it cannot be read.
fn canon(text: &str) -> Option<String> {
    let f = parse_formula(text.trim());
    (!f.is_empty()).then(|| fmt_formula(&f))
}

fn same_compound(a: &Alt, b: &Alt) -> bool {
    match (&a.ik1, &b.ik1) {
        (Some(x), Some(y)) => x == y,
        _ => a.name.to_lowercase() == b.name.to_lowercase(),
    }
}

fn from_library(formula: &str, lib: &LibraryIndex, s: &AltSources) -> Vec<Alt> {
    let mut out: Vec<Alt> = Vec::new();
    for (k, file) in lib.files.iter().enumerate() {
        for (row, r) in file.records.iter().enumerate() {
            let own = canon(&r.formula).or_else(|| {
                s.st.formula(&r.smiles)
                    .map(|f| fmt_formula(&f))
                    .filter(|f| !f.is_empty())
            });
            if own.as_deref() != Some(formula) {
                continue;
            }
            let cmp = s
                .exp
                .filter(|e| !e.mz.is_empty() && !r.mz.is_empty())
                .map(|e| {
                    compare(
                        e,
                        &lib.ms2(super::libraries::Ref { file: k, row }),
                        s.tol,
                        None,
                        0.0,
                    )
                });
            let alt = Alt {
                name: if r.name.is_empty() {
                    "(unnamed)".to_string()
                } else {
                    r.name.clone()
                },
                source: format!("library: {}", file.label),
                entry: Some(r.entry.clone()).filter(|e| !e.is_empty()),
                ik1: Some(r.ik1.clone()).filter(|e| !e.is_empty()),
                smiles: Some(r.smiles.clone()).filter(|e| !e.is_empty()),
                score: cmp.as_ref().map(|c| round_to(c.cos, 4)),
                matched: cmp.as_ref().map(|c| c.matched),
                logp: None,
            };
            match out.iter_mut().find(|x| same_compound(x, &alt)) {
                Some(x) if alt.score > x.score => *x = alt,
                Some(_) => {}
                None => out.push(alt),
            }
        }
    }
    out
}

fn from_csi(formula: &str, s: &AltSources) -> Vec<Alt> {
    s.csi
        .iter()
        .filter(|c| canon(&c.formula).as_deref() == Some(formula))
        .map(|c| Alt {
            name: if c.name.is_empty() {
                c.smiles.clone()
            } else {
                c.name.clone()
            },
            source: "CSI:FingerID".to_string(),
            entry: None,
            ik1: c.ik1.clone(),
            smiles: Some(c.smiles.clone()).filter(|x| !x.is_empty()),
            score: None,
            matched: None,
            logp: c.xlogp.filter(|v| v.is_finite()).map(|v| round_to(v, 2)),
        })
        .collect()
}

/// The alternatives with `formula` (neutral, any notation), best spectral score first, those without a score after, then by name. A library compound whose
/// logP the CSI:FingerID candidates give (same first InChIKey block) takes it.
pub fn same_formula(formula: &str, lib: &LibraryIndex, src: &AltSources) -> Vec<Alt> {
    let Some(want) = canon(formula) else {
        return Vec::new();
    };
    let csi = from_csi(&want, src);
    let mut out = from_library(&want, lib, src);
    for a in out.iter_mut().filter(|a| a.logp.is_none()) {
        a.logp = csi
            .iter()
            .find(|c| c.ik1.is_some() && c.ik1 == a.ik1)
            .and_then(|c| c.logp);
    }
    for c in csi {
        match out.iter_mut().find(|x| same_compound(x, &c)) {
            Some(x) => {
                x.source = format!("{} + CSI:FingerID", x.source);
                x.logp = x.logp.or(c.logp);
            }
            None => out.push(c),
        }
    }
    out.sort_by(|a, b| {
        let key = |x: &Alt| x.score.map_or(f64::INFINITY, |s| -s);
        key(a)
            .total_cmp(&key(b))
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}
