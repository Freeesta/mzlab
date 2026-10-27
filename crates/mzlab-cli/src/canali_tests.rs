//! `mzlab-cli canali`: the channels of synthetic files. The files of `tools/dati_sintetici.py` (skipped without python3 and numpy, `MZLAB_GOLDEN_STRICT=1`
//! fails instead) give the lists written below by hand; an mzML made here gives the same JSON as the engine of the page (`Routes::canali`).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};

use crate::execute;

fn canali_di(file: &Path) -> Value {
    let o = execute(&["canali".to_string(), file.to_string_lossy().into_owned()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    parse(&o.stdout).unwrap()
}

fn ids(v: &Value) -> Vec<String> {
    v.get("canali")
        .unwrap()
        .arr()
        .iter()
        .map(|c| c.get("id").unwrap().str().unwrap().to_string())
        .collect()
}

fn campo(v: &Value, i: usize, k: &str) -> f64 {
    v.get("canali").unwrap().arr()[i]
        .get(k)
        .unwrap()
        .f64()
        .unwrap()
}

fn proposto(v: &Value) -> &str {
    v.get("tipo")
        .unwrap()
        .get("proposto")
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
    let o = execute(&["canali".into(), f.to_string_lossy().into_owned()]);
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
    assert_eq!(ids(&canali_di(&f)), ["tic", "bpc", "ms:+ p ESI Q1MS"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn usage_errors_exit_with_2() {
    for a in [
        vec!["canali"],
        vec!["canali", "a", "b"],
        vec!["canali", "--nope"],
        vec!["canali", "/nonexistent/x.mzML"],
    ] {
        assert_eq!(
            execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()).code,
            2,
            "{a:?}"
        );
    }
    assert_eq!(execute(&["canali".into(), "--help".into()]).code, 0);
}

/// The files of `tools/dati_sintetici.py` the tests need, in a new folder; `None` without python3 and numpy.
fn sintetici() -> Option<PathBuf> {
    // one directory per call: the tests run in parallel
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("mzlab_cli_canali_dati_{}_{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys; from pathlib import Path; import numpy as np; sys.path.insert(0, sys.argv[1]); import dati_sintetici as d, dati_sintetici_canali as k; o = Path(sys.argv[2]); \
                  k.canali_casi(o / 'canali'); r = np.random.default_rng(3); d.ms2(o / 'B_MS2.mzML', r, 15); d.mrm(o / 'B_MRM.mzML', 1e5, r, 1); \
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
    let Some(d) = sintetici() else { return };
    // MRM: the TIC of the maker, one channel per transition
    let v = canali_di(&d.join("B_MRM.mzML"));
    assert_eq!(
        ids(&v),
        ["tic", "srm:364.1>194.1@20", "srm:364.1>152.1@20", "uv:twc"]
    );
    assert_eq!(proposto(&v), "lc-mrm");
    let uv = ids(&v).iter().position(|i| i == "uv:twc").unwrap();
    let c = &v.get("canali").unwrap().arr()[uv];
    assert!(matches!(c.get("gen"), Some(g) if g.f64().is_none()));
    assert_eq!(c.get("fam").unwrap().str(), Some("uv"));
}

#[test]
fn channels_of_the_synthetic_files() {
    let Some(d) = sintetici() else { return };
    // targeted MS2 in LC: the survey scan and one channel per fixed precursor, each hanging from the survey
    let v = canali_di(&d.join("canali/K_MIRATO-t0.mzML"));
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
    assert_eq!(
        (campo(&v, 2, "n"), campo(&v, 3, "n"), campo(&v, 4, "n")),
        (100.0, 100.0, 100.0)
    );
    assert_eq!((campo(&v, 3, "gen"), campo(&v, 4, "gen")), (2.0, 2.0));
    assert_eq!(proposto(&v), "lc-ms2-mirato");
    // MSn in blocks: MS3 hangs from the MS2 of its first precursor
    let v = canali_di(&d.join("canali/K_BLOCCHI-t0.mzML"));
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
    assert_eq!((campo(&v, 4, "gen"), campo(&v, 5, "gen")), (3.0, 3.0));
    assert_eq!(proposto(&v), "infusione-msn");
    // 70 distinct precursors: one channel for all of them
    let v = canali_di(&d.join("canali/K_DDA-t0.mzML"));
    assert_eq!(ids(&v), ["tic", "bpc", "ms:FTMS + p ESI Full ms", "dd:2"]);
    assert_eq!((campo(&v, 3, "n"), campo(&v, 3, "liv")), (70.0, 2.0));
    assert_eq!(proposto(&v), "lc-dda");
    // the lab-like MS2 file: two precursors, no survey scan
    let v = canali_di(&d.join("B_MS2.mzML"));
    assert_eq!(
        ids(&v),
        ["tic", "bpc", "ms:ms2 229", "ms:ms2 305", "uv:twc"]
    );
    assert_eq!((campo(&v, 2, "n"), campo(&v, 3, "n")), (503.0, 502.0));
    // MRM + EPI: the triggered scans next to the transitions
    let v = canali_di(&d.join("C_MRM-EPI.mzML"));
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
    assert_eq!(proposto(&v), "misto");
    // IDA with a few precursors: the survey scan and the MS2 channels that add up to the product-ion scans
    let v = canali_di(&d.join("C_IDA.mzML"));
    let c = v.get("canali").unwrap().arr();
    assert_eq!(c[2].get("id").unwrap().str(), Some("ms:ms1"));
    assert_eq!(c[2].get("n").unwrap().f64(), Some(160.0));
    let ms2: f64 = c
        .iter()
        .filter(|x| x.get("liv").unwrap().f64() == Some(2.0))
        .map(|x| x.get("n").unwrap().f64().unwrap())
        .sum();
    assert_eq!(ms2, 320.0);
    let _ = std::fs::remove_dir_all(&d);
}

/// An mzML with a TWC (a 2 × 2 `dad_matrix` is not needed for the trace) and a pump pressure trace: `--canale` reads them as the old routes do.
#[test]
fn canale_reads_a_uv_and_an_analog_trace() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_canale_uv_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let arr = |acc: &str, unit: &str, v: &str| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"{unit}/><binary>{v}</binary></binaryDataArray>")
    };
    let b64 = |v: &[f64]| {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        let mut o = String::new();
        for c in raw.chunks(3) {
            let n = (c[0] as u32) << 16
                | (*c.get(1).unwrap_or(&0) as u32) << 8
                | *c.get(2).unwrap_or(&0) as u32;
            for k in 0..4 {
                o.push(if k <= c.len() {
                    A[(n >> (18 - 6 * k) & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        o
    };
    let chrom = |i: usize, id: &str, acc: &str, unit: &str| {
        format!("<chromatogram index=\"{i}\" id=\"{id}\" defaultArrayLength=\"2\"><cvParam accession=\"{acc}\" name=\"x\"/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></chromatogram>", arr("MS:1000595", "", &b64(&[0.0, 0.03])), arr("MS:1000515", unit, &b64(&[1.0, 2.0])))
    };
    let xml = format!(
        "<mzML><run><chromatogramList count=\"2\">{}{}</chromatogramList></run></mzML>",
        chrom(0, "TWC", "MS:1000813", ""),
        chrom(1, "Pump Pressure", "MS:1003019", " unitName=\"bar\"")
    );
    let f = dir.join("u.mzML");
    std::fs::write(&f, xml).unwrap();
    let p = f.to_string_lossy().into_owned();
    let run = |extra: &[&str]| {
        let mut a = vec!["canali".to_string(), p.clone()];
        a.extend(extra.iter().map(|s| s.to_string()));
        execute(&a)
    };
    let ids = ids(&canali_di(&f));
    assert!(
        ids.contains(&"uv:twc".to_string()) && ids.contains(&"an:1".to_string()),
        "{ids:?}"
    );
    assert_eq!(proposto(&canali_di(&f)), "solo-uv");
    let twc = run(&["--canale", "uv:twc"]);
    assert_eq!(twc.code, 0, "{}", twc.stderr);
    assert_eq!(
        String::from_utf8(twc.stdout).unwrap(),
        "{\"rt\":[0,0.03],\"y\":[1,2]}\n"
    );
    let an = run(&["--canale", "an:1", "--n", "50"]);
    assert_eq!(an.code, 0, "{}", an.stderr);
    assert!(String::from_utf8(an.stdout)
        .unwrap()
        .contains("\"unit\":\"bar\""));
    assert_eq!(run(&["--canale", "tic"]).code, 2);
    assert_eq!(run(&["--rt", "1"]).code, 2);
    let _ = std::fs::remove_dir_all(&dir);
}
