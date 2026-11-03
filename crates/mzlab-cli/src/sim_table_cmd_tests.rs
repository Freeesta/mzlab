//! `mzlab-cli sim-table` on the SIM `.wiff` of `tools/wiff_sim.py` (`sim3`: three named samples, seven ions; skipped without python3): one row per ion,
//! one column per sample, apexes where the writer put them, the same bytes from the `.wiff` and from its samples written as mzML, and a copy of the
//! `.wiff.scan` cut at 60 % gives the points before the cut with a warning.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};

use crate::execute;

fn fixture(tag: &str) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_cli_simtab_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let ok = Command::new("python3").arg(script).arg(&d).output().ok().filter(|o| o.status.success());
    if ok.is_none() {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "python3 not available");
        eprintln!("SKIP sim-table: python3 not available");
        return None;
    }
    Some(d)
}

fn table(args: &[&Path], extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["sim-table".to_string()];
    a.extend(args.iter().map(|p| p.to_string_lossy().into_owned()));
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn text(o: &crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn one_row_per_ion_one_column_per_sample() {
    let Some(d) = fixture("rows") else { return };
    let truth = parse(&std::fs::read(d.join("sim3.json")).unwrap()).unwrap();
    let area = text(&table(&[&d.join("sim3.wiff")], &[]));
    let lines: Vec<&str> = area.lines().collect();
    assert_eq!(lines[0], "q1,name,Sample_A,Sample_B,Sample_C");
    let ions = truth.get("ioni").unwrap().arr();
    assert_eq!(lines.len(), ions.len() + 1, "{area}");
    let rt = text(&table(&[&d.join("sim3.wiff")], &["--set", "output=apex_rt"]));
    let apici = truth.get("apici").unwrap().arr();
    for (j, (ion, line)) in ions.iter().zip(rt.lines().skip(1)).enumerate() {
        let cells: Vec<&str> = line.split(',').collect();
        assert_eq!(cells[0].parse::<f64>().unwrap(), ion.get("q1").and_then(Value::f64).unwrap());
        assert_eq!(cells[1], ion.get("name").and_then(Value::str).unwrap());
        for s in 0..3 {
            let want = apici[s].arr()[j].f64().unwrap();
            let got: f64 = cells[2 + s].parse().unwrap();
            assert!((got - want).abs() <= 0.021, "{line}: sample {s} {got} vs {want}");
        }
        assert!(lines[j + 1].split(',').skip(2).all(|a| a.parse::<f64>().unwrap() > 0.0), "{area}");
    }
    let bad = table(&[&d.join("sim3.wiff")], &["--set", "output=height"]);
    assert_ne!(bad.code, 0);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_wiff_and_its_mzml_give_the_same_bytes() {
    let Some(d) = fixture("mzml") else { return };
    let member = |file: &str| {
        let bytes = std::fs::read(d.join(file)).unwrap();
        Membro { nome: file.into(), dim: bytes.len() as u64, dati: Box::new(bytes), sha1: None }
    };
    let set = FileSet { membri: vec![member("sim3.wiff"), member("sim3.wiff.scan")] };
    let mut l = lettori::apri(set, &Limiti::sito()).unwrap();
    let mut files = Vec::new();
    for (c, name) in ["Sample_A", "Sample_B", "Sample_C"].iter().enumerate() {
        let bytes = mzlab_core::lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        let p = d.join(format!("{name}.mzML"));
        std::fs::write(&p, bytes).unwrap();
        files.push(p);
    }
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    for extra in [&[][..], &["--set", "output=apex_rt"][..]] {
        assert_eq!(text(&table(&refs, extra)), text(&table(&[&d.join("sim3.wiff")], extra)));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_scan_file_cut_short_gives_the_points_before_the_cut_and_a_warning() {
    let Some(d) = fixture("cut") else { return };
    let full = text(&table(&[&d.join("sim3.wiff")], &[]));
    let o = table(&[&d.join("sim3_cut.wiff")], &[]);
    let cut = text(&o);
    assert!(o.stderr.contains("warning: Sample_B.mzML:") && o.stderr.contains("warning: Sample_C.mzML:"), "{}", o.stderr);
    assert!(!o.stderr.contains("Sample_A"), "{}", o.stderr);
    let col = |t: &str, k: usize| -> Vec<String> { t.lines().skip(1).map(|l| l.split(',').nth(k).unwrap().to_string()).collect() };
    assert_eq!(col(&cut, 2), col(&full, 2), "the first sample lies before the cut");
    let b = col(&cut, 3);
    assert!(b[..6].iter().all(|v| !v.is_empty()) && b[6].is_empty(), "{cut}");
    assert!(col(&cut, 4).iter().all(String::is_empty), "{cut}");
    let _ = std::fs::remove_dir_all(&d);
}
