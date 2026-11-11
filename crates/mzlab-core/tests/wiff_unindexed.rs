//! A `.wiff.scan` newer than its `.wiff` (a copy made while Analyst was acquiring): the sections the index does not list are samples after the indexed ones,
//! with an inferred name and method, and the file says so once (`Reader::unindexed`). Synthetic pairs of `tools/wiff_riferimento.py`; skipped without python3
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member, Reader};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_nonind_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_unindexed: python3 not available");
        return None;
    }
    let want =
        serde_json::from_slice(&std::fs::read(out.join("nonindicizzati.json")).unwrap()).unwrap();
    Some((out, want))
}

fn open(dir: &Path, name: &str) -> Box<dyn Reader> {
    let member = |file: String| {
        let b = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    };
    readers::apri(set, &Limits::sito()).unwrap()
}

fn trace(l: &mut Box<dyn Reader>, c: usize, tail: &str) -> (Vec<f64>, Vec<f64>) {
    let id = l
        .chromatograms(c)
        .unwrap()
        .into_iter()
        .find(|h| h.id.contains(tail))
        .unwrap()
        .id;
    let ch = l.chromatogram(c, &id).unwrap();
    (ch.time, ch.intensity)
}

#[test]
fn unlisted_sections_are_samples_with_inferred_name_and_method() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let mut l = open(&dir, "nonindicizzato");
    assert_eq!(l.unindexed(), (2, 0));
    let names: Vec<String> = l.samples().iter().map(|s| s.nome.clone()).collect();
    let ids: Vec<u32> = l.samples().iter().map(|s| s.id).collect();
    assert_eq!(ids, [1, 2, 3, 4]);
    assert_eq!(&names[2..], ["nonindicizzato_s3", "nonindicizzato_s4"]);
    for (c, sec) in [(2, "3"), (3, "4")] {
        let w = &want["sections"][sec];
        let t: Vec<f64> = w["time"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        for (tail, key) in [("transition=0 ", "q1"), ("transition=1 ", "q2")] {
            let (time, inten) = trace(&mut l, c, tail);
            let v: Vec<f64> = w[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            assert_eq!(inten, v, "section {sec} {key}");
            assert!(
                time.len() == t.len() && time.iter().zip(&t).all(|(a, b)| (a - b).abs() < 1e-12),
                "section {sec} times"
            );
        }
        // the method (two transitions) is the one of the indexed sample, and the header says that name and method were inferred
        let h = l.mzml_header(c).unwrap().unwrap();
        assert!(
            h.run_params
                .iter()
                .any(|(k, v)| k == "mzlab:wiff-unindexed" && v.contains("sample 2")),
            "inferred marker"
        );
    }
    // an indexed sample says nothing of the kind
    let h = l.mzml_header(1).unwrap().unwrap();
    assert!(!h
        .run_params
        .iter()
        .any(|(k, _)| k == "mzlab:wiff-unindexed"));
}

#[test]
fn a_section_that_cannot_be_checked_is_left_out_and_counted() {
    let Some((dir, _)) = reference() else { return };
    // the donor holds scans: its blocks have no fixed size, the section's geometry is unknown
    let l = open(&dir, "nonindicizzato_scansione");
    assert_eq!((l.unindexed(), l.samples().len()), ((0, 1), 1));
    // a block with a negative count is not an MRM block of that layout
    let l = open(&dir, "nonindicizzato_guasto");
    assert_eq!((l.unindexed(), l.samples().len()), ((0, 1), 1));
}

#[test]
fn a_complete_pair_has_nothing_unlisted() {
    let Some((dir, _)) = reference() else { return };
    for name in ["one", "two", "transizioni_lotto", "buchi"] {
        assert_eq!(open(&dir, name).unindexed(), (0, 0), "{name}");
    }
}
