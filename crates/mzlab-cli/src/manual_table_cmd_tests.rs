//! `mzlab-cli peak table` on the SIM `.wiff` of `tools/wiff_sim.py` (`sim3`: a blank and three named samples, seven ions at a 0.8 s cycle, an isomer
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
        eprintln!("SKIP peak table: python3 not available");
        return None;
    }
    Some(d)
}

fn table(args: &[&Path], extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["peak".to_string(), "table".to_string()];
    a.extend(args.iter().map(|p| p.to_string_lossy().into_owned()));
    a.extend(extra.iter().map(|s| s.to_string()));
    execute(&a)
}

fn text(o: &crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// The windows file `w` in the fixture directory, as a `--windows` value.
fn windows(d: &Path, w: &str) -> String {
    let p = d.join("windows.json");
    std::fs::write(&p, w).unwrap();
    p.display().to_string()
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
    let t = text(&table(&[&d.join("sim3.wiff")], &["--windows", &set]));
    let r: Vec<Vec<&str>> = t.lines().map(|l| l.split(',').collect()).collect();
    assert_eq!(r[0].join(","), mzlab_core::manint::HEAD.trim_end());
    assert_eq!(r.len(), 5, "{t}");
    let key: Vec<String> = r[1..].iter().map(|x| x[..2].join(",")).collect();
    assert_eq!(
        key,
        [
            "253.1,Sample_A",
            "253.1,Sample_C",
            "251.1,Sample_B",
            "251.1,Sample_B"
        ],
        "channels in the order of the file, samples in order, the pair by start"
    );
    let apex: Vec<f64> = r[1..].iter().map(|x| x[4].parse().unwrap()).collect();
    let want = [at(1, 0, 0), at(3, 0, 0), at(2, 2, 0), at(2, 2, 1)];
    for (g, w) in apex.iter().zip(want) {
        assert!((g - w).abs() <= 0.014, "{apex:?} vs {want:?}");
    }
    assert_eq!((r[1][10], r[2][10]), ("edges", "zero"));
    assert!(
        r[1..].iter().all(|x| x[5].parse::<f64>().unwrap() > 0.0),
        "{t}"
    );
    let sums = text(&table(
        &[&d.join("sim3.wiff")],
        &["--windows", &set, "--sums"],
    ));
    let s: Vec<&str> = sums.lines().collect();
    assert_eq!(s.len(), 7, "{sums}");
    assert!(
        s[3].starts_with("253.1,sum,,") && s[6].starts_with("251.1,sum,,"),
        "{sums}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A window with `follow` on ionA of sample A reaches the same channel of every other sample, with its name (sample C has its own, by `names`) and an
/// origin; the blank has no peak there; `shifts=true` prints the same edges as JSON.
#[test]
fn a_window_applied_to_the_other_samples_is_the_same_bytes_as_its_shifts() {
    let Some(d) = fixture("follow") else { return };
    let w = d.join("sim3.wiff");
    let set = windows(
        &d,
        r#"[{"file":1,"tr":0,"a":1.89,"b":2.19,"follow":true,"label":"ionA peak","names":{"3":"C only"}}]"#,
    );
    let t = text(&table(&[&w], &["--windows", &set]));
    let r: Vec<Vec<&str>> = t.lines().skip(1).map(|l| l.split(',').collect()).collect();
    assert!(r.len() >= 2, "{t}");
    let by = |n: &str| r.iter().find(|x| x[1] == n).expect(n).clone();
    assert_eq!(
        (by("Sample_A")[3], by("Sample_A")[13]),
        ("ionA peak", "manual"),
        "{t}"
    );
    assert_eq!(
        by("Blank")[13],
        "no_peak",
        "a sample with no peak is integrated and marked, not hidden"
    );
    assert_eq!(by("Sample_B")[13], "applied");
    assert!(
        r.iter().all(|x| x[13] != "manual" || x[1] == "Sample_A"),
        "{t}"
    );
    let c = r.iter().find(|x| x[1] == "Sample_C").expect("sample C");
    assert_eq!(c[3], "C only", "a name for one sample alone");
    let again = text(&table(&[&w], &["--windows", &set]));
    assert_eq!(t, again, "deterministic");
    let j = text(&table(&[&w], &["--windows", &set, "--shifts"]));
    assert!(j.starts_with("[{\"file\":"), "{j}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--auto` finds the peaks of every sample without a windows file: the same bytes twice, every window of the answer is a row of the table, and a
/// `params` file moves the answer (a signal-to-noise nobody reaches finds no peak).
#[test]
fn auto_finds_the_peaks_of_every_sample_and_its_windows_are_the_table_rows() {
    let Some(d) = fixture("auto") else { return };
    let w = d.join("sim3.wiff");
    let j = text(&table(&[&w], &["--auto"]));
    assert!(j.starts_with("{\"tol\":"), "{j}");
    assert_eq!(j, text(&table(&[&w], &["--auto"])), "deterministic");
    let v = parse(j.as_bytes()).unwrap();
    let n = v.get("windows").unwrap().arr().len();
    assert!(n >= 4, "{j}");
    assert!(v.get("log").unwrap().arr().len() >= 4, "{j}");
    let set = windows(
        &d,
        &j[j.find("\"windows\":").unwrap() + 10..j.find(",\"log\"").unwrap()],
    );
    let t = text(&table(&[&w], &["--windows", &set]));
    assert_eq!(t.lines().count() - 1, n, "{t}");
    assert!(t.contains(",auto") || t.contains(",unmatched"), "{t}");
    let p = d.join("params.json");
    std::fs::write(&p, r#"{"sn":1000000}"#).unwrap();
    let none = text(&table(
        &[&w],
        &["--auto", "--params", &p.display().to_string()],
    ));
    assert!(none.contains("\"windows\":[]"), "{none}");
    assert_ne!(
        table(&[&w], &["--params", "x.json"]).code,
        0,
        "params without auto"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn settings_and_windows_that_name_nothing_fail() {
    let Some(d) = fixture("bad") else { return };
    let w = d.join("sim3.wiff");
    assert_ne!(table(&[&w], &[]).code, 0, "no windows");
    let ok = windows(&d, WINDOWS);
    assert_ne!(table(&[&w], &["--windows", &ok, "--sums=yes"]).code, 0);
    assert_ne!(
        table(&[&w], &["--windows", &ok, "--set", "baseline=zero"]).code,
        0
    );
    for bad in [
        r#"[{"file":4,"tr":0,"a":1,"b":2}]"#,
        r#"[{"file":1,"tr":7,"a":1,"b":2}]"#,
        r#"[{"file":1,"tr":0,"a":1,"b":2,"baseline":"line"}]"#,
    ] {
        let set = windows(&d, bad);
        assert_ne!(table(&[&w], &["--windows", &set]).code, 0, "{bad}");
    }
    let empty = windows(&d, "[]");
    assert_eq!(
        text(&table(&[&w], &["--windows", &empty])),
        mzlab_core::manint::HEAD
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// The `time_min` column: the time point each sample name carries (`sim_tempo.wiff`: a blank with a ratio, t0, 3 h, 8 h, a repeated 3 h and a name with
/// H20 that is no time), and `--times PATH` replacing or clearing it for the samples it names.
#[test]
fn the_time_point_of_each_sample_is_in_the_table_and_can_be_replaced() {
    let Some(d) = fixture("time") else { return };
    let w = d.join("sim_tempo.wiff");
    let list: Vec<String> = (0..6)
        .map(|i| format!(r#"{{"file":{i},"tr":0,"a":1.0,"b":2.5}}"#))
        .collect();
    let set = windows(&d, &format!("[{}]", list.join(",")));
    let pairs = |t: &str| -> Vec<(String, String)> {
        t.lines()
            .skip(1)
            .map(|l| {
                let c: Vec<&str> = l.split(',').collect();
                (c[1].to_string(), c[2].to_string())
            })
            .collect()
    };
    let want = |t: [&str; 6]| -> Vec<(String, String)> {
        [
            "blk 90-10",
            "RET t0 (2)",
            "RET 3h_2",
            "RET 8h (2)",
            "PERM 3h",
            "H20 demi",
        ]
        .iter()
        .zip(t)
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect()
    };
    let t = text(&table(&[&w], &["--windows", &set]));
    assert_eq!(pairs(&t), want(["", "0", "180", "480", "180", ""]), "{t}");
    let tp = d.join("times.json");
    std::fs::write(&tp, r#"{"RET 3h_2":45,"PERM 3h":null,"H20 demi":10}"#).unwrap();
    let set_t = tp.display().to_string();
    let t = text(&table(&[&w], &["--windows", &set, "--times", &set_t]));
    assert_eq!(pairs(&t), want(["", "0", "45", "480", "", "10"]), "{t}");
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
    for extra in [&["--windows", &set][..], &["--windows", &set, "--sums"][..]] {
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
    let o = table(&[&d.join("sim3_cut.wiff")], &["--windows", &set]);
    assert!(
        o.stderr.contains("warning: Sample_B.mzML:")
            && o.stderr.contains("warning: Sample_C.mzML:")
            && !o.stderr.contains("Sample_A"),
        "{}",
        o.stderr
    );
    assert_eq!(
        text(&o),
        text(&table(&[&d.join("sim3.wiff")], &["--windows", &set]))
    );
    let c = windows(&d, r#"[{"file":3,"tr":0,"a":1.97,"b":2.27}]"#);
    let o = table(&[&d.join("sim3_cut.wiff")], &["--windows", &c]);
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("warning: Sample_C.mzML:"), "{}", o.stderr);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn channels_lists_the_sim_channels_of_the_wiff() {
    let Some(d) = fixture("channels") else { return };
    let o = execute(&[
        "file".to_string(),
        "channels".to_string(),
        d.join("sim3.wiff").to_string_lossy().into_owned(),
    ]);
    let t = text(&o);
    assert_eq!(t.matches("\"family\":\"sim\"").count(), 7, "{t}");
    assert!(
        t.contains("\"label\":\"SIM 251.1 (+) 20 ms «ionC»\""),
        "{t}"
    );
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

/// The help lists the columns of the CSV header, in its order: both come from the same constants.
#[test]
fn help_lists_the_header_columns_in_order() {
    let h = super::help();
    let head = mzlab_core::manint::HEAD.trim_end();
    assert_eq!(head.split(',').count(), 14);
    assert!(
        h.contains(&format!("Columns, in this order: {head}.")),
        "{h}"
    );
    assert!(h.contains(mzlab_core::manint::EXT_HEAD.trim_start_matches(',')));
    for e in [
        mzlab_core::manint::SN_HEAD,
        mzlab_core::manint::FIG_HEAD,
        mzlab_core::manint::QUALITY_HEAD,
    ] {
        assert!(h.contains(e.trim_start_matches(',')), "{e}");
    }
    assert!(!h.contains("--set"), "peak table has no --set");
}
