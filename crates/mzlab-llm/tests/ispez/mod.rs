//! Helpers shared by the `ispezione` tests: build the sources from the synthetic files with the engine of `mzlab-core`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::campioni;
use mzlab_core::idxread::Index;
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::lr::{open_items, Sample};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::upload::prepare;
use mzlab_llm::inspect::scan_stats;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The synthetic files, written once per process; `None` without python3 and numpy.
pub fn synthetic() -> Option<&'static Path> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let out = std::env::temp_dir().join(format!("mzlab-ispezione-{}", std::process::id()));
        let run = Command::new("python3")
            .current_dir(root())
            .arg("tools/dati_sintetici.py")
            .arg(&out)
            .output();
        if matches!(&run, Ok(o) if o.status.success()) {
            Some(out)
        } else {
            assert!(
                std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
                "the generator did not run: {run:?}"
            );
            eprintln!("python3/numpy not available: test skipped");
            None
        }
    })
    .as_deref()
}

pub fn j(text: &str) -> Value {
    parse(text.as_bytes()).unwrap_or_else(|e| panic!("{e:?}: {text}"))
}

pub fn quoted(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}

pub fn series_of(rt: &[f64], y: &[f64]) -> String {
    let mut o = Out::default();
    o.raw("{\"rt\":")
        .floats(rt)
        .raw(",\"i\":")
        .floats(y)
        .raw("}");
    o.0
}

/// A trace the way the detection of features would report a strong one: the maximum of every 1 Da bin of the survey scans, kept when it stands far above
/// the others and cut at a tenth of its height.
pub fn signals(idx: &mut Index<Vec<u8>>) -> Vec<[f64; 3]> {
    let Some(si) = idx.section_of(1, None) else {
        return Vec::new();
    };
    let rt = idx.arrays(si).unwrap().rt.clone();
    let mut out = Vec::new();
    for lo in 120..480 {
        let (a, b) = (
            idx.search(si, lo as f64, false).unwrap(),
            idx.search(si, lo as f64 + 1.0, false).unwrap(),
        );
        let y = idx.max_window(si, a, b).unwrap();
        let top = y.iter().copied().fold(0.0, f64::max);
        if top < 5.0e4 {
            continue;
        }
        let apex = y.iter().position(|v| *v == top).unwrap();
        let mut mz = 0.0;
        idx.each_peak(si, a, b, |p, m, it| {
            if p == apex && it == top {
                mz = m;
            }
        })
        .unwrap();
        let area: f64 = y
            .windows(2)
            .zip(rt.windows(2))
            .filter(|(w, _)| w[0] >= top * 0.1)
            .map(|(w, t)| w[0] * (t[1] - t[0]))
            .sum();
        // RT: the centre of gravity of the top half of the peak, finer than the scan step
        let (w, wt) = y
            .iter()
            .zip(&rt)
            .filter(|(v, _)| **v >= top * 0.5)
            .fold((0.0, 0.0), |(a, b), (v, t)| (a + v * t, b + v));
        let _ = apex;
        out.push([mz, w / wt, area]);
    }
    out
}

pub struct Opened {
    pub items: String,
    pub stats: Value,
    pub mrm: String,
    pub tic: String,
    pub bpc: String,
    pub signals: Vec<[f64; 3]>,
}

pub fn open(dir: &Path, name: &str) -> Opened {
    let bytes = std::fs::read(dir.join(name)).unwrap();
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &bytes, 256 << 20).unwrap();
    let len = idx.len() as u64;
    let mut idx = Index::open(idx, len).unwrap();
    let mut m = Mzml::new(bytes.clone(), bytes.len() as u64);
    let items: Vec<String> = open_items(&mut idx, &mut m, name, &Sample::default())
        .unwrap()
        .into_iter()
        .map(|i| i.info)
        .collect();
    let end = idx.scans.last().map_or(0, |s| s.end);
    let mrm: Vec<String> = m
        .chromatograms(end)
        .unwrap()
        .iter()
        .filter(|c| c.kind == "srm")
        .map(|c| {
            let mut o = Out::default();
            o.raw("{\"q1\":")
                .opt(c.q1)
                .raw(",\"q3\":")
                .opt(c.q3)
                .raw("}");
            o.0
        })
        .collect();
    let (mut tic, mut bpc) = ("null".to_string(), "null".to_string());
    if let Some(si) = idx.section_of(1, None) {
        let a = idx.arrays(si).unwrap();
        tic = series_of(&a.rt, &a.sumint);
        bpc = series_of(&a.rt, &a.bp_inten);
    }
    Opened {
        items: format!("[{}]", items.join(",")),
        stats: scan_stats(&idx.scans),
        mrm: format!("[{}]", mrm.join(",")),
        tic,
        bpc,
        signals: signals(&mut idx),
    }
}

pub fn stats_text(v: &Value) -> String {
    let mut o = Out::default();
    mzlab_llm::tools::value(&mut o, v);
    o.0
}

/// The sample table the names propose, one entry per file in the order given.
pub fn proposals(names: &[&str]) -> Vec<Value> {
    let files: Vec<String> = names
        .iter()
        .map(|n| format!("{{\"name\":{}}}", quoted(n)))
        .collect();
    let out = campioni::route("proponi", &format!("{{\"files\":[{}]}}", files.join(","))).unwrap();
    j(&out).get("campioni").unwrap().arr().to_vec()
}

pub fn sample_text(c: &Value, order: usize) -> String {
    format!(
        "{{\"tipo\":{},\"ordine_iniezione\":{order}}}",
        quoted(c.get("tipo").unwrap().str().unwrap())
    )
}

pub fn sources(
    dir: &Path,
    names: &[&str],
    extra: &dyn Fn(&str) -> String,
    with_series: bool,
) -> Value {
    let samples = proposals(names);
    let files: Vec<String> = names
        .iter()
        .zip(&samples)
        .enumerate()
        .map(|(k, (name, c))| {
            let o = open(dir, name);
            let sig: Vec<String> = o.signals.iter().map(|s| format!("{{\"mz\":{},\"rt\":{},\"area\":{}}}", s[0], s[1], s[2])).collect();
            let series = if with_series {
                format!(",\"tic\":{},\"bpc\":{}", o.tic, o.bpc)
            } else {
                String::new()
            };
            format!(
                "{{\"name\":{},\"items\":{},\"scan_stats\":{},\"mrm\":{},\"sample\":{},\"signals\":[{}]{series}{}}}",
                quoted(name),
                o.items,
                stats_text(&o.stats),
                o.mrm,
                sample_text(c, k + 1),
                sig.join(","),
                extra(name)
            )
        })
        .collect();
    j(&format!(
        "{{\"profile\":\"ricerca\",\"hide_names\":false,\"files\":[{}]}}",
        files.join(",")
    ))
}

pub fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k)
        .and_then(Value::str)
        .unwrap_or_else(|| panic!("no {k} in a file"))
}

pub fn list(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .unwrap_or(&Value::Null)
        .arr()
        .iter()
        .filter_map(|x| x.str().map(String::from))
        .collect()
}

pub fn files_of(out: &str) -> Vec<Value> {
    j(out).get("files").unwrap().arr().to_vec()
}

pub const KINDS: [&str; 11] = [
    "B_FullMass-t0.mzML",
    "B_FullMass-neg-t0.mzML",
    "C_POLALT-t0.mzML",
    "C_IDA-t0.mzML",
    "C_MRM-EPI-t0.mzML",
    "B_MRM-STD_12ppm.mzML",
    "B_MS2-t15.mzML",
    "HR_DDA-Exploris-t30.mzML",
    "HR_DDA-Fusion-t30.mzML",
    "HR_rotto-t30.mzML",
    "D_DIA-t0.mzML",
];
