//! `mzlab-cli sim-table` on the SIM `.wiff` of `tools/wiff_sim.py` (`sim3`: a blank and three named samples, seven ions at a 0.8 s cycle, an isomer pair;
//! skipped without python3): one row per sample, one column per ion, every peak where the writer put it (the pair as two peaks), the same bytes from the
//! `.wiff` and from its samples written as mzML, and a copy of the `.wiff.scan` cut at 60 % gives the points before the cut with a warning.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};

use crate::execute;

fn fixture(tag: &str) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_cli_simtab_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&d)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
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
fn one_row_per_sample_one_column_per_ion_and_every_peak() {
    let Some(d) = fixture("rows") else { return };
    let truth = parse(&std::fs::read(d.join("sim3.json")).unwrap()).unwrap();
    let ions = truth.get("ioni").unwrap().arr();
    let samples: Vec<&str> = truth
        .get("campioni")
        .unwrap()
        .arr()
        .iter()
        .map(|v| v.str().unwrap())
        .collect();
    let area = text(&table(&[&d.join("sim3.wiff")], &[]));
    let lines: Vec<&str> = area.lines().collect();
    let head: Vec<String> = ions
        .iter()
        .map(|i| format!("{}", i.get("q1").and_then(Value::f64).unwrap()))
        .collect();
    assert_eq!(lines[0], format!("sample,{}", head.join(",")));
    assert_eq!(lines.len(), samples.len() + 1, "{area}");
    assert_eq!(
        lines[1],
        format!("Blank{}", ",0.00000E+00".repeat(ions.len())),
        "the blank: every channel, no peak"
    );
    let peaks = text(&table(&[&d.join("sim3.wiff")], &["--set", "output=peaks"]));
    let picchi = truth.get("picchi").unwrap().arr();
    for (s, name) in samples.iter().enumerate() {
        for (j, ion) in ions.iter().enumerate() {
            let q1 = format!("{:.4}", ion.get("q1").and_then(Value::f64).unwrap());
            let got: Vec<f64> = peaks
                .lines()
                .filter(|l| l.starts_with(&format!("{name},{q1},")))
                .filter_map(|l| l.split(',').nth(4).and_then(|v| v.parse().ok()))
                .collect();
            let want: Vec<f64> = picchi[s].arr()[j]
                .arr()
                .iter()
                .map(|v| v.f64().unwrap())
                .collect();
            assert_eq!(got.len(), want.len(), "{name} {q1}: {got:?} vs {want:?}");
            assert!(
                got.iter().zip(&want).all(|(g, w)| (g - w).abs() <= 0.014),
                "{name} {q1}: {got:?} vs {want:?}"
            );
        }
    }
    let flip = text(&table(
        &[&d.join("sim3.wiff")],
        &["--set", "layout=channels", "--set", "output=n_peaks"],
    ));
    assert_eq!(
        flip.lines().next().unwrap(),
        format!("channel,name,{}", samples.join(","))
    );
    assert_eq!(flip.lines().nth(3).unwrap(), "251.1,ionC,0,2,2,2", "{flip}");
    for bad in ["output=area", "layout=rows"] {
        assert_ne!(table(&[&d.join("sim3.wiff")], &["--set", bad]).code, 0);
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn groups_order_the_rows_and_add_a_group_column() {
    let Some(d) = fixture("groups") else { return };
    let g = text(&table(
        &[&d.join("sim3.wiff")],
        &[
            "--set",
            "output=n_peaks",
            "--set",
            "group.Sample_B=late",
            "--set",
            "group.Blank=early",
        ],
    ));
    let rows: Vec<String> = g
        .lines()
        .map(|l| l.splitn(3, ',').take(2).collect::<Vec<_>>().join(","))
        .collect();
    assert_eq!(
        rows,
        [
            "sample,group",
            "Blank,early",
            "Sample_B,late",
            "Sample_A,",
            "Sample_C,"
        ],
        "groups in the order they first appear, then the rest: {g}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_wiff_and_its_mzml_give_the_same_bytes() {
    let Some(d) = fixture("mzml") else { return };
    let member = |file: &str| {
        let bytes = std::fs::read(d.join(file)).unwrap();
        Membro {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![member("sim3.wiff"), member("sim3.wiff.scan")],
    };
    let mut l = lettori::apri(set, &Limiti::sito()).unwrap();
    let mut files = Vec::new();
    for (c, name) in ["Blank", "Sample_A", "Sample_B", "Sample_C"]
        .iter()
        .enumerate()
    {
        let bytes =
            mzlab_core::lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        let p = d.join(format!("{name}.mzML"));
        std::fs::write(&p, bytes).unwrap();
        files.push(p);
    }
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    for extra in [
        &[][..],
        &["--set", "output=peaks"][..],
        &["--set", "output=apex_rt", "--set", "layout=channels"][..],
    ] {
        assert_eq!(
            text(&table(&refs, extra)),
            text(&table(&[&d.join("sim3.wiff")], extra))
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_scan_file_cut_short_gives_the_points_before_the_cut_and_a_warning() {
    let Some(d) = fixture("cut") else { return };
    let full = text(&table(&[&d.join("sim3.wiff")], &[]));
    let o = table(&[&d.join("sim3_cut.wiff")], &[]);
    let cut = text(&o);
    assert!(
        o.stderr.contains("warning: Sample_B.mzML:")
            && o.stderr.contains("warning: Sample_C.mzML:"),
        "{}",
        o.stderr
    );
    assert!(!o.stderr.contains("Sample_A"), "{}", o.stderr);
    let rows = |t: &str| -> Vec<String> { t.lines().map(str::to_string).collect() };
    let (cut, full) = (rows(&cut), rows(&full));
    assert_eq!(
        cut[..3],
        full[..3],
        "the blank and the first sample lie before the cut"
    );
    let b: Vec<&str> = cut[3].split(',').skip(1).collect();
    assert!(
        b[..3].iter().all(|v| v.parse::<f64>().unwrap() > 0.0)
            && b[3..].iter().all(|v| *v == "0.00000E+00"),
        "{cut:?}"
    );
    assert!(cut[4].split(',').skip(1).all(str::is_empty), "{cut:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn channels_lists_the_sim_channels_of_the_wiff() {
    let Some(d) = fixture("channels") else { return };
    let o = execute(&[
        "channels".to_string(),
        d.join("sim3.wiff").to_string_lossy().into_owned(),
    ]);
    let t = text(&o);
    assert_eq!(t.matches("\"fam\":\"sim\"").count(), 7, "{t}");
    assert!(t.contains("\"et\":\"SIM 251.1 (+) 20 ms «ionC»\""), "{t}");
    let _ = std::fs::remove_dir_all(&d);
}
