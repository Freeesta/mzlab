// Ported from Attributio ms2_engine/matcher.py (the entropy "identity" search; build_flash.py builds the index).
//! Entropy-similarity search of feature spectra against an engine library. The cleaning of the spectra, the 0.02 Da fragment tolerance and the ppm precursor window
//! follow the original; the score is the entropy similarity of `mzlab-lib` (held equal to `ms_entropy` by its own parity tests). A hit also needs a minimum of shared
//! ions (the original had no such gate: two peaks can score high on a sparse spectrum).

use super::compile::EngineEntry;
use mzlab_lib::clean::{clean, CleanOpts};
use mzlab_lib::scores::{pair_scores, Spec};

pub const FRAG_TOL_DA: f64 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOpts {
    pub ppm: f64,
    pub min_score: f64,
    /// Minimum of matched ions between query and library spectrum.
    pub min_shared: usize,
    pub top: usize,
}

impl Default for SearchOpts {
    fn default() -> Self {
        SearchOpts {
            ppm: 10.0,
            min_score: 0.6,
            min_shared: 3,
            top: 20,
        }
    }
}

/// A feature spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub id: String,
    pub precursor_mz: f64,
    pub pol: i8,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub library_precursor_mz: f64,
    pub adduct: String,
    pub score: f64,
    pub shared: usize,
    pub ppm: f64,
}

fn f32r(v: f64) -> f64 {
    v as f32 as f64
}

/// `ms_entropy` cleaning: peaks in m/z [10, 1000] as float32, then the default cleaning (precursor region, 1 % noise, 0.05 Da centroiding), maximum 1.
fn clean_spec(prec: f64, mz: &[f64], it: &[f64]) -> Option<Spec> {
    let (m, i): (Vec<f64>, Vec<f64>) = mz
        .iter()
        .zip(it)
        .map(|(a, b)| (f32r(*a), f32r(*b)))
        .filter(|(a, b)| *a >= 10.0 && *a <= 1000.0 && *b > 0.0)
        .unzip();
    let (m, i) = clean(prec, &m, &i, &CleanOpts::default());
    let mx = i.iter().cloned().fold(0.0, f64::max);
    if m.is_empty() || mx <= 0.0 {
        return None;
    }
    Some(Spec {
        mz: m,
        it: i.iter().map(|v| v / mx).collect(),
        prec,
    })
}

struct Prepared {
    entry_ix: usize,
    prec: f64,
    spec: Spec,
}

/// A library cleaned and sorted by precursor, one list per polarity.
pub struct Ms2Library {
    entries: Vec<EngineEntry>,
    pos: Vec<Prepared>,
    neg: Vec<Prepared>,
}

impl Ms2Library {
    pub fn new(entries: Vec<EngineEntry>) -> Ms2Library {
        let (mut pos, mut neg) = (Vec::new(), Vec::new());
        for (k, e) in entries.iter().enumerate() {
            if let Some(spec) = clean_spec(e.precursor_mz, &e.mz, &e.it) {
                let p = Prepared {
                    entry_ix: k,
                    prec: e.precursor_mz,
                    spec,
                };
                if e.pol > 0 {
                    pos.push(p)
                } else {
                    neg.push(p)
                }
            }
        }
        let key = |p: &Prepared| p.prec;
        pos.sort_by(|a, b| key(a).total_cmp(&key(b)).then(a.entry_ix.cmp(&b.entry_ix)));
        neg.sort_by(|a, b| key(a).total_cmp(&key(b)).then(a.entry_ix.cmp(&b.entry_ix)));
        Ms2Library { entries, pos, neg }
    }

    pub fn len(&self) -> usize {
        self.pos.len() + self.neg.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Hits of one query, best score first; empty when the query has no usable peak.
    pub fn search(&self, q: &Query, o: &SearchOpts) -> Vec<SearchHit> {
        let Some(spec) = clean_spec(q.precursor_mz, &q.mz, &q.it) else {
            return Vec::new();
        };
        let list = if q.pol > 0 { &self.pos } else { &self.neg };
        let tol = q.precursor_mz * o.ppm / 1e6;
        let lo = list.partition_point(|p| p.prec < q.precursor_mz - tol);
        let mut hits = Vec::new();
        for p in list[lo..]
            .iter()
            .take_while(|p| p.prec <= q.precursor_mz + tol)
        {
            let sc = pair_scores(&spec, &p.spec, FRAG_TOL_DA);
            if sc.entropy >= o.min_score && sc.shared >= o.min_shared {
                let e = &self.entries[p.entry_ix];
                hits.push(SearchHit {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    library_precursor_mz: e.precursor_mz,
                    adduct: e.adduct.clone(),
                    score: sc.entropy,
                    shared: sc.shared,
                    ppm: (e.precursor_mz - q.precursor_mz) / q.precursor_mz * 1e6,
                });
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.id.cmp(&b.id)));
        hits.truncate(o.top.max(1));
        hits
    }
}
