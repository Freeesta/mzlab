//! A SIM (Q1MI) `.wiff` of a triple quadrupole (`tools/wiff_sim.py`: two samples, a positive experiment with three ions and a negative one with one,
//! two isobaric peaks on one ion): each ion is one SIM chromatogram with its m/z, dwell, polarity and name, in the engine, through the registry and
//! after a round trip through the mzML; a SIM method of two periods is refused with `periods=2`, an empty row does not cut the ions. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::mzml::ChromKind;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

/// The pairs in a folder of this test only: the tests run in parallel and must not rewrite each other's files.
fn generate() -> Option<(PathBuf, Value)> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("mzlab_wiff_sim_{}_{k}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
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
        eprintln!("SKIP wiff_sim: python3 not available");
        return None;
    }
    let json = serde_json::from_slice(&std::fs::read(out.join("sim.json")).unwrap()).unwrap();
    Some((out, json))
}

fn pair(dir: &Path) -> FileSet {
    let member = |file: &str| {
        let bytes = std::fs::read(dir.join(file)).unwrap();
        Membro {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![member("sim.wiff"), member("sim.wiff.scan")],
    }
}

fn want_id(n: u32, ion: &Value, j: usize) -> String {
    let pol = if ion["polarita"] == 1 {
        "positive"
    } else {
        "negative"
    };
    format!(
        "SIM SIC Q1={} sample={n} period=1 experiment={} transition={j} dwell={} polarity={pol} name={}",
        ion["q1"].as_f64().unwrap(),
        ion["esperimento"],
        ion["dwell"].as_f64().unwrap(),
        ion["name"].as_str().unwrap()
    )
}

/// The RTs of the local maxima above half of the highest point of a trace.
fn apexes(t: &[f64], y: &[f64]) -> Vec<f64> {
    let top = y.iter().copied().fold(0.0, f64::max);
    (1..y.len() - 1)
        .filter(|&i| y[i] > y[i - 1] && y[i] >= y[i + 1] && y[i] > 0.2 * top)
        .map(|i| t[i])
        .collect()
}

#[test]
fn every_ion_is_one_sim_chromatogram_in_the_engine() {
    let Some((dir, want)) = generate() else {
        return;
    };
    let ions = want["ioni"].as_array().unwrap();
    let ole = container(std::fs::read(dir.join("sim.wiff")).unwrap(), "sim").unwrap();
    let mut scan = std::fs::read(dir.join("sim.wiff.scan")).unwrap();
    for n in 1..=2u32 {
        let w = Wiff::open(&ole, "sim", Some(scan.len() as u64), n).unwrap();
        let run = w.run(&mut scan).unwrap();
        assert!(run.spectra.is_empty());
        assert!(run.chromatograms.iter().all(|c| c.kind != ChromKind::Srm));
        let sim: Vec<_> = run
            .chromatograms
            .iter()
            .filter(|c| c.kind == ChromKind::Sic)
            .collect();
        assert_eq!(sim.len(), ions.len());
        // the experiments alternate within a cycle: positive ions at the even ids, the negative one in its own experiment
        let mut j = [0usize; 2];
        for (c, ion) in sim.iter().zip(ions) {
            let e = ion["esperimento"].as_u64().unwrap() as usize - 1;
            assert_eq!(c.id, want_id(n, ion, j[e]));
            j[e] += 1;
            assert_eq!((c.q1, c.q3), (ion["q1"].as_f64(), None));
            assert_eq!(c.time.len(), want["cicli"].as_u64().unwrap() as usize);
            let rts = &want["campioni"][n.to_string()][ion["q1"].as_f64().unwrap().to_string()];
            let got = apexes(&c.time, &c.intensity);
            let rts: Vec<f64> = rts
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            assert_eq!(got.len(), rts.len(), "{}: {got:?}", c.id);
            assert!(
                got.iter().zip(&rts).all(|(a, b)| (a - b).abs() < 0.03),
                "{}: {got:?}",
                c.id
            );
        }
    }
}

#[test]
fn the_registry_and_the_mzml_keep_the_sim_channels() {
    let Some((dir, want)) = generate() else {
        return;
    };
    let ions = want["ioni"].as_array().unwrap();
    let mut l = lettori::apri(pair(&dir), &Limiti::sito()).unwrap();
    assert_eq!(l.campioni().len(), 2);
    for c in 0..2 {
        let list = l.cromatogrammi(c).unwrap();
        let sim: Vec<_> = list.iter().filter(|v| v.tipo == ChromKind::Sic).collect();
        assert_eq!(sim.len(), ions.len(), "sample {}", c + 1);
        assert!(sim[0]
            .id
            .starts_with(&format!("SIM SIC Q1=237.1 sample={} ", c + 1)));
    }
    let bytes = mzlab_core::lettori_mzml::scrivi(&mut *l, 1, Vec::new(), &mut |_, _| {}).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(text.matches("MS:1000627").count(), ions.len());
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let chroms = m.chromatograms(0).unwrap();
    // in the list of the headers a SIM ion is a transition with no Q3: the whole MRM path of the page serves it
    let sim: Vec<_> = chroms
        .iter()
        .filter(|c| c.id.starts_with("SIM SIC "))
        .collect();
    assert_eq!(sim.len(), ions.len());
    for (c, ion) in sim.iter().zip(ions) {
        assert_eq!(
            (c.kind.as_str(), c.q1, c.q3),
            ("srm", ion["q1"].as_f64(), None)
        );
        assert_eq!(
            (c.dwell, c.name.as_str()),
            (ion["dwell"].as_f64(), ion["name"].as_str().unwrap())
        );
    }
}

#[test]
fn a_sim_method_of_two_periods_is_refused_and_an_empty_row_is_left_out() {
    let Some((dir, want)) = generate() else {
        return;
    };
    let read = |name: &str| {
        let ole = container(
            std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
            name,
        )
        .unwrap();
        let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
        Wiff::open(&ole, name, Some(scan.len() as u64), 1).and_then(|w| w.run(&mut scan))
    };
    let err = read("sim_periodi").err().unwrap();
    assert_eq!(err.key, "err.file.wiff");
    assert!(err
        .params
        .iter()
        .any(|(k, v)| *k == "detail" && v == "periods=2"));
    let run = read("sim_vuoti").unwrap();
    let cycles = want["cicli"].as_u64().unwrap() as usize;
    let sim: Vec<_> = run
        .chromatograms
        .iter()
        .filter(|c| c.kind == ChromKind::Sic)
        .collect();
    assert_eq!(sim.len(), want["ioni"].as_array().unwrap().len());
    for c in sim {
        let n = if c.id.contains("experiment=1 ") {
            cycles - 1
        } else {
            cycles
        };
        assert_eq!((c.time.len(), c.intensity.len()), (n, n), "{}", c.id);
    }
}

#[test]
fn sim_channels_of_a_method_without_ions_have_a_place_but_no_q1() {
    let Some((dir, _)) = generate() else {
        return;
    };
    let ole = container(
        std::fs::read(dir.join("sim_senzaq1.wiff")).unwrap(),
        "sim_senzaq1",
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join("sim_senzaq1.wiff.scan")).unwrap();
    let w = Wiff::open(&ole, "sim_senzaq1", Some(scan.len() as u64), 2).unwrap();
    let run = w.run(&mut scan).unwrap();
    let sim: Vec<_> = run
        .chromatograms
        .iter()
        .filter(|c| c.kind == ChromKind::Sic)
        .collect();
    assert_eq!(
        sim.len(),
        7,
        "{:?}",
        run.chromatograms.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    for (j, c) in sim.iter().enumerate() {
        assert!(
            c.id.starts_with("SIM SIC sample=2 ") && !c.id.contains("Q1="),
            "{}",
            c.id
        );
        assert!(c.id.contains(&format!("transition={j}")), "{}", c.id);
        assert_eq!((c.q1, c.q3), (None, None));
    }
}
