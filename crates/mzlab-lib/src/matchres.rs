//! The compound-centric result of a library match (`libmatch/1`, docs/schemi/libmatch-1.json): the hits of identity and analogue searches over a `LibSet`
//! grouped by compound (first InChIKey block, else the normalised name), with badge, evidence and the peak pairs a mirror plot draws. Built here once for the
//! Flow, the side panel and the CLI. The confidence level is not computed: the field stays null for the cascade, which reads `experimental`, `hr`, scores and peaks.

use crate::error::Result;
use crate::frag_tol::FragTol;
use crate::io::Source;
use crate::libinfo::Kind;
use crate::libset::LibSet;
use crate::search::{AnalogHit, AnalogOpts, Hit, IdentityOpts, Query, Tol};
use crate::spec::Meta;
use mzlab_picchi::sortc::SortC;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Exact,
    Analog,
    InSilico,
}

impl Badge {
    pub fn as_str(self) -> &'static str {
        match self {
            Badge::Exact => "exact",
            Badge::Analog => "analog",
            Badge::InSilico => "in_silico",
        }
    }
    fn rank(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairKind {
    Direct,
    Shifted,
    Loss,
}

impl PairKind {
    fn as_str(self) -> &'static str {
        match self {
            PairKind::Direct => "direct",
            PairKind::Shifted => "shifted",
            PairKind::Loss => "loss",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PeakPair {
    pub query_mz: f64,
    pub library_mz: f64,
    pub kind: PairKind,
}

/// One library spectrum matched to the query. `delta_mz` is query minus library precursor (the modification of an analogue), `delta_ppm` that over the
/// library precursor; `of` is the number of query peaks compared (the 32 strongest for analogues).
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumMatch {
    pub lib: u16,
    pub index: u32,
    pub accession: String,
    pub score: f64,
    pub score_kind: &'static str,
    pub reverse: f64,
    pub explained: f64,
    pub matched: usize,
    pub of: usize,
    pub direct: usize,
    pub shifted: usize,
    pub delta_ppm: f64,
    pub delta_mz: f64,
    pub adduct: String,
    pub ce: String,
    pub instrument: String,
    pub licence: String,
    /// Mask of the libraries (within the search mask) that hold this same spectrum.
    pub libs: u16,
    pub pairs: Vec<PeakPair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Compound {
    pub inchikey14: String,
    pub name: String,
    pub formula: String,
    pub smiles: String,
    pub badge: Badge,
    pub experimental: bool,
    /// The spectra, best score first; the first is the best one.
    pub spectra: Vec<SpectrumMatch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LibRef {
    pub index: u16,
    pub name: String,
    pub kind: &'static str,
}

#[derive(Debug, Clone)]
pub struct MatchParams {
    /// Precursor window of an exact match: ppm (high resolution) or Da.
    pub exact: Tol,
    pub frag: FragTol,
    /// Polarity of the libraries searched, 0 = the query's.
    pub pol: i8,
    pub mask: u16,
    pub top_compounds: usize,
    pub analogs: bool,
    pub analog_max_delta: f64,
    pub min_matched: usize,
    pub lib_names: Vec<String>,
}

impl MatchParams {
    pub fn new(exact: Tol, frag: FragTol, mask: u16) -> MatchParams {
        MatchParams {
            exact,
            frag,
            pol: 0,
            mask,
            top_compounds: 10,
            analogs: true,
            analog_max_delta: 200.0,
            min_matched: 4,
            lib_names: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    pub hr: bool,
    pub mask: u16,
    pub top_compounds: usize,
    pub libraries: Vec<LibRef>,
    pub compounds: Vec<Compound>,
}

fn norm_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Compound identity: first block of the InChIKey, else the normalised name, else the accession (a spectrum alone).
fn compound_key(m: &Meta) -> (String, String) {
    let k = m.inchikey.trim();
    if k.len() >= 14 && k.is_char_boundary(14) {
        return (k[..14].to_uppercase(), k[..14].to_uppercase());
    }
    let n = norm_name(&m.name);
    if !n.is_empty() {
        return (format!("name:{n}"), String::new());
    }
    (format!("acc:{}", m.accession), String::new())
}

fn spectrum_of(lib: u16, index: u32, libs: u16, meta: &Meta) -> SpectrumMatch {
    SpectrumMatch {
        lib,
        index,
        accession: meta.accession.clone(),
        score: 0.0,
        score_kind: "",
        reverse: 0.0,
        explained: 0.0,
        matched: 0,
        of: 0,
        direct: 0,
        shifted: 0,
        delta_ppm: 0.0,
        delta_mz: 0.0,
        adduct: meta.adduct.clone(),
        ce: meta.ce.clone(),
        instrument: meta.instrument.clone(),
        licence: meta.license.clone(),
        libs,
        pairs: Vec::new(),
    }
}

fn identity_match(q: &Query, h: &Hit, mut m: SpectrumMatch) -> SpectrumMatch {
    m.score = h.entropy;
    m.score_kind = "entropy";
    m.reverse = h.reverse;
    m.explained = h.explained;
    m.matched = h.pairs.len();
    m.of = q.spec.mz.len();
    m.direct = h.pairs.len();
    m.delta_mz = q.prec - h.prec;
    m.delta_ppm = m.delta_mz / h.prec * 1e6;
    m.pairs = h
        .pairs
        .iter()
        .map(|&(i, j)| PeakPair {
            query_mz: q.spec.mz[i],
            library_mz: h.spec.mz[j],
            kind: PairKind::Direct,
        })
        .collect();
    m
}

fn analog_match(q: &Query, h: &AnalogHit, mut m: SpectrumMatch) -> SpectrumMatch {
    let mc = &h.mc;
    let (na, nb): (f64, f64) = (mc.a_top.it.iter().sum(), mc.b_top.it.iter().sum());
    let dot: f64 = mc
        .pairs
        .iter()
        .map(|&(i, j, _)| (mc.a_top.it[i] * mc.b_top.it[j]).sqrt())
        .sum();
    let na_m: f64 = mc.pairs.iter().map(|&(i, _, _)| mc.a_top.it[i]).sum();
    m.score = mc.cosine;
    m.score_kind = "modified_cosine";
    m.reverse = if na_m > 0.0 && nb > 0.0 {
        dot / (na_m * nb).sqrt()
    } else {
        0.0
    };
    m.explained = if na > 0.0 { na_m / na } else { 0.0 };
    m.matched = mc.matched;
    m.of = mc.a_top.mz.len();
    m.shifted = mc.shifted;
    m.direct = mc.matched - mc.shifted;
    m.delta_mz = q.prec - h.prec;
    m.delta_ppm = m.delta_mz / h.prec * 1e6;
    m.pairs = mc
        .pairs
        .iter()
        .map(|&(i, j, sh)| PeakPair {
            query_mz: mc.a_top.mz[i],
            library_mz: mc.b_top.mz[j],
            kind: if sh {
                PairKind::Shifted
            } else {
                PairKind::Direct
            },
        })
        .collect();
    m
}

fn lib_kind<S: Source>(set: &LibSet<S>, k: u16) -> Kind {
    set.lib(k as usize)
        .and_then(|l| l.info().kind)
        .unwrap_or(Kind::Experimental)
}

/// Searches the libraries of `p.mask` (identity in the exact window, then analogues) and groups the spectra by compound: exact first, then analogues,
/// then in silico, each by best score; at most `p.top_compounds` compounds.
pub fn match_query<S: Source>(set: &LibSet<S>, q: &Query, p: &MatchParams) -> Result<MatchResult> {
    let pol = if p.pol != 0 { p.pol } else { q.pol };
    let per = p.top_compounds.max(1) * 3;
    let io = IdentityOpts {
        prec_tol: p.exact,
        frag_da: 0.0,
        pol,
        top: per,
    };
    let mut found: Vec<(Badge, SpectrumMatch, Meta)> = Vec::new();
    let mut add = |set: &LibSet<S>,
                   lib: u16,
                   index: u32,
                   libs: u16,
                   analog: bool,
                   f: &dyn Fn(SpectrumMatch) -> SpectrumMatch|
     -> Result<()> {
        let meta = set
            .lib(lib as usize)
            .map(|l| l.meta(index))
            .transpose()?
            .unwrap_or_default();
        let silico = lib_kind(set, lib) == Kind::InSilico;
        let badge = if silico {
            Badge::InSilico
        } else if analog {
            Badge::Analog
        } else {
            Badge::Exact
        };
        let base = spectrum_of(lib, index, libs, &meta);
        found.push((badge, f(base), meta));
        Ok(())
    };
    for h in set.identity(q, &io, p.frag, p.mask)? {
        add(set, h.lib, h.index, h.libs, false, &|m| {
            identity_match(q, &h.hit, m)
        })?;
    }
    if p.analogs {
        let window = p.exact.window(q.prec).max(0.5);
        let ao = AnalogOpts {
            pol,
            max_delta: p.analog_max_delta,
            min_delta: window,
            min_matched: p.min_matched,
            top: per,
            ..AnalogOpts::default()
        };
        let ao = AnalogOpts {
            frag_da: p.frag.da,
            frag_ppm: p.frag.ppm,
            ..ao
        };
        for h in set.analogs(q, &ao, p.mask)? {
            add(set, h.lib, h.index, h.libs, true, &|m| {
                analog_match(q, &h.hit, m)
            })?;
        }
    }
    let mut compounds = group_by_compound(set, found);
    compounds.truncate(p.top_compounds.max(1));
    let libraries = (0..set.len() as u16)
        .map(|k| LibRef {
            index: k,
            name: p.lib_names.get(k as usize).cloned().unwrap_or_default(),
            kind: lib_kind(set, k).as_str(),
        })
        .collect();
    Ok(MatchResult {
        hr: matches!(p.exact, Tol::Ppm(_)),
        mask: p.mask,
        top_compounds: p.top_compounds.max(1),
        libraries,
        compounds,
    })
}

/// Groups the spectra by compound and orders them: badge, then best score.
fn group_by_compound<S: Source>(
    set: &LibSet<S>,
    found: Vec<(Badge, SpectrumMatch, Meta)>,
) -> Vec<Compound> {
    // insertion order is kept in `compounds`; the map only finds the group of a key
    let mut compounds: Vec<Compound> = Vec::new();
    let mut at: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (badge, sm, meta) in found {
        let (key, ik) = compound_key(&meta);
        let exp = lib_kind(set, sm.lib) == Kind::Experimental;
        match at.get(&key) {
            Some(&g) => {
                let c = &mut compounds[g];
                c.badge = if badge.rank() < c.badge.rank() {
                    badge
                } else {
                    c.badge
                };
                c.experimental |= exp;
                c.spectra.push(sm);
            }
            None => {
                at.insert(key, compounds.len());
                compounds.push(Compound {
                    inchikey14: ik,
                    name: meta.name,
                    formula: meta.formula,
                    smiles: meta.smiles,
                    badge,
                    experimental: exp,
                    spectra: vec![sm],
                });
            }
        }
    }
    for c in &mut compounds {
        c.spectra.sortc_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(a.lib.cmp(&b.lib))
                .then(a.index.cmp(&b.index))
        });
    }
    compounds.sortc_by(|a, b| {
        a.badge
            .rank()
            .cmp(&b.badge.rank())
            .then(b.spectra[0].score.total_cmp(&a.spectra[0].score))
            .then(a.name.cmp(&b.name))
    });
    compounds
}

mod json;
