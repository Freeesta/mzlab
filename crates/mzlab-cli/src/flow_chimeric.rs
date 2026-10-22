//! The native executor of `spectra.chimeric@1`: for every MS2 spectrum of an MGF, the purity of its precursor in the MS1 scans around it (`fill_purity`,
//! the one of `tp.link`) and, with a feature table, the features co-isolated in its window (`ms2_engine::chimeric::intruders`). The spectrum is written
//! as it came, with the header lines `PURITY`, `PURITY_BAND`, `CHIMERIC`, `INTRUDERS` and, for the `ms1_window` algorithm, `MS1_COISOLATED*` added; nothing is removed from it.

use std::fmt::Write as _;

use crate::flow::{all_inputs, io, read_all, read_one, Exec, FileRunner};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::attributio::ms2_engine::chimeric::{default_window, intruders, FeaturePoint};
use mzlab_feat::attributio::ms2_engine::deconv::{deconvolve, Candidate};
use mzlab_feat::attributio::review::raw::Ms1Metrics;
use mzlab_feat::interp::ms2link::{
    fill_purity_with, ms1_window_metrics, LinkParams, Ms2Scan, PURITY_MIN,
};
use mzlab_feat::legacy::ms2::Ms2Index;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_lib::spec::Spectrum;

use crate::flow_match::{library_spectra, Named};

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("spectra.chimeric", Exec::File("spectra", chimeric_job))];

/// The MS2 scan of the index file a spectrum comes from is found by its RT (a 3-decimal second in the MGF) and the centre of its window.
const RT_TOL_MIN: f64 = 2e-4;
const CENTRE_TOL_DA: f64 = 0.6;

/// What the header of one MGF block says about its spectrum.
struct Block<'a> {
    lines: Vec<&'a str>,
    prec: f64,
    rt: f64,
    polarity: i8,
    feature: Option<i64>,
}

fn parse(text: &str) -> (Vec<&str>, Vec<Block<'_>>) {
    let (mut loose, mut blocks, mut cur) = (vec![], vec![], None::<Vec<&str>>);
    for line in text.lines() {
        match (line.trim(), cur.as_mut()) {
            ("BEGIN IONS", _) => cur = Some(vec![]),
            ("END IONS", Some(_)) => blocks.push(block(cur.take().unwrap_or_default())),
            (_, Some(l)) => l.push(line),
            (_, None) => loose.push(line),
        }
    }
    (loose, blocks)
}

fn block(lines: Vec<&str>) -> Block<'_> {
    let get = |key: &str| lines.iter().find_map(|l| l.strip_prefix(key));
    let prec = get("PEPMASS=").and_then(|v| v.split_whitespace().next()?.parse().ok());
    let rt: Option<f64> = get("RTINSECONDS=").and_then(|v| v.trim().parse().ok());
    let polarity = match get("IONMODE=").map(str::trim) {
        Some(m) if m.eq_ignore_ascii_case("negative") => -1,
        _ => 1,
    };
    let feature = get("FEATURE_ID=")
        .or_else(|| get("ACCESSION="))
        .and_then(|v| v.trim().parse().ok());
    Block {
        prec: prec.unwrap_or(f64::NAN),
        rt: rt.map_or(f64::NAN, |s| s / 60.0),
        polarity,
        feature,
        lines,
    }
}

/// Purity under which a spectrum is in the `low` band, whatever the chimeric limit.
const LOW_PURITY: f64 = 0.5;

fn param(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// What the MS1 around every block says: the purity of its precursor (`None` where no index file has the scan or no MS1 lies around it) and, with
/// `chim_rel` (the `ms1_window` algorithm), the ions co-isolated in its window.
/// The same two lists, borrowed.
type Seen<'a> = (&'a [Option<(f64, bool)>], &'a [Option<Ms1Metrics>]);

type Found = (Vec<Option<(f64, bool)>>, Vec<Option<Ms1Metrics>>);

fn purities(r: &FileRunner, job: &Job, blocks: &[Block], chim_rel: Option<f64>) -> Result<Found> {
    let paths = all_inputs(r, job, "index");
    let mut files = paths
        .iter()
        .map(crate::flow_base::open)
        .collect::<Result<Vec<_>>>()?;
    let ms2: Vec<Ms2Index> = files.iter().map(Ms2Index::of).collect();
    // block -> (file, scan of the file)
    let mut found: Vec<Option<(usize, usize)>> = vec![None; blocks.len()];
    for (b, f) in blocks.iter().zip(found.iter_mut()) {
        *f = ms2.iter().enumerate().find_map(|(k, m)| {
            let i = (0..m.scan.len()).find(|&i| {
                (m.rt[i] - b.rt).abs() <= RT_TOL_MIN
                    && (m.center[i] - b.prec).abs() <= CENTRE_TOL_DA
            })?;
            Some((k, m.scan[i]))
        });
    }
    let mut out = vec![None; blocks.len()];
    let mut ms1 = vec![None; blocks.len()];
    for (k, file) in files.iter_mut().enumerate() {
        for polarity in [1i8, -1] {
            let mut mine: Vec<usize> = (0..blocks.len())
                .filter(|&i| found[i].is_some_and(|f| f.0 == k) && blocks[i].polarity == polarity)
                .collect();
            mine.sort_unstable();
            let mut scans: Vec<Ms2Scan> = mine
                .iter()
                .map(|&i| {
                    let g = found[i].map_or(0, |f| f.1);
                    let s = &file.scans[g];
                    Ms2Scan {
                        file: k,
                        scan: g,
                        rt: s.rt,
                        precursor: blocks[i].prec,
                        iso: s.iso.unwrap_or((blocks[i].prec, blocks[i].prec)),
                        ce: None,
                        scan_range: None,
                        purity: f64::NAN,
                        chimeric: false,
                        mz: vec![],
                        it: vec![],
                    }
                })
                .collect();
            if scans.is_empty() {
                continue;
            }
            let link = LinkParams {
                purity_min: param(job, "purity_min").unwrap_or(PURITY_MIN),
                ..LinkParams::default()
            };
            fill_purity_with(file, polarity, &mut scans, &link)?;
            for (&i, s) in mine.iter().zip(&scans) {
                out[i] = Some((s.purity, s.chimeric));
            }
            if let Some(rel) = chim_rel {
                for (&i, m) in mine
                    .iter()
                    .zip(ms1_window_metrics(file, polarity, &scans, rel)?)
                {
                    ms1[i] = m;
                }
            }
        }
    }
    Ok((out, ms1))
}

/// The feature rows of the `.mzfeat` as points: id, m/z and RT (min).
fn points(table: &[u8]) -> Result<Vec<FeaturePoint>> {
    if table.first() == Some(&b'{') {
        return Err(Error::new("err.flow.export.view").with("port", "features"));
    }
    let (t, _) = read_table_from(&mut Slice(table), table.len() as u64)?;
    Ok((0..t.len())
        .map(|r| FeaturePoint {
            pol: "POS".into(),
            id: t.id(r) as i64,
            mz: t.mz[r],
            rt: t.rt[r],
        })
        .collect())
}

/// The MGF with the header lines added; blocks and loose lines keep their order.
pub(crate) fn chimeric_text(
    (window, rt_tol): (f64, f64),
    spectra: &[u8],
    (pure, ms1): Seen,
    chim_cap: f64,
    extra: &[String],
    table: Option<&[FeaturePoint]>,
) -> String {
    let text = String::from_utf8_lossy(spectra);
    let (loose, blocks) = parse(&text);
    let mut out = String::new();
    for l in loose.iter().filter(|l| l.starts_with('#')) {
        let _ = writeln!(out, "{l}");
    }
    for (n, (b, p)) in blocks.iter().zip(pure).enumerate() {
        out.push_str("BEGIN IONS\n");
        if let Some(Some(m)) = ms1.get(n) {
            let list: Vec<String> = m
                .chim
                .iter()
                .map(|(mz, rel)| format!("{mz:.4}:{rel:.3}"))
                .collect();
            let _ = writeln!(out, "MS1_COISOLATED={}", list.join(";"));
            let _ = writeln!(out, "MS1_COISOLATED_N={}", m.chim_n);
            let _ = writeln!(out, "MS1_CAPPED={}", m.chim_max >= chim_cap);
        }
        if let Some(lines) = extra.get(n) {
            out.push_str(lines);
        }
        if let Some(&(purity, chimeric)) = p.as_ref() {
            if purity.is_finite() {
                let _ = writeln!(out, "PURITY={purity:.4}");
                let band = match purity {
                    p if p < LOW_PURITY => "low",
                    _ if chimeric => "medium",
                    _ => "high",
                };
                let _ = writeln!(out, "PURITY_BAND={band}");
            }
            let _ = writeln!(out, "CHIMERIC={chimeric}");
        }
        if let Some(all) = table.filter(|_| b.prec.is_finite() && b.rt.is_finite()) {
            let target = FeaturePoint {
                pol: "POS".into(),
                id: b.feature.unwrap_or(-1),
                mz: b.prec,
                rt: b.rt,
            };
            let list: Vec<String> = intruders(all, &target, window, rt_tol)
                .iter()
                .map(|x| format!("{}:{}:{}", x.id, x.delta_mz, x.delta_rt))
                .collect();
            let _ = writeln!(out, "INTRUDERS={}", list.join(";"));
        }
        for l in &b.lines {
            let _ = writeln!(out, "{l}");
        }
        out.push_str("END IONS\n");
    }
    out
}

/// `DECONV=name:share;…` and `DECONV_RESIDUAL=` for every block: the library spectra whose precursor lies within half of `window` of the block's, as
/// a non-negative mix of its peaks. Empty for a block without precursor or peaks.
fn deconvolutions(job: &Job, library: Named, text: &str, window: f64) -> Result<Vec<String>> {
    let lib = library_spectra(library)?;
    let tol = param(job, "tol_da").unwrap_or(0.02);
    let min_share = param(job, "min_share").unwrap_or(0.05);
    let (_, blocks) = parse(text);
    let mut out = Vec::with_capacity(blocks.len());
    for b in &blocks {
        let (mz, it) = peaks(&b.lines);
        let cands: Vec<&Spectrum> = lib
            .iter()
            .filter(|s| s.pol == b.polarity && (s.prec - b.prec).abs() <= window / 2.0)
            .collect();
        let view: Vec<Candidate> = cands
            .iter()
            .map(|s| Candidate {
                mz: &s.mz,
                it: &s.it,
            })
            .collect();
        let Some(d) = b
            .prec
            .is_finite()
            .then(|| deconvolve(&mz, &it, &view, tol))
            .flatten()
        else {
            out.push(String::new());
            continue;
        };
        let list: Vec<String> = cands
            .iter()
            .zip(&d.shares)
            .filter(|(_, &x)| x >= min_share)
            .map(|(s, x)| format!("{}:{x:.3}", s.meta.name.replace([';', ':'], " ")))
            .collect();
        out.push(format!(
            "DECONV={}\nDECONV_RESIDUAL={:.3}\n",
            list.join(";"),
            d.residual
        ));
    }
    Ok(out)
}

/// The `mz intensity` lines of a block.
fn peaks(lines: &[&str]) -> (Vec<f64>, Vec<f64>) {
    let pair = |l: &&str| {
        let mut f = l.split_whitespace();
        Some((
            f.next()?.parse::<f64>().ok()?,
            f.next()?.parse::<f64>().ok()?,
        ))
    };
    lines.iter().filter_map(pair).unzip()
}

fn chimeric_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let spectra = read_one(r, job, "spectra")?;
    if read_all(r, job, "index")?.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "index"));
    }
    let text = String::from_utf8_lossy(&spectra.1);
    let (_, blocks) = parse(&text);
    let ms1_mode = job.instance.algorithm.as_deref() == Some("ms1_window");
    let chim_rel = ms1_mode.then(|| param(job, "chim_rel").unwrap_or(0.10));
    let (pure, ms1) = purities(r, job, &blocks, chim_rel)?;
    let has_table = job.inputs.iter().any(|(p, _)| p == "features");
    let all = if has_table {
        Some(points(&read_one(r, job, "features")?.1)?)
    } else {
        None
    };
    let tol = (
        param(job, "window").unwrap_or_else(|| default_window("")),
        param(job, "rt_tol").unwrap_or(0.1),
    );
    let extra = if job.instance.algorithm.as_deref() == Some("library_deconvolution") {
        if !job.inputs.iter().any(|(p, _)| p == "library") {
            return Err(Error::new("err.flow.port.missing").with("port", "library"));
        }
        let library = read_one(r, job, "library")?;
        deconvolutions(job, (&library.0, &library.1), &text, tol.0)?
    } else {
        vec![]
    };
    let text = chimeric_text(
        tol,
        &spectra.1,
        (&pure, &ms1),
        param(job, "chim_cap").unwrap_or(0.50),
        &extra,
        all.as_deref(),
    );
    let out = r.work.join(format!("{}.mgf", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Spectra)
}
