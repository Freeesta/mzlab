//! `mzlab-cli file channels`: the channels of synthetic files. The files of `tools/dati_sintetici.py` (skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1`
//! fails instead) give the lists written below by hand; an mzML made here gives the same JSON as the engine of the page (`Routes::canali`).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::execute;

/// `mzlab-cli file VERB ARGS...`.
fn fch(verb: &str, rest: &[&str]) -> crate::Outcome {
    let mut a = vec!["file".to_string(), verb.to_string()];
    a.extend(rest.iter().map(|x| x.to_string()));
    execute(&a)
}

fn channels_of(file: &Path) -> Value {
    let o = fch("channels", &[&file.to_string_lossy()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

fn ids(v: &Value) -> Vec<String> {
    v.get("channels")
        .unwrap()
        .arr()
        .iter()
        .map(|c| c.get("id").unwrap().str().unwrap().to_string())
        .collect()
}

/// `mzlab-cli trace chrom FILE --id ID`.
fn fch_trace(file: &str, id: &str) -> crate::Outcome {
    execute(&["trace", "chrom", file, "--id", id].map(String::from))
}

/// `mzlab-cli file channels FILE extra...`.
fn run_new(p: &str, extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["file".to_string(), "channels".to_string(), p.to_string()];
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn field(v: &Value, i: usize, k: &str) -> f64 {
    v.get("channels").unwrap().arr()[i]
        .get(k)
        .unwrap()
        .f64()
        .unwrap()
}

fn pts(v: &Value, i: usize) -> f64 {
    field(v, i, "n_points")
}

fn gen_of(v: &Value, i: usize) -> f64 {
    field(v, i, "generator")
}

fn proposed(v: &Value) -> &str {
    v.get("kind")
        .unwrap()
        .get("proposed")
        .unwrap()
        .str()
        .unwrap()
}

#[test]
fn cli_gives_the_json_of_the_engine() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_canali_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("a.mzML");
    std::fs::write(&f, crate::tests::mzml(10.0)).unwrap();
    let o = fch("channels", &[&f.to_string_lossy()]);
    let b = std::fs::read(&f).unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "a.mzML").unwrap();
    let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    let mut r = mzlab_core::routes::Routes::new(mzlab_core::idxread::Index::open(idx, n).unwrap());
    r.pon_cromatogrammi("a.mzML", &[], &Default::default());
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        r.canali().json() + "\n"
    );
    assert_eq!(ids(&channels_of(&f)), ["tic", "bpc", "ms:+ p ESI Q1MS"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_errors_exit_with_2() {
    for a in [
        vec!["file", "channels"],
        vec!["file", "channels", "a", "b"],
        vec!["file", "channels", "--nope"],
        vec!["file", "channels", "a.mzML", "--sample", "1"],
        vec!["file", "channels", "a.wiff", "--sample", "x"],
    ] {
        assert_eq!(
            execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()).code,
            2,
            "{a:?}"
        );
    }
    assert_eq!(fch("channels", &["--help"]).code, 0);
    assert_eq!(run_new("/nonexistent/x.mzML", &[]).code, 4);
}

/// The files of `tools/dati_sintetici.py` the tests need, in a new folder; `None` without python3 and numpy.
fn synthetic() -> Option<PathBuf> {
    // one directory per call: the tests run in parallel
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("mzlab_cli_canali_dati_{}_{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys; from pathlib import Path; import numpy as np; sys.path.insert(0, sys.argv[1]); import dati_sintetici as d, synthetic_data_channels as k; o = Path(sys.argv[2]); \
                  k.channel_cases(o / 'canali'); r = np.random.default_rng(3); d.ms2(o / 'B_MS2.mzML', r, 15); d.mrm(o / 'B_MRM.mzML', 1e5, r, 1); \
                  d.mixed_ida(o / 'C_IDA.mzML', r); d.mixed_mrm_epi(o / 'C_MRM-EPI.mzML', r)";
    let ok = Command::new("python3")
        .args(["-c", script])
        .arg(&tools)
        .arg(&d)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return None;
    }
    Some(d)
}

#[test]
fn mrm_file_lists_its_uv_trace_as_a_root() {
    let Some(d) = synthetic() else { return };
    // MRM: the TIC of the maker, one channel per transition
    let v = channels_of(&d.join("B_MRM.mzML"));
    assert_eq!(
        ids(&v),
        ["tic", "srm:364.1>194.1@20", "srm:364.1>152.1@20", "uv:twc"]
    );
    assert_eq!(proposed(&v), "lc-mrm");
    let uv = ids(&v).iter().position(|i| i == "uv:twc").unwrap();
    let c = &v.get("channels").unwrap().arr()[uv];
    assert!(matches!(c.get("generator"), Some(g) if g.f64().is_none()));
    assert_eq!(c.get("family").unwrap().str(), Some("uv"));
}

#[test]
fn channels_of_the_synthetic_files() {
    let Some(d) = synthetic() else { return };
    // targeted MS2 in LC: the survey scan and one channel per fixed precursor, each hanging from the survey
    let v = channels_of(&d.join("canali/K_MIRATO-t0.mzML"));
    assert_eq!(
        ids(&v),
        [
            "tic",
            "bpc",
            "ms:ITMS + c ESI Full ms",
            "ms:ITMS + ms2 222 @cid",
            "ms:ITMS + ms2 305 @cid"
        ]
    );
    assert_eq!((pts(&v, 2), pts(&v, 3), pts(&v, 4)), (100.0, 100.0, 100.0));
    assert_eq!((gen_of(&v, 3), gen_of(&v, 4)), (2.0, 2.0));
    assert_eq!(proposed(&v), "lc-ms2-targeted");
    // MSn in blocks: MS3 hangs from the MS2 of its first precursor
    let v = channels_of(&d.join("canali/K_BLOCCHI-t0.mzML"));
    assert_eq!(
        ids(&v),
        [
            "tic",
            "bpc",
            "ms:ITMS + c ESI Full ms",
            "ms:ITMS + ms2 317 @cid",
            "ms:ITMS + ms3 317 > 261 @cid30",
            "ms:ITMS + ms3 317 > 299 @cid30"
        ]
    );
    assert_eq!((gen_of(&v, 4), gen_of(&v, 5)), (3.0, 3.0));
    assert_eq!(proposed(&v), "msn-infusion");
    // 70 distinct precursors: one channel for all of them
    let v = channels_of(&d.join("canali/K_DDA-t0.mzML"));
    assert_eq!(ids(&v), ["tic", "bpc", "ms:FTMS + p ESI Full ms", "dd:2"]);
    assert_eq!((pts(&v, 3), field(&v, 3, "ms_level")), (70.0, 2.0));
    assert_eq!(proposed(&v), "lc-dda");
    // the lab-like MS2 file: two precursors, no survey scan
    let v = channels_of(&d.join("B_MS2.mzML"));
    assert_eq!(
        ids(&v),
        ["tic", "bpc", "ms:ms2 229", "ms:ms2 305", "uv:twc"]
    );
    assert_eq!((pts(&v, 2), pts(&v, 3)), (503.0, 502.0));
    // MRM + EPI: the triggered scans next to the transitions
    let v = channels_of(&d.join("C_MRM-EPI.mzML"));
    assert_eq!(
        ids(&v),
        [
            "tic",
            "bpc",
            "ms:ms2 364",
            "ms:ms2 365",
            "srm:364.1>194.1@20",
            "srm:364.1>152.1@20"
        ]
    );
    assert_eq!(proposed(&v), "mixed");
    // IDA with a few precursors: the survey scan and the MS2 channels that add up to the product-ion scans
    let v = channels_of(&d.join("C_IDA.mzML"));
    let c = v.get("channels").unwrap().arr();
    assert_eq!(c[2].get("id").unwrap().str(), Some("ms:ms1"));
    assert_eq!(c[2].get("n_points").unwrap().f64(), Some(160.0));
    let ms2: f64 = c
        .iter()
        .filter(|x| x.get("ms_level").unwrap().f64() == Some(2.0))
        .map(|x| x.get("n_points").unwrap().f64().unwrap())
        .sum();
    assert_eq!(ms2, 320.0);
    let _ = std::fs::remove_dir_all(&d);
}

/// A SIM channel id has spaces and `name=` words: the error line names it whole, never split into parameters.
#[test]
fn an_unknown_sim_id_is_named_whole() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_canale_sim_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("e.mzML");
    std::fs::write(
        &f,
        "<mzML><run><chromatogramList count=\"0\"></chromatogramList></run></mzML>",
    )
    .unwrap();
    let id = "sim:SIM SIC Q1=237.1 sample=1 polarity=positive name=ionA";
    let o = fch_trace(&f.to_string_lossy(), id);
    assert_eq!(o.code, 5);
    let want = format!("error: Channel {id} does not exist in this file. (err.canali.unknown)\n");
    assert!(o.stderr.starts_with(&want), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Each sample of a `.wiff` (`two.wiff` of `tools/wiff_riferimento.py`: a scan sample, an MRM sample) answers with the bytes of the mzML `convert` writes
/// for it, as the site and the app list each sample on its own; without `--sample` the first one, with the others named on the standard error.
#[test]
fn each_sample_of_a_wiff_has_the_channels_of_its_mzml() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_canali_wiff_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(dir.join("in"))
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let s = |p: PathBuf| p.to_string_lossy().into_owned();
    let wiff = s(dir.join("in/two.wiff"));
    let conv = fch("convert", &[&wiff, "--out", &s(dir.join("out"))]);
    assert_eq!(conv.code, 0, "{}", conv.stderr);
    for k in ["1", "2"] {
        let o = fch("channels", &[&wiff, "--sample", k]);
        let want = fch("channels", &[&s(dir.join(format!("out/two-{k}.mzML")))]);
        assert_eq!(
            (o.code, &o.stdout, o.stderr.as_str()),
            (0, &want.stdout, ""),
            "sample {k}"
        );
    }
    let mrm = String::from_utf8(fch("channels", &[&wiff, "--sample", "2"]).stdout).unwrap();
    assert!(
        mrm.contains("\"proposed\":\"lc-mrm\"") && mrm.contains("\"id\":\"srm:"),
        "{mrm}"
    );
    let first = fch("channels", &[&wiff]);
    let one = fch("channels", &[&wiff, "--sample", "1"]);
    assert_eq!(first.stdout, one.stdout);
    assert!(
        first.stderr.contains("2 samples (ids 1, 2)"),
        "{}",
        first.stderr
    );
    let bad = fch("channels", &[&wiff, "--sample", "9"]);
    assert_eq!(bad.code, 5);
    assert!(
        bad.stderr.starts_with(
            "error: This file has no sample 9: its samples are 1, 2. (err.sample.unknown"
        ),
        "{}",
        bad.stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The device traces of a `.wiff` (`dispositivi` of `tools/wiff_riferimento.py`: two pressures, a flow, a temperature) are `an:` channels, the same the
/// converted mzML lists; `trace chrom --id an:1` of the mzML gives the column pressure (2000 psi, shown in bar).
#[test]
fn the_device_traces_of_a_wiff_are_the_analog_channels_of_its_mzml() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_canali_dev_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let made = Command::new("python3")
        .arg(tools.join("wiff_riferimento.py"))
        .arg(dir.join("in"))
        .output();
    if !made.is_ok_and(|o| o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("skipped: python3 not available");
        return;
    }
    let s = |p: PathBuf| p.to_string_lossy().into_owned();
    let wiff = s(dir.join("in/dispositivi.wiff"));
    let conv = fch("convert", &[&wiff, "--out", &s(dir.join("out"))]);
    assert_eq!(conv.code, 0, "{}", conv.stderr);
    let mzml = s(dir.join("out/dispositivi.mzML"));
    let o = fch("channels", &[&wiff]);
    assert_eq!(o.stdout, fch("channels", &[&mzml]).stdout);
    let list = String::from_utf8(o.stdout).unwrap();
    for et in [
        "Column Pressure (pascal)",
        "Pump A Flowrate (µL/min)",
        "Column Pressure #1 (pascal)",
        "Oven Temperature (°C)",
    ] {
        assert!(
            list.contains(&format!("\"label\":\"{et}\"")),
            "{et}: {list}"
        );
    }
    assert!(!list.contains("Empty"), "{list}");
    let one = fch_trace(&mzml, "an:1");
    assert_eq!(one.code, 0, "{}", one.stderr);
    let one = String::from_utf8(one.stdout).unwrap();
    assert!(
        one.contains("\"y_unit\":\"bar\"") && one.contains(",137.895]"),
        "2000 psi in bar: {one}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
