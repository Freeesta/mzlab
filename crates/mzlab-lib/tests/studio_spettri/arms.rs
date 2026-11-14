//! Arms of the spectral study. The baselines always go through the real engine: a `.mzlib` built by `build::build_in_memory` from the targets and
//! their `DecoyKind::Fragments` decoys (what `spectra.match decoys=spectrum` builds), `search::identity_with` / `analogs_with`, and the q-values
//! of `decoy::lib_fdr` on the best target and the best decoy of each query, exactly as `decoy::best_scores_with` picks them.

use super::r#gen::Peaks;
use super::scen::{Entry, Scen};
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::decoy::{is_decoy, lib_fdr, quantize, with_decoys, DecoyKind, DEFAULT_SEED};
use mzlab_lib::flash::FlashAcc;
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::modified_cosine;
use mzlab_lib::search::{analogs_with, identity_with, AnalogOpts, Hit, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};

/// The defaults of `spectra.match` on a high-resolution query: 10 ppm precursor window, 0.02 Da fragments.
pub const PREC_PPM: f64 = 10.0;
pub const FRAG_DA: f64 = 0.02;
pub const BASELINES: [&str; 4] = ["entropy", "cosine", "modified_cosine", "analogs"];

pub fn spectrum(id: usize, e: &Entry) -> Spectrum {
    Spectrum {
        prec: e.peaks.prec,
        pol: 1,
        mz: e.peaks.mz.clone(),
        it: e.peaks.it.clone(),
        meta: Meta {
            name: format!("syn {id}"),
            adduct: "[M+H]+".into(),
            ce: "35".into(),
            formula: super::r#gen::text(&e.neutral),
            accession: format!("SYN-{id}"),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The `.mzlib` bytes of the library of `s` (entries `keep`, or all), each target followed by its decoy.
pub fn build_lib(s: &Scen, keep: Option<&[bool]>) -> Vec<u8> {
    let targets: Vec<Spectrum> = (0..s.lib.len())
        .filter(|&i| keep.is_none_or(|k| k[i]))
        .map(|i| spectrum(i, &s.lib[i]))
        .collect();
    let all = with_decoys(&targets, DecoyKind::Fragments, DEFAULT_SEED);
    build_in_memory(&all, BuildOpts::default()).unwrap().0
}

/// An opened library and, per library index, the entry id of a target (`None` for a decoy).
pub struct Lib<'a> {
    pub lib: Library<&'a [u8]>,
    pub ids: Vec<Option<usize>>,
}

pub fn open(bytes: &[u8]) -> Lib<'_> {
    let lib = Library::open(bytes, CACHE_BYTES).unwrap();
    let ids = (0..lib.n_spectra())
        .map(|i| {
            let a = lib.meta(i).unwrap().accession;
            (!is_decoy(&a)).then(|| a.trim_start_matches("SYN-").parse().unwrap())
        })
        .collect();
    Lib { lib, ids }
}

pub fn query(p: &Peaks) -> Option<Query> {
    Query::new(p.prec, 1, &p.mz, &p.it)
}

/// One arm on one query: target entries ranked best first with their scores, the best decoy score, the shared peaks and the library peak count of
/// the top hit, and the q-value of the top score within the batch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub ranked: Vec<(usize, f32)>,
    pub decoy: f32,
    pub shared: usize,
    pub nlib: usize,
    pub q: f32,
}

pub fn identity_opts() -> IdentityOpts {
    IdentityOpts {
        prec_tol: Tol::Ppm(PREC_PPM),
        frag_da: FRAG_DA,
        pol: 1,
        top: usize::MAX,
    }
}

/// Scores (score, shared peaks) of every candidate of the identity window, ranked by the quantized score (ties keep the engine's order).
pub fn identity_row(l: &Lib, q: &Query, score: &dyn Fn(&Query, &Hit) -> (f64, usize)) -> Row {
    let mut row = Row::default();
    let mut t: Vec<(usize, f32, usize, usize)> = Vec::new();
    for h in identity_with(&l.lib, q, &identity_opts(), FragTol::from(FRAG_DA)).unwrap() {
        let (s, shared) = score(q, &h);
        match l.ids[h.index as usize] {
            Some(id) => t.push((id, quantize(s), shared, h.spec.mz.len())),
            None => row.decoy = row.decoy.max(quantize(s)),
        }
    }
    t.sort_by(|a, b| b.1.total_cmp(&a.1));
    if let Some(top) = t.first() {
        (row.shared, row.nlib) = (top.2, top.3);
    }
    row.ranked = t.iter().map(|x| (x.0, x.1)).collect();
    row
}

/// Analogue search as `spectra.match` runs it (Flash hybrid candidates, modified cosine ranking), decoys counted like the identity arms.
pub fn analog_row(l: &Lib, q: &Query, acc: &mut FlashAcc) -> Row {
    let o = AnalogOpts {
        frag_da: FRAG_DA,
        pol: 1,
        ..AnalogOpts::default()
    };
    let mut row = Row::default();
    for h in analogs_with(&l.lib, q, &o, acc).unwrap() {
        let s = quantize(h.mc.cosine);
        match l.ids[h.index as usize] {
            Some(id) => {
                if row.ranked.is_empty() {
                    (row.shared, row.nlib) = (h.mc.matched, h.spec.mz.len());
                }
                row.ranked.push((id, s));
            }
            None => row.decoy = row.decoy.max(s),
        }
    }
    row
}

/// The q-value of each top score within the batch (`decoy::lib_fdr`; a query without a target gets q = 1).
pub fn add_q(rows: &mut [Row]) {
    let t: Vec<f32> = rows
        .iter()
        .map(|r| r.ranked.first().map_or(f32::NAN, |x| x.1))
        .collect();
    let d: Vec<f32> = rows.iter().map(|r| r.decoy).collect();
    for (r, f) in rows.iter_mut().zip(lib_fdr(&t, &d, DecoyKind::Fragments)) {
        r.q = f.q_value;
    }
}

/// The rows of a baseline arm over the queries (`None`: nothing left after the cleaning).
pub fn baseline(arm: &str, l: &Lib, qs: &[Option<Query>]) -> Vec<Row> {
    let mut acc = FlashAcc::new();
    let frag = FragTol::from(FRAG_DA);
    let mut rows: Vec<Row> = qs
        .iter()
        .map(|q| {
            let Some(q) = q else { return Row::default() };
            match arm {
                "entropy" => identity_row(l, q, &|_, h| (h.entropy, h.shared)),
                "cosine" => identity_row(l, q, &|_, h| (h.cosine, h.shared)),
                "modified_cosine" => identity_row(l, q, &|q, h| {
                    let m = modified_cosine(&q.spec, &h.spec, frag);
                    (m.cosine, m.matched)
                }),
                "analogs" => analog_row(l, q, &mut acc),
                _ => panic!("unknown arm {arm}"),
            }
        })
        .collect();
    add_q(&mut rows);
    rows
}
