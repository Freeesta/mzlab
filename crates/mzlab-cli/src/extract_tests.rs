//! Fixtures of the `trace`, `spectrum` and `file info` tests: an mzML written here, the routes of the page on its bytes, and the synthetic files of
//! `tools/dati_sintetici.py` (skipped without python3 and numpy; `MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::routes::Routes;

use super::{parse, tokens};

/// `mzlab-cli ARGS...`.
pub(crate) fn run(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// A folder of its own for one test.
pub(crate) fn dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("mzlab_cli_{tag}_{}_{k}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The Q1 mzML of `crate::tests::mzml` in a new folder: (folder, path).
pub(crate) fn mzml_file(tag: &str) -> (PathBuf, String) {
    let d = dir(tag);
    let f = d.join("a.mzML");
    std::fs::write(&f, crate::tests::mzml(10.0)).unwrap();
    (d, f.to_string_lossy().into_owned())
}

/// The routes of the page on the index of `path` (what the site holds for the same bytes), with no chromatogram headers.
pub(crate) fn page_routes(path: &str) -> Routes<Vec<u8>> {
    let b = std::fs::read(path).unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "a.mzML").unwrap();
    let idx = mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20)
        .unwrap()
        .0;
    let n = idx.len() as u64;
    let mut r = Routes::new(mzlab_core::idxread::Index::open(idx, n).unwrap());
    r.pon_cromatogrammi("a.mzML", &[], &Default::default());
    r
}

/// The `x`/`y` tokens of a CLI `trace/1` or `spectrum/1` document (`points`), as the text of each point.
pub(crate) fn doc_points(doc: &str) -> Vec<(String, String)> {
    let at = doc.find("\"points\":[").unwrap() + 10;
    let end = at + doc[at..].find("]]").map_or(0, |e| e + 1);
    let body = &doc[at..end];
    if body.is_empty() {
        return Vec::new();
    }
    body.trim_start_matches('[')
        .trim_end_matches(']')
        .split("],[")
        .map(|p| {
            let (a, b) = p.split_once(',').unwrap();
            (a.to_string(), b.to_string())
        })
        .collect()
}

/// The points of a route answer `{"rt":[..],"y":[..]}` (or `mz`) as the same text pairs.
pub(crate) fn route_points(text: &str, x: &str) -> Vec<(String, String)> {
    let (a, b) = (tokens(text, x).unwrap(), tokens(text, "y").unwrap());
    a.iter()
        .zip(b)
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// The synthetic files (the MRM and MS2 files, the channel cases and the mixed ones) in a new folder; `None` without python3 and numpy.
pub(crate) fn synthetic() -> Option<PathBuf> {
    let d = dir("dati");
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
fn the_argument_reader_splits_values_switches_and_words() {
    let a = ["f", "--id", "tic", "--json", "--format", "csv"]
        .map(String::from)
        .to_vec();
    let p = parse(a, &["--id", "--format"], &[]).unwrap().unwrap();
    assert_eq!(p.pos, ["f"]);
    assert_eq!((p.get("--id"), p.csv()), (Some("tic"), Ok(true)));
    assert!(parse(vec!["--nope".into()], &[], &[]).is_err());
    assert!(parse(vec!["--help".into()], &[], &[]).unwrap().is_none());
}

/// The members of a document are exactly those its schema in `docs/schemi/` lists.
#[test]
fn the_documents_have_the_keys_of_their_schemas() {
    use mzlab_core::json::parse;
    let keys =
        |v: &mzlab_core::json::Value| v.entries().iter().map(|e| e.0.clone()).collect::<Vec<_>>();
    let props = |b: &[u8]| keys(parse(b).unwrap().get("properties").unwrap());
    let (d, f) = mzml_file("schema_keys");
    let doc =
        |a: &[&str]| parse(String::from_utf8(run(a).stdout).unwrap().trim().as_bytes()).unwrap();
    assert_eq!(
        keys(&doc(&["trace", "chrom", &f, "--id", "tic"])),
        props(include_bytes!("../../../docs/schemi/trace-1.json"))
    );
    assert_eq!(
        keys(&doc(&["trace", "xic", &f, "--mz", "241.3", "--ppm", "10"])),
        props(include_bytes!("../../../docs/schemi/trace-1.json"))
    );
    assert_eq!(
        keys(&doc(&["spectrum", "get", &f, "--rt", "3"])),
        props(include_bytes!("../../../docs/schemi/spectrum-1.json"))
    );
    let list = doc(&["spectrum", "list", &f]);
    assert_eq!(
        keys(&list),
        props(include_bytes!("../../../docs/schemi/scanlist-1.json"))
    );
    let schema = parse(include_bytes!("../../../docs/schemi/scanlist-1.json")).unwrap();
    let item = schema
        .get("properties")
        .unwrap()
        .get("scans")
        .unwrap()
        .get("items")
        .unwrap();
    assert_eq!(
        keys(&list.get("scans").unwrap().arr()[0]),
        keys(item.get("properties").unwrap())
    );
    assert_eq!(
        keys(&doc(&["file", "info", &f])),
        props(include_bytes!("../../../docs/schemi/file_info-1.json"))
    );
    let _ = std::fs::remove_dir_all(d);
}
