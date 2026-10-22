//! The native executor of `spectra.chimeric@1`: for every MS2 spectrum of an MGF, the purity of its precursor in the MS1 scans around it (`fill_purity`,
//! the one of `tp.link`) and, with a feature table, the features co-isolated in its window (`ms2_engine::chimeric::intruders`). The spectrum is written
//! as it came, with the header lines `PURITY`, `CHIMERIC` and `INTRUDERS` added; nothing is removed from it.

use std::fmt::Write as _;

use crate::flow::{all_inputs, io, read_all, read_one, Exec, FileRunner};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::attributio::ms2_engine::chimeric::{default_window, intruders, FeaturePoint};
use mzlab_feat::interp::ms2link::{fill_purity, Ms2Scan};
use mzlab_feat::legacy::ms2::Ms2Index;
use mzlab_feat::store::{read_table_from, Slice};

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

fn param(job: &Job, name: &str) -> Option<f64> {
    let found = job.resolved.params.iter().find(|(k, _)| k == name);
    found.and_then(|(_, v)| v.f64())
}

/// The purity of the precursor of every block, `None` where no index file has the scan or no MS1 lies around it.
fn purities(r: &FileRunner, job: &Job, blocks: &[Block]) -> Result<Vec<Option<(f64, bool)>>> {
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
    for k in 0..files.len() {
        for polarity in [1i8, -1] {
            let mut mine: Vec<usize> = (0..blocks.len())
                .filter(|&i| found[i].is_some_and(|f| f.0 == k) && blocks[i].polarity == polarity)
                .collect();
            mine.sort_unstable();
            let mut scans: Vec<Ms2Scan> = mine
                .iter()
                .map(|&i| {
                    let g = found[i].map_or(0, |f| f.1);
                    let s = &files[k].scans[g];
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
            fill_purity(&mut files[k], polarity, &mut scans)?;
            for (&i, s) in mine.iter().zip(&scans) {
                out[i] = Some((s.purity, s.chimeric));
            }
        }
    }
    Ok(out)
}

/// The feature rows of the `.mzfeat` as points: id, m/z and RT (min).
fn points(table: &[u8]) -> Result<Vec<FeaturePoint>> {
    if table.first() == Some(&b'{') {
        return Err(Error::new("err.flow.export.view"));
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
    pure: &[Option<(f64, bool)>],
    table: Option<&[FeaturePoint]>,
) -> String {
    let text = String::from_utf8_lossy(spectra);
    let (loose, blocks) = parse(&text);
    let mut out = String::new();
    for l in loose.iter().filter(|l| l.starts_with('#')) {
        let _ = writeln!(out, "{l}");
    }
    for (b, p) in blocks.iter().zip(pure) {
        out.push_str("BEGIN IONS\n");
        if let Some((purity, chimeric)) = p {
            if purity.is_finite() {
                let _ = writeln!(out, "PURITY={purity:.4}");
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

fn chimeric_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let spectra = read_one(r, job, "spectra")?;
    if read_all(r, job, "index")?.is_empty() {
        return Err(Error::new("err.flow.port.missing").with("port", "index"));
    }
    let text = String::from_utf8_lossy(&spectra.1);
    let (_, blocks) = parse(&text);
    let pure = purities(r, job, &blocks)?;
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
    let text = chimeric_text(tol, &spectra.1, &pure, all.as_deref());
    let out = r.work.join(format!("{}.mgf", job.instance.id));
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Spectra)
}
