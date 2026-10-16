// Ported from Attributio src/attributio/review/build.py (_feature, _finish_candidates, _purity_block).
//! The review of one feature: candidate structures and how much of the MS2 each explains, the reference spectra (library exact and analog, the hits of
//! the pipeline tools), the raw proofs and the purity of the MS2, ending in the tier of `propose`. Everything the file systems hold (the MGF block, the
//! libraries, the raw scans) arrives through the arguments; a missing piece is "not available", never an error.

use super::explain::{explain, fmt_formula, parse_formula, prep_ms2, Explanation, Formula};
use super::isotopes::{pattern, round_to};
use super::libraries::{LibraryIndex, Ref};
use super::matching::{compare, delta_formulas, match_spectra, Ms2};
use super::proposal::{
    Alternative, Best, Evidence, Proposal, PurityEvidence, RawEvidence, Reference, ToolRef,
};
use super::propose::{propose, MgfBlock, ProposeInput};
use super::purity::{
    badge, candidate_scores, impossible_losses, isolation_purity, low_information, unexplained,
    CandidateScores, CoelRow,
};
use super::raw::{best_scan, coelution, ms1_metrics, RawSource, ScanInfo};
use super::settings::ReviewSettings;
use super::sources::{GnpsHit, SiriusHit};
use super::structures::{structures_of, Structures};
use super::tsv::Cell;
use crate::attributio::annotate::Feature;
use std::collections::HashMap;

/// What the review reads from the outside.
pub struct Env<'a, R: RawSource, S: Structures> {
    pub s: &'a ReviewSettings,
    pub lib: &'a LibraryIndex,
    pub st: &'a S,
    pub raw: &'a R,
}

/// The inputs of one feature: its MS2 block and the hits the pipeline tools gave it.
#[derive(Default)]
pub struct Inputs<'a> {
    pub mgf: Option<&'a MgfBlock>,
    pub sirius: Vec<&'a SiriusHit>,
    pub gnps: Vec<&'a GnpsHit>,
}

fn text(c: &Cell) -> Option<String> {
    match c {
        Cell::Str(s) if !s.trim().is_empty() && s.to_lowercase() != "nan" => Some(s.clone()),
        _ => None,
    }
}

fn opt_text(s: &Option<String>) -> Option<String> {
    s.as_ref()
        .filter(|s| !s.trim().is_empty() && s.to_lowercase() != "nan")
        .cloned()
}

/// A candidate structure (or just a formula) with the explanation of the spectrum it gives.
struct Cand {
    label: &'static str,
    smiles: Option<String>,
    fd: Option<Formula>,
    ex: Explanation,
    fits: bool,
    key: (bool, bool, f64),
}

fn candidates<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    f: &Feature,
    exp: (&[f64], &[f64]),
) -> Vec<Cand> {
    let (s, mz) = (env.s, f.mz.unwrap_or(f64::NAN));
    let library_first = f.level == 2
        || [
            "expert review",
            "isomers not discriminated",
            "name conflict",
            "diagnostic evidence",
        ]
        .contains(&f.level_basis.as_str());
    let mut c: Vec<Cand> = Vec::new();
    for (label, smi) in structures_of(f) {
        let Some(fd) = env.st.formula(&smi) else {
            continue;
        };
        let ex = explain(Some(&fd), mz, &f.pol, exp, s);
        let fits = ex.fits(s);
        let key = (fits, (label == "library") == library_first, ex.frac_expl);
        c.push(Cand {
            label,
            smiles: Some(smi),
            fd: Some(fd),
            ex,
            fits,
            key,
        });
    }
    if c.is_empty() {
        let fd = [&f.csi.formula, &f.mzlib.formula]
            .iter()
            .find_map(|x| opt_text(x))
            .map(|t| parse_formula(&t));
        let ex = explain(fd.as_ref(), mz, &f.pol, exp, s);
        c.push(Cand {
            label: "formula",
            smiles: None,
            fd,
            ex,
            fits: false,
            key: (false, false, 0.0),
        });
    }
    c.sort_by(|a, b| {
        b.key.0.cmp(&a.key.0).then(b.key.1.cmp(&a.key.1)).then(
            b.key
                .2
                .partial_cmp(&a.key.2)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });
    c
}

/// A reference being collected: a spectrum of the libraries (with its row) or a hit of a tool whose spectrum is not in them.
struct RefIn {
    origin: &'static str,
    lib: Option<Ref>,
    sc: Option<(f64, f64, usize, usize, usize)>,
    tools: Vec<ToolHit>,
    usi_id: Option<String>,
}

#[derive(Clone)]
struct ToolHit {
    tool: &'static str,
    kind: String,
    ref_id: Option<String>,
    name: Option<String>,
    score: Option<f64>,
    shared: Option<f64>,
    ref_prec: Option<f64>,
    adduct: Option<String>,
    ik1: Option<String>,
    smiles: Option<String>,
    library: Option<String>,
    entry: Option<String>,
}

fn ik1_of(inchikey: &str) -> Option<String> {
    let k = inchikey.trim().to_uppercase();
    (k.chars().count() >= 14).then(|| k.chars().take(14).collect())
}

/// The tool hits of a feature in the order of the reference tool: SIRIUS, GNPS2, mzMine.
fn tool_hits(f: &Feature, inp: &Inputs, prec: f64, s: &ReviewSettings) -> Vec<ToolHit> {
    let mut out = Vec::new();
    for h in &inp.sirius {
        out.push(ToolHit {
            tool: "SIRIUS",
            kind: h.kind.to_string(),
            ref_id: Some(h.ref_id.clone()).filter(|r| !r.is_empty()),
            name: text(&h.name),
            score: h.similarity.map(|v| round_to(v, 3)),
            shared: h.shared,
            ref_prec: h.ref_prec,
            adduct: h.adduct.clone(),
            ik1: h.ik1.clone(),
            smiles: text(&h.smiles),
            library: None,
            entry: None,
        });
    }
    for h in &inp.gnps {
        let exact = h
            .mass_diff
            .is_none_or(|m| m.abs() <= prec * s.exact_ppm * 1e-6);
        out.push(ToolHit {
            tool: "GNPS2",
            kind: if exact { "exact" } else { "analog" }.to_string(),
            ref_id: text(&h.ref_id),
            name: Some(h.name.clone()),
            score: h.score.map(|v| round_to(v, 3)),
            shared: h.shared,
            ref_prec: h.ref_prec,
            adduct: text(&h.adduct),
            ik1: None,
            smiles: text(&h.smiles),
            library: text(&h.library),
            entry: None,
        });
    }
    if let Some(name) = opt_text(&f.mzlib.name) {
        out.extend(mzmine_hits(f, name));
    }
    out
}

fn mzmine_hits(f: &Feature, name: String) -> Vec<ToolHit> {
    let m = &f.mzlib;
    let ik1 = m
        .inchikey
        .as_deref()
        .and_then(|k| Some(k.chars().take(14).collect::<String>()).filter(|k| !k.is_empty()));
    let mk = |id: Option<String>, entry: Option<String>| ToolHit {
        tool: "mzMine",
        kind: "exact".to_string(),
        ref_id: id,
        name: Some(name.clone()),
        score: m.score.map(|v| round_to(v, 3)),
        shared: m.peaks,
        ref_prec: m.precursor_mz,
        adduct: opt_text(&m.adduct),
        ik1: ik1.clone(),
        smiles: opt_text(&m.smiles),
        library: None,
        entry,
    };
    // the entry and the USI of the hit are not columns of the typed evidence: the name stands for both
    vec![mk(None, None)]
}

/// The experimental spectrum as the page holds it (rounded m/z, intensities to four digits).
fn quantised(mz: &[f64], it: &[f64], decimals: i32) -> (Vec<f64>, Vec<f64>) {
    (
        mz.iter().map(|&m| round_to(m, decimals)).collect(),
        it.iter().map(|&v| super::purity::sig4(v)).collect(),
    )
}

/// The reference peaks written into the page: at least `rel_min` of the base peak (taken below the precursor − 0.5, as the cosine does).
fn page_peaks(mz: &[f64], it: &[f64], prec: f64, rel_min: f64) -> Vec<usize> {
    let below: Vec<f64> = (0..mz.len())
        .filter(|&i| mz[i] < prec - 0.5)
        .map(|i| it[i])
        .collect();
    let base = if below.is_empty() {
        it.iter().cloned().fold(0.0, f64::max)
    } else {
        below.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    };
    (0..mz.len()).filter(|&i| it[i] >= rel_min * base).collect()
}

fn same_structure(my_ik: &Option<String>, ik1: &Option<String>) -> bool {
    matches!((my_ik, ik1), (Some(a), Some(b)) if a == b)
}

/// The library references of the feature: exact (best `top_n`, same structure first at equal cosine) and analogs (best `top_n`).
fn library_refs<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    pol: &str,
    exp: &Ms2,
    low: Option<f64>,
    fadd: Option<&str>,
    my_ik: &Option<String>,
) -> Vec<RefIn> {
    let s = env.s;
    let mut exact: Vec<(super::matching::Comparison, Ref)> = env
        .lib
        .exact(exp.prec, pol, s.precursor_ppm, fadd)
        .into_iter()
        .map(|r| (compare(exp, &env.lib.ms2(r), s.tol_da, low, 0.0), r))
        .collect();
    exact.sort_by(|a, b| {
        b.0.cos_vis
            .total_cmp(&a.0.cos_vis)
            .then(b.0.cos.total_cmp(&a.0.cos))
    });
    let mut seen: Vec<String> = Vec::new();
    let mut refs: Vec<RefIn> = Vec::new();
    for (c, r) in exact {
        let entry = env.lib.record(r).entry.clone();
        if !entry.is_empty() && seen.contains(&entry) {
            continue;
        }
        seen.push(entry);
        refs.push(RefIn {
            origin: "library",
            lib: Some(r),
            sc: Some((c.cos_vis, c.cos, 0, 0, 0)),
            tools: vec![],
            usi_id: None,
        });
    }
    refs.sort_by(|a, b| {
        let key = |x: &RefIn| {
            let r = x.lib.expect("library reference");
            (
                -round_to(x.sc.map_or(0.0, |c| c.0), 3),
                !(my_ik.is_some() && ik1_of(&env.lib.record(r).inchikey) == *my_ik),
            )
        };
        let (ka, kb) = (key(a), key(b));
        ka.0.total_cmp(&kb.0).then(ka.1.cmp(&kb.1))
    });
    refs.truncate(s.top_n as usize);
    let mut analogs: Vec<(f64, f64, RefIn)> = Vec::new();
    for r in env.lib.analog_candidates(
        exp,
        pol,
        s.analog_window_da,
        s.analog_prefilter as usize,
        s.exact_ppm,
    ) {
        let dm = exp.prec - env.lib.record(r).prec;
        if dm.abs() < s.analog_min_shift_da {
            continue;
        }
        let c = compare(exp, &env.lib.ms2(r), s.tol_da, low, dm);
        analogs.push((
            c.cos_vis,
            c.cos,
            RefIn {
                origin: "library-analog",
                lib: Some(r),
                sc: None,
                tools: vec![],
                usi_id: None,
            },
        ));
    }
    analogs.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.total_cmp(&a.1)));
    refs.extend(analogs.into_iter().take(s.top_n as usize).map(|a| a.2));
    refs
}

/// Adds the tool hits to the references: a hit on a spectrum already listed joins it; else the spectrum is looked up by entry in the libraries.
fn add_tool_refs(lib: &LibraryIndex, refs: &mut Vec<RefIn>, hits: Vec<ToolHit>) -> Vec<ToolHit> {
    let mut by_entry: HashMap<String, usize> = HashMap::new();
    for (i, r) in refs.iter().enumerate() {
        if let Some(x) = r
            .lib
            .map(|x| &lib.record(x).entry)
            .filter(|e| !e.is_empty())
        {
            by_entry.entry(x.clone()).or_insert(i);
        }
    }
    let mut kept = Vec::new();
    for h in hits {
        let rid = h.ref_id.clone();
        if let Some(&i) = rid.as_ref().and_then(|r| by_entry.get(r)) {
            refs[i].tools.push(h.clone());
            kept.push(h);
            continue;
        }
        let found = rid
            .as_deref()
            .and_then(|r| lib.find_entry(r))
            .or_else(|| h.entry.as_deref().and_then(|e| lib.find_entry(e)));
        let origin = RefIn {
            origin: "tool",
            lib: found,
            sc: None,
            tools: vec![h.clone()],
            usi_id: rid,
        };
        if let Some(r) = found {
            by_entry.insert(lib.record(r).entry.clone(), refs.len());
        }
        refs.push(origin);
        kept.push(h);
    }
    kept
}

pub(super) struct Scored {
    pub(super) r: Reference,
    pub(super) spec: Option<Ms2>,
}

/// Scores every reference on the spectra as the page holds them and orders them: exact first, then by cos_vis, same structure first.
fn finish_refs<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    refs: Vec<RefIn>,
    exp_q: &Ms2,
    low: Option<f64>,
    my_ik: &Option<String>,
    explained: &[bool],
) -> Vec<Scored> {
    let s = env.s;
    let prec = exp_q.prec;
    let mut out: Vec<Scored> = Vec::new();
    for c in refs {
        let t0 = c.tools.first().cloned();
        let (rec, meta) = match c.lib {
            Some(r) => (Some(env.lib.ms2(r)), Some(env.lib.record(r).clone())),
            None => (None, None),
        };
        let rprec = meta
            .as_ref()
            .map(|m| m.prec)
            .or(t0.as_ref().and_then(|t| t.ref_prec))
            .filter(|p| p.is_finite() && *p != 0.0)
            .or((t0.as_ref().map_or("exact", |t| t.kind.as_str()) == "exact").then_some(prec));
        let dm = rprec.map_or(0.0, |p| prec - p);
        let analog = rprec.is_some()
            && dm.abs() > prec * s.exact_ppm * 1e-6
            && dm.abs() >= s.analog_min_shift_da;
        let smiles = meta
            .as_ref()
            .map(|m| m.smiles.clone())
            .filter(|x| !x.is_empty())
            .or(t0.as_ref().and_then(|t| t.smiles.clone()));
        let ik1 = meta
            .as_ref()
            .and_then(|m| ik1_of(&m.inchikey))
            .or(t0.as_ref().and_then(|t| t.ik1.clone()))
            .or_else(|| smiles.as_deref().and_then(|x| env.st.ik1(x)));
        let mut r = Reference {
            origin: c.origin.to_string(),
            library: match (c.lib, t0.as_ref()) {
                (Some(x), _) => Some(env.lib.files[x.file].label.clone()),
                (None, Some(t)) => Some(match t.tool {
                    "SIRIUS" => "SIRIUS database (by USI)".to_string(),
                    "GNPS2" => t
                        .library
                        .clone()
                        .unwrap_or_else(|| "GNPS library (by USI)".to_string()),
                    "mzMine" => "mzMine library".to_string(),
                    other => other.to_string(),
                }),
                _ => None,
            },
            entry: meta
                .as_ref()
                .map(|m| m.entry.clone())
                .filter(|e| !e.is_empty())
                .or(c.usi_id.clone()),
            name: meta
                .as_ref()
                .map(|m| m.name.clone())
                .filter(|n| !n.is_empty())
                .or(t0.as_ref().and_then(|t| t.name.clone()))
                .unwrap_or_else(|| "(unnamed)".to_string()),
            same: same_structure(my_ik, &ik1),
            ik1,
            smiles,
            prec: rprec.map(|p| round_to(p, 5)),
            adduct: meta
                .as_ref()
                .map(|m| m.adduct.clone())
                .filter(|a| !a.is_empty())
                .or(t0.as_ref().and_then(|t| t.adduct.clone())),
            kind: if analog { "analog" } else { "exact" }.to_string(),
            dm: analog.then(|| round_to(dm, 5)),
            dm_ppm: rprec.map(|p| round_to(dm / p * 1e6, 1)),
            dm_formulas: if analog {
                delta_formulas(dm, s.delta_formula_mda, 5)
                    .into_iter()
                    .map(|d| d.label)
                    .collect()
            } else {
                vec![]
            },
            tools: c
                .tools
                .iter()
                .map(|t| ToolRef {
                    tool: t.tool.to_string(),
                    kind: t.kind.clone(),
                    score: t.score,
                    shared: t.shared,
                    ref_id: t.ref_id.clone(),
                })
                .collect(),
            cos: None,
            cos_vis: None,
            matched: 0,
            matched_vis: 0,
            n_shifted: 0,
            available: rec.is_some(),
            scores: None,
        };
        let mut spec = None;
        if let (Some(m), false) = (rec.as_ref(), exp_q.mz.is_empty()) {
            let rp = rprec.unwrap_or(prec);
            let keep = page_peaks(&m.mz, &m.it, rp, 0.01);
            let mut o = keep;
            o.sort_by(|&a, &b| m.mz[a].total_cmp(&m.mz[b]));
            let q = quantised(
                &o.iter().map(|&i| m.mz[i]).collect::<Vec<_>>(),
                &o.iter().map(|&i| m.it[i]).collect::<Vec<_>>(),
                s.ref_decimals as i32,
            );
            let rq = Ms2 {
                mz: q.0,
                it: q.1,
                prec: rp,
            };
            let shift = if analog { dm } else { 0.0 };
            let sc = compare(exp_q, &rq, s.tol_da, low, shift);
            r.cos = Some(round_to(sc.cos, 4));
            r.cos_vis = Some(round_to(sc.cos_vis, 4));
            r.matched = sc.matched as i64;
            r.matched_vis = sc.matched_vis as i64;
            r.n_shifted = sc.n_shifted as i64;
            r.scores = Some(candidate_scores(
                exp_q, &rq, s.tol_da, low, shift, explained,
            ));
            spec = Some(rq);
        }
        out.push(Scored { r, spec });
    }
    let mut seen: Vec<(String, String)> = Vec::new();
    out.retain(|x| {
        let k = (
            x.r.entry.clone().unwrap_or_default() + &x.r.name,
            x.r.kind.clone(),
        );
        let new = !seen.contains(&k);
        seen.push(k);
        new
    });
    out.sort_by(|a, b| {
        let ka = (a.r.kind != "exact", -a.r.cos_vis.unwrap_or(-1.0), !a.r.same);
        let kb = (b.r.kind != "exact", -b.r.cos_vis.unwrap_or(-1.0), !b.r.same);
        ka.0.cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(ka.2.cmp(&kb.2))
    });
    out
}

/// The MS2 peaks without the precursor's own 13C isotopologues.
fn without_precursor_isotopes(mz: &[f64], it: &[f64], prec: f64) -> (Vec<f64>, Vec<f64>) {
    let keep: Vec<usize> = (0..mz.len())
        .filter(|&i| {
            (1..=3).all(|k| {
                (mz[i] - (prec + k as f64 * 1.0033548)).abs() > f64::max(0.005, prec * 1e-5)
            })
        })
        .collect();
    (
        keep.iter().map(|&i| mz[i]).collect(),
        keep.iter().map(|&i| it[i]).collect(),
    )
}

fn raw_evidence<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    blk: &MgfBlock,
    f: &Feature,
    ion: Option<&Formula>,
) -> Option<(RawEvidence, Option<super::raw::ScanInfo>)> {
    if blk.srcs.is_empty() {
        return None;
    }
    let rt = round_to(f.rt.unwrap_or(f64::NAN), 3);
    let (file, scan) = best_scan(env.raw, &blk.srcs, rt, &[])?;
    let info = env.raw.scan_info(&file, scan)?;
    let ms1 = ms1_metrics(Some(&info), f.mz.unwrap_or(f64::NAN), ion, env.s);
    let ev = RawEvidence {
        file,
        scan,
        low: info.low,
        high: info.high,
        target: info.target,
        lo: info.lo,
        hi: info.hi,
        ms1,
    };
    Some((ev, Some(info)))
}

/// The purity block: isolation window, co-elution, unexplained signal, impossible losses, the three-state badge.
#[allow(clippy::too_many_arguments)]
fn purity_block<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    f: &Feature,
    ion: Option<&Formula>,
    info: Option<&ScanInfo>,
    scan: Option<(&str, i64)>,
    exp_q: &Ms2,
    explained: &[bool],
    refs: &[Scored],
    imposs: Vec<super::purity::Impossible>,
) -> Option<PurityEvidence> {
    let s = env.s;
    let mz = f.mz.unwrap_or(f64::NAN);
    let iso = info.filter(|i| !i.ms1_mz.is_empty()).and_then(|i| {
        let sticks = ion.map(|x| pattern(x, &f.pol, 1e-3));
        isolation_purity(
            &i.ms1_mz,
            &i.ms1_it,
            mz,
            i.target,
            i.lo,
            i.hi,
            sticks.as_deref().filter(|v| !v.is_empty()),
            &f.pol,
            10.0,
            0.10,
        )
    });
    let best = refs
        .iter()
        .find(|x| x.r.scores.is_some() && x.r.kind == "exact")
        .or_else(|| refs.iter().find(|x| x.r.scores.is_some()));
    let unexpl = if exp_q.mz.is_empty() {
        None
    } else if let Some(b) = best.and_then(|b| b.spec.as_ref().map(|sp| (b, sp))) {
        let shift = b.0.r.dm.unwrap_or(0.0);
        let pairs = match_spectra(exp_q, b.1, s.tol_da, shift, None).pairs;
        let mut matched = vec![false; exp_q.mz.len()];
        pairs.iter().for_each(|p| matched[p.0] = true);
        Some(round_to(
            unexplained(&exp_q.it, explained, Some(&matched)),
            3,
        ))
    } else {
        Some(round_to(unexplained(&exp_q.it, explained, None), 3))
    };
    let coel: Vec<CoelRow> = iso
        .as_ref()
        .map_or(vec![], |i| coelution_rows(env, i, scan, mz));
    let bq: Option<&CandidateScores> = best.and_then(|b| b.r.scores.as_ref());
    let b = badge(iso.as_ref(), &coel, unexpl, &imposs, bq);
    let li = low_information(bq, 3);
    (iso.is_some() || !coel.is_empty() || unexpl.is_some() || !imposs.is_empty()).then(|| {
        PurityEvidence {
            purity: iso.as_ref().and_then(|i| i.purity),
            prec_int: iso.as_ref().map(|i| i.prec_int),
            co: coel
                .iter()
                .map(|c| (c.mz, c.rel, "contaminant".to_string(), c.offset, c.r))
                .collect(),
            unexplained: unexpl,
            imposs,
            badge: b.state.to_string(),
            badge_reasons: b.reasons,
            low_information: li.map(|l| l.low),
        }
    })
}

/// Co-isolated contaminants with the apex offset and correlation of their XIC against the precursor's (none when the raw data gives no XIC).
fn coelution_rows<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    iso: &super::purity::IsolationPurity,
    scan: Option<(&str, i64)>,
    mz: f64,
) -> Vec<CoelRow> {
    let cont: Vec<&super::purity::CoIon> =
        iso.co.iter().filter(|c| c.kind == "contaminant").collect();
    let xic = scan.and_then(|(file, sc)| {
        let rt = env.raw.scan_rt(file, sc)?;
        let mzs: Vec<f64> = std::iter::once(mz)
            .chain(cont.iter().map(|c| c.mz))
            .collect();
        env.raw.xic(file, rt, env.s.coelution_half_min, &mzs)
    });
    cont.iter()
        .enumerate()
        .map(|(k, c)| {
            let ce = xic
                .as_ref()
                .and_then(|(rt, tr)| coelution(rt, tr.first()?, tr.get(k + 1)?));
            CoelRow {
                mz: c.mz,
                rel: c.rel,
                r: ce.as_ref().map(|x| x.r),
                offset: ce.map(|x| x.offset),
            }
        })
        .collect()
}

fn alternatives(c: &[Cand], st: &impl Structures) -> Vec<Alternative> {
    c.iter()
        .map(|c| Alternative {
            label: c.label.to_string(),
            formula: c.fd.as_ref().map(fmt_formula),
            adduct: c.ex.adduct.clone(),
            ppm: Some(c.ex.ion_ppm)
                .filter(|p| p.is_finite())
                .map(|p| round_to(p, 2)),
            n_expl: c.ex.n_expl,
            frac: round_to(c.ex.frac_expl, 3),
            fits: c.fits,
            ik1: c.smiles.as_deref().and_then(|x| st.ik1(x)),
        })
        .collect()
}

/// The proposal of the review for one feature.
pub fn review_feature<R: RawSource, S: Structures>(
    env: &Env<R, S>,
    f: &Feature,
    inp: &Inputs,
) -> Proposal {
    let s = env.s;
    let mz = f.mz.unwrap_or(f64::NAN);
    let (e_mz, e_it) = match inp.mgf {
        Some(b) => {
            let (m, i, _) = prep_ms2(&b.mz, &b.it, b.prec, s.rel_min);
            (m, i)
        }
        None => (vec![], vec![]),
    };
    let prec = inp.mgf.map_or(mz, |b| b.prec);
    let mut cands = candidates(env, f, (&e_mz, &e_it));
    let prim = cands.remove(0);
    let ex = &prim.ex;
    let my_ik = prim
        .smiles
        .as_deref()
        .and_then(|x| env.st.ik1(x))
        .or_else(|| opt_text(&f.inchikey_skeleton));
    let raw = inp
        .mgf
        .and_then(|b| raw_evidence(env, b, f, ex.ion.as_ref()));
    let low = raw.as_ref().map(|r| r.0.low);
    let (exp_mz, exp_it) = quantised(&e_mz, &e_it, s.exp_decimals as i32);
    let exp_q = Ms2 {
        mz: exp_mz,
        it: exp_it,
        prec,
    };
    let explained = ex.explained_mask();
    let mut refs: Vec<RefIn> = Vec::new();
    if !e_mz.is_empty() {
        let fadd = ex.fits(s).then(|| ex.adduct.clone()).flatten();
        refs = library_refs(
            env,
            &f.pol,
            &Ms2 {
                mz: e_mz.clone(),
                it: e_it.clone(),
                prec,
            },
            low,
            fadd.as_deref(),
            &my_ik,
        );
    }
    add_tool_refs(env.lib, &mut refs, tool_hits(f, inp, prec, s));
    let scored = finish_refs(env, refs, &exp_q, low, &my_ik, &explained);
    let imposs = match inp.mgf {
        Some(b) => {
            let (um, ui) = without_precursor_isotopes(&b.mz, &b.it, prec);
            let above: Vec<usize> = (0..um.len()).filter(|&i| um[i] > prec + 0.5).collect();
            let mz_all: Vec<f64> = e_mz
                .iter()
                .cloned()
                .chain(above.iter().map(|&i| um[i]))
                .collect();
            let it_all: Vec<f64> = e_it
                .iter()
                .cloned()
                .chain(above.iter().map(|&i| ui[i]))
                .collect();
            let base = e_it.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            impossible_losses(
                &mz_all,
                &it_all,
                prec,
                0.05,
                0.005,
                Some(if base.is_finite() { base } else { 1.0 }),
            )
        }
        None => vec![],
    };
    let info = raw.as_ref().and_then(|r| r.1.as_ref());
    let purity = purity_block(
        env,
        f,
        ex.ion.as_ref(),
        info,
        raw.as_ref().map(|r| (r.0.file.as_str(), r.0.scan)),
        &exp_q,
        &explained,
        &scored,
        imposs,
    );
    let exact: Vec<&Scored> = scored
        .iter()
        .filter(|c| c.r.kind == "exact" && c.r.cos_vis.is_some())
        .collect();
    let same = exact.iter().find(|c| c.r.same);
    let others: Vec<&&Scored> = exact.iter().filter(|c| !c.r.same).collect();
    let m1 = raw.as_ref().and_then(|r| r.0.ms1.as_ref());
    let pk = [f.spec.peaks, f.gnps.peaks, f.mzlib.peaks]
        .into_iter()
        .flatten()
        .fold(f64::NEG_INFINITY, f64::max);
    let r3 = |v: f64| Some(round_to(v, 3)).filter(|x| x.is_finite());
    let pi = ProposeInput {
        level: f.level,
        level_sub: f.level_sub.clone(),
        adduct: ex.adduct.clone(),
        ion_ppm: round_to(ex.ion_ppm, 2),
        n_expl: ex.n_expl as i64,
        n_peaks: e_mz.len() as i64,
        frac_expl: ex.frac_expl,
        decoy_frac: ex.decoy_frac,
        level_basis: f.level_basis.clone(),
        lib_peaks: if pk.is_finite() { pk as i64 } else { 0 },
        iso_ok: m1.and_then(|m| m.iso_ok),
        obs1: m1.and_then(|m| r3(m.obs1)),
        th1: m1.and_then(|m| r3(m.th1)),
        obs2: m1.and_then(|m| r3(m.obs2)),
        th2: m1.and_then(|m| r3(m.th2)),
        chim_n: m1.map(|m| m.chim_n as i64),
        chim_max: m1.and_then(|m| r3(m.chim_max)),
        ref_cos_vis: same.and_then(|c| c.r.cos_vis),
        ref_matched_vis: same.map(|c| c.r.matched_vis),
        other_best: others.first().and_then(|c| c.r.cos_vis),
        other_refs: others
            .iter()
            .take(2)
            .map(|c| {
                format!(
                    "{} ({}) cos_vis {:.2}/{}",
                    c.r.name,
                    c.r.library
                        .clone()
                        .unwrap_or_else(|| "mzMine library".to_string()),
                    c.r.cos_vis.unwrap_or(0.0),
                    c.r.matched_vis
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
    };
    let t = propose(&pi, s);
    let n_raw = inp.mgf.map_or(0, |b| b.mz.len());
    let pim = inp
        .mgf
        .map(|b| super::propose::precursor_in_ms2(&b.mz, &b.it, prec, 10.0, 0.01));
    Proposal {
        pol: f.pol.clone(),
        id: f.id,
        class: t.tier.to_string(),
        proposed: t.proposed.to_string(),
        reasons: t.reasons,
        best: Best {
            structure_from: prim.label.to_string(),
            formula: prim.fd.as_ref().map(fmt_formula),
            smiles: prim.smiles.clone(),
            ik1: my_ik,
            adduct: ex.adduct.clone(),
            ion_formula: ex.ion_formula.clone(),
            ion_ppm: Some(ex.ion_ppm)
                .filter(|p| p.is_finite())
                .map(|p| round_to(p, 2)),
        },
        alternatives: alternatives(&cands, env.st),
        evidence: Evidence {
            n_peaks: e_mz.len(),
            n_raw_peaks: n_raw,
            n_explained: ex.n_expl,
            frac_explained: round_to(ex.frac_expl, 3),
            decoy_frac: round_to(ex.decoy_frac, 3),
            precursor_present: pim.as_ref().is_some_and(|p| p.present),
            precursor_rel: pim.and_then(|p| p.prec_rel),
            raw: raw.map(|r| r.0),
            view: Some(super::view::build(
                &e_mz, &e_it, ex, &exp_q, &scored, low, env.s, &f.pol,
            )),
            references: scored.into_iter().map(|x| x.r).collect(),
            purity,
        },
    }
}
