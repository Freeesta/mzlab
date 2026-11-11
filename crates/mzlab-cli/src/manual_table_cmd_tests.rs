//! `mzlab-cli manual-table` on the SIM `.wiff` of `tools/wiff_sim.py` (`sim3`: a blank and three named samples, seven ions at a 0.8 s cycle, an isomer
//! pair; skipped without python3): windows drawn at the peaks the writer put give one row each, grouped by channel, with an optional sum row; the same
//! bytes from the `.wiff` and from its samples written as mzML; a copy of the `.wiff.scan` cut at 60 % warns and measures the points before the cut.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};
use mzlab_core::readers::{self, FileSet, Limits, Member};

use crate::execute;

fn fixture(tag: &str) -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_cli_manint_{tag}_{}", std::process::id()));
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
        eprintln!("SKIP manual-table: python3 not available");
        return None;
    }
    Some(d)
}

fn table(args: &[&Path], extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["manual-table".to_string()];
    a.extend(args.iter().map(|p| p.to_string_lossy().into_owned()));
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn text(o: &crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// The windows file `w` in the fixture directory, as a `--set` value.
fn windows(d: &Path, w: &str) -> String {
    let p = d.join("windows.json");
    std::fs::write(&p, w).unwrap();
    format!("windows={}", p.display())
}

/// Windows of 0.3 min around the peaks the writer put: ionA (transition 0) of samples A and C, the isomer pair of ionC (transition 2) of sample B.
const WINDOWS: &str = r#"[{"file":3,"tr":0,"a":1.97,"b":2.27},{"file":1,"tr":0,"a":1.89,"b":2.19,"baseline":"edges"},
{"file":2,"tr":2,"a":6.75,"b":7.05},{"file":2,"tr":2,"a":6.33,"b":6.63}]"#;

#[test]
fn one_row_per_window_grouped_by_channel_at_the_peaks_written() {
    let Some(d) = fixture("rows") else { return };
    let truth = parse(&std::fs::read(d.join("sim3.json")).unwrap()).unwrap();
    let peaks = truth.get("picchi").unwrap().arr();
    let at = |s: usize, j: usize, k: usize| peaks[s].arr()[j].arr()[k].f64().unwrap();
    let set = windows(&d, WINDOWS);
    let t = text(&table(&[&d.join("sim3.wiff")], &["--set", &set]));
    let r: Vec<Vec<&str>> = t.lines().map(|l| l.split(',').collect()).collect();
    assert_eq!(r[0].join(","), mzlab_core::manint::HEAD.trim_end());
    assert_eq!(r.len(), 5, "{t}");
    let key: Vec<String> = r[1..].iter().map(|x| x[..3].join(",")).collect();
    assert_eq!(
        key,
        [
            "253.1,ionA,Sample_A",
            "253.1,ionA,Sample_C",
            "251.1,ionC,Sample_B",
            "251.1,ionC,Sample_B"
        ],
        "channels in the order of the file, samples in order, the pair by start"
    );
    let apex: Vec<f64> = r[1..].iter().map(|x| x[5].parse().unwrap()).collect();
    let want = [at(1, 0, 0), at(3, 0, 0), at(2, 2, 0), at(2, 2, 1)];
    for (g, w) in apex.iter().zip(want) {
        assert!((g - w).abs() <= 0.014, "{apex:?} vs {want:?}");
    }
    assert_eq!((r[1][9], r[2][9]), ("edges", "zero"));
    assert!(
        r[1..].iter().all(|x| x[6].parse::<f64>().unwrap() > 0.0),
        "{t}"
    );
    let sums = text(&table(
        &[&d.join("sim3.wiff")],
        &["--set", &set, "--set", "sums=true"],
    ));
    let s: Vec<&str> = sums.lines().collect();
    assert_eq!(s.len(), 7, "{sums}");
    assert!(
        s[3].starts_with("253.1,ionA,sum,,,,") && s[6].starts_with("251.1,ionC,sum,,,,"),
        "{sums}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn settings_and_windows_that_name_nothing_fail() {
    let Some(d) = fixture("bad") else { return };
    let w = d.join("sim3.wiff");
    assert_ne!(table(&[&w], &[]).code, 0, "no windows");
    let ok = windows(&d, WINDOWS);
    assert_ne!(table(&[&w], &["--set", &ok, "--set", "sums=yes"]).code, 0);
    assert_ne!(
        table(&[&w], &["--set", &ok, "--set", "baseline=zero"]).code,
        0
    );
    for bad in [
        r#"[{"file":4,"tr":0,"a":1,"b":2}]"#,
        r#"[{"file":1,"tr":7,"a":1,"b":2}]"#,
        r#"[{"file":1,"tr":0,"a":1,"b":2,"baseline":"line"}]"#,
    ] {
        let set = windows(&d, bad);
        assert_ne!(table(&[&w], &["--set", &set]).code, 0, "{bad}");
    }
    let empty = windows(&d, "[]");
    assert_eq!(
        text(&table(&[&w], &["--set", &empty])),
        mzlab_core::manint::HEAD
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_wiff_and_its_mzml_give_the_same_bytes() {
    let Some(d) = fixture("mzml") else { return };
    let member = |file: &str| {
        let bytes = std::fs::read(d.join(file)).unwrap();
        Member {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![member("sim3.wiff"), member("sim3.wiff.scan")],
    };
    let mut l = readers::apri(set, &Limits::sito()).unwrap();
    let mut files = Vec::new();
    for (c, name) in ["Blank", "Sample_A", "Sample_B", "Sample_C"]
        .iter()
        .enumerate()
    {
        let bytes =
            mzlab_core::readers_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        let p = d.join(format!("{name}.mzML"));
        std::fs::write(&p, bytes).unwrap();
        files.push(p);
    }
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let set = windows(&d, WINDOWS);
    for extra in [
        &["--set", &set][..],
        &["--set", &set, "--set", "sums=true"][..],
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
    // samples A and B keep the points of their first minutes, sample C lies wholly past the cut: a window on it names no times
    let set = windows(
        &d,
        r#"[{"file":1,"tr":0,"a":1.89,"b":2.19},{"file":2,"tr":0,"a":1.93,"b":2.23}]"#,
    );
    let o = table(&[&d.join("sim3_cut.wiff")], &["--set", &set]);
    assert!(
        o.stderr.contains("warning: Sample_B.mzML:")
            && o.stderr.contains("warning: Sample_C.mzML:")
            && !o.stderr.contains("Sample_A"),
        "{}",
        o.stderr
    );
    assert_eq!(
        text(&o),
        text(&table(&[&d.join("sim3.wiff")], &["--set", &set]))
    );
    let c = windows(&d, r#"[{"file":3,"tr":0,"a":1.97,"b":2.27}]"#);
    let o = table(&[&d.join("sim3_cut.wiff")], &["--set", &c]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("warning: Sample_C.mzML:"), "{}", o.stderr);
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

#[test]
fn a_lone_unnamed_sample_is_named_after_its_file_in_any_case() {
    // the reader names it «<file> #<number>» with the extension as the file has it (an upper-case .WIFF, a container number other than 1)
    use super::wiff_name;
    assert_eq!(wiff_name("U", "U.WIFF #1", &[]), "U.mzML");
    assert_eq!(wiff_name("x", "x.wiff #3", &[]), "x.mzML");
    assert_eq!(wiff_name("x", "x.wiff #3a", &[]), "x.wiff #3a.mzML");
    assert_eq!(
        wiff_name("x", "Named", &["named.mzML".into()]),
        "Named.wiff.mzML"
    );
}
