//! Dependent (IDA) product-ion scans of a `.wiff` carry the precursor of their `DDERealTimeData` row, never the mass of the method header (a saved
//! template): without the stream, or with a stream or row that does not fit, they carry none and are counted (`warn.wiff.idaPrecursor`). A targeted
//! EPI (no stream in the file) keeps its header mass. Checked on `ida`, `ida_epi` and `epi_fisso` of `tools/wiff_riferimento.py` (an independent
//! writer) through `Wiff::run`, the `Lettore` index (what the site reads), the listing and the mzML. The index rows of the IDA sample hold no point
//! count, so none of its blocks is counted as damaged. Skipped without python3.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_ida_{}", std::process::id()));
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
        eprintln!("SKIP wiff_ida: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

/// Each sample of `name` against its rows of the reference (`rows[n]`) and the count of scans with no precursor (`unread[n]`).
fn check(dir: &Path, name: &str, rows: &Value, unread: &Value) {
    let file = format!("{name}.wiff");
    let ole = container(std::fs::read(dir.join(&file)).unwrap(), &file).unwrap();
    let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    let mut l = readers::apri(set(dir, name), &Limits::sito()).unwrap();
    let listing = readers::samples_json(&mut *l);
    let samples = rows.as_object().unwrap();
    for (c, (n, rows)) in samples.iter().enumerate() {
        let rows = rows.as_array().unwrap();
        let id: u32 = n.parse().unwrap();
        let w = Wiff::open(&ole, &file, Some(scan.len() as u64), id).unwrap();
        let run = w.run(&mut scan).unwrap();
        let index = l.index(c).unwrap().to_vec();
        assert_eq!(run.spectra.len(), rows.len(), "{name} {n}");
        assert_eq!(index.len(), rows.len(), "{name} {n}: index");
        let lost = l.losses(c).unwrap();
        assert!(
            !lost.partial(),
            "{name} {n}: no block is damaged ({lost:?})"
        );
        for ((g, i), e) in run.spectra.iter().zip(&index).zip(rows) {
            let p = e["precursor"].as_f64();
            assert_eq!(g.id, e["id"].as_str().unwrap());
            assert_eq!(
                (i64::from(g.level), g.precursor),
                (e["level"].as_i64().unwrap(), p),
                "{name} {}",
                g.id
            );
            assert_eq!((i.id.as_str(), i.precursore), (g.id.as_str(), p), "index");
        }
        let want = unread[n].as_u64().unwrap_or(0) as usize;
        assert_eq!(
            l.unread_precursors(c),
            want,
            "{name} {n}: scans with no precursor"
        );
        assert_eq!(w.unread_precursors, want, "{name} {n}");
        let listed: Value = serde_json::from_str(&listing).unwrap();
        let entry = &listed["samples"][c];
        assert_eq!(entry["id"].as_u64(), Some(u64::from(id)), "{listing}");
        assert_eq!(
            entry["noPrecursor"].as_u64().unwrap_or(0) as usize,
            want,
            "{listing}"
        );
        check_mzml(&mut *l, c, name, n, rows);
    }
}

/// The mzML of sample `c`: one precursor per row that has one, with its charge; the template mass of the method never.
fn check_mzml(l: &mut dyn readers::Reader, c: usize, name: &str, n: &str, rows: &[Value]) {
    let mut mzml = Vec::new();
    let conv = mzlab_core::readers_mzml::Conversion {
        steps: Some(&[]),
        ..Default::default()
    };
    conv.write(l, c, &mut mzml, &mut |_, _| {}).unwrap();
    let text = String::from_utf8(mzml).unwrap();
    let precs: Vec<f64> = rows
        .iter()
        .filter_map(|e| e["precursor"].as_f64())
        .collect();
    assert_eq!(
        text.matches("<precursorList").count(),
        precs.len(),
        "{name} {n}: precursor lists"
    );
    for p in &precs {
        assert!(
            text.contains(&format!("name=\"selected ion m/z\" value=\"{p}\"")),
            "{name} {n}: {p}"
        );
    }
    assert!(
        !text.contains("value=\"188.4\"") || name == "epi_fisso",
        "{name} {n}: the template mass"
    );
    let charges: Vec<i64> = rows.iter().filter_map(|e| e["charge"].as_i64()).collect();
    assert_eq!(
        text.matches("MS:1000041").count(),
        charges.len(),
        "{name} {n}: charges"
    );
    for z in charges {
        assert!(
            text.contains(&format!("name=\"charge state\" value=\"{z}\"")),
            "{name} {n}: z {z}"
        );
    }
}

#[test]
fn dependent_product_scans_carry_the_precursor_of_their_row() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let ida = &want["ida"];
    check(&dir, "ida", &ida["ida"], &ida["unread"]);
    check(&dir, "ida_epi", &ida["ida_epi"], &ida["ida_epi_unread"]);
    check(&dir, "epi_fisso", &ida["epi_fisso"], &Value::Null);
}
