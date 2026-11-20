//! Tests of `features_note`: the recipe line, the profile warning and the `features-notes/1` file for unit-resolution files, the threshold on both sides
//! of one half of the MS1 scans, and the same notices as the `estimates/1` of the Flow.

use super::*;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};
use mzlab_ops::run::feat::lr_note;
use mzlab_ops::run::feat::run_estimate::estimate_of;

/// A unit-resolution mzML of `n` MS1 scans: one ion at 152.2 eluting at the middle; the first `profiles` scans are profile points (0.1 Da grid, FWHM
/// 0.7 Da), the others one centroid each.
fn mzml(profiles: usize, n: usize) -> Vec<u8> {
    let head = Header {
        run: "q1".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &head, Options::default()).unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let rt = 0.5 + 0.02 * i as f64;
        let h = 2e5 * (-0.5 * ((i as f64 - n as f64 / 2.0) / 4.0).powi(2)).exp();
        let profile = i < profiles;
        let (mz, y): (Vec<f64>, Vec<f32>) = if profile {
            let mz: Vec<f64> = (0..300).map(|k| 120.0 + 0.1 * k as f64).collect();
            let y = mz
                .iter()
                .map(|&m| (h * (-0.5 * ((m - 152.2) / 0.3).powi(2)).exp() + 20.0) as f32)
                .collect();
            (mz, y)
        } else {
            (vec![152.2], vec![(h + 20.0) as f32])
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt,
            polarity: 1,
            profile,
            filter: "+ p ESI Q1MS [120.000-150.000]".into(),
            precursor: None,
            mz,
            intensity: y,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The file with `profiles` profile scans among 40, its index kept in a folder of its own, and what `features` makes of it in that folder.
struct Case {
    dir: std::path::PathBuf,
    note: Note,
}

impl Case {
    fn new(profiles: usize) -> Case {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("mzlab_fnote_{k}_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (f, idx) = (dir.join("q1.mzML"), dir.join("0.mzidx"));
        std::fs::write(&f, mzml(profiles, 40)).unwrap();
        crate::build_index_of(&f, "", &idx, mzlab_core::budget_auto::native_index_budget())
            .unwrap();
        let mut index = crate::open(&idx).unwrap();
        let recipe = Recipe::for_index(&mut index).unwrap();
        let note = note_of(&recipe, &index);
        Case { dir, note }
    }
    fn lines(&self) -> Vec<String> {
        lines("q1.mzML", &self.note)
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn unit_resolution_profile_data_get_the_recipe_line_and_the_warning() {
    let l = Case::new(40).lines();
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(
        l[0].starts_with("recipe q1.mzML: unit resolution (1 Da channels centred on n"),
        "{}",
        l[0]
    );
    assert!(
        l[0].ends_with("40 of 40 MS1 scans in profile mode"),
        "{}",
        l[0]
    );
    assert!(
        l[1].starts_with("warning: q1.mzML: 40 of 40 MS1 scans are profile mode"),
        "{}",
        l[1]
    );
    assert!(l[1].ends_with("(warn.features.profileUnit)"), "{}", l[1]);
}

#[test]
fn centroided_unit_resolution_data_get_the_recipe_line_only() {
    let l = Case::new(0).lines();
    assert_eq!(l.len(), 1, "{l:?}");
    assert!(
        l[0].ends_with("0 of 40 MS1 scans in profile mode"),
        "{}",
        l[0]
    );
}

#[test]
fn the_warning_starts_at_half_of_the_ms1_scans_profile() {
    // 19 of 40 is a mixed file below the share the recipe itself takes a per-point floor at; 20 of 40 is the first above it
    let below = Case::new(19).lines();
    assert_eq!(below.len(), 1, "{below:?}");
    assert!(below[0].ends_with("19 of 40 MS1 scans in profile mode"));
    let at = Case::new(20).lines();
    assert_eq!(at.len(), 2, "{at:?}");
    assert!(at[1].starts_with("warning: q1.mzML: 20 of 40 MS1 scans are profile mode"));
}

#[test]
fn a_file_with_no_ms1_scan_has_no_notice() {
    let n = Note {
        unit: true,
        defect: 0.0,
        floor: 1.0,
        min_height: 3.0,
        profile: 0,
        scans: 0,
        notices: lr_note::notices_of(0, 0),
    };
    assert!(n.notices.is_empty());
    assert_eq!(lines("q1.mzML", &n).len(), 1);
}

#[test]
fn the_notes_say_what_the_stderr_lines_say() {
    let c = Case::new(20);
    let t = notes_text(&[("q1.mzML".to_string(), c.note.clone())]);
    assert!(
        t.starts_with("{\"schema\":\"features-notes/1\",\"files\":[{\"file\":\"q1.mzML\",\"axis\":\"Da\",\"defect\":"),
        "{t}"
    );
    assert!(
        t.ends_with(
            ",\"profile_scans\":20,\"ms1_scans\":40,\"notices\":[{\"key\":\"warn.features.profileUnit\",\"params\":{\"profile\":20,\"scans\":40}}]}]}\n"
        ),
        "{t}"
    );
    let below = notes_text(&[("q1.mzML".to_string(), Case::new(19).note.clone())]);
    assert!(below.ends_with("\"notices\":[]}]}\n"), "{below}");
}

#[test]
fn a_ppm_axis_file_has_notes_without_a_notice() {
    let mut n = Case::new(40).note.clone();
    n.unit = false;
    n.notices.clear();
    assert!(lines("q1.mzML", &n).is_empty());
    let t = notes_text(&[("q1.mzML".to_string(), n)]);
    assert!(t.contains("\"axis\":\"ppm\""), "{t}");
    assert!(t.ends_with("\"notices\":[]}]}\n"), "{t}");
}

/// A folder with the series `a.mzML` (all profile), `b.mzML` (centroid) and `c.mzML` (19 of 40 profile), in that order.
fn series() -> (std::path::PathBuf, Vec<std::path::PathBuf>) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("mzlab_fseries_{k}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let files: Vec<_> = [("a", 40), ("b", 0), ("c", 19)]
        .iter()
        .map(|(n, p)| {
            let f = d.join(format!("{n}.mzML"));
            std::fs::write(&f, mzml(*p, 40)).unwrap();
            f
        })
        .collect();
    (d, files)
}

#[test]
fn the_notes_of_a_series_hold_one_entry_per_file_in_the_given_order() {
    let (d, files) = series();
    let tmp = d.join("tmp");
    std::fs::create_dir_all(&tmp).unwrap();
    let labels = vec![String::new(); 3];
    crate::features_of(&files, &labels, "", &tmp).unwrap();
    let doc = mzlab_core::json::parse(&std::fs::read(tmp.join("0.notes.json")).unwrap()).unwrap();
    let list = doc.get("files").unwrap().arr();
    let got: Vec<(String, usize, usize)> = list
        .iter()
        .map(|e| {
            let n = |k: &str| e.get(k).and_then(|v| v.f64()).unwrap() as usize;
            (
                e.get("file").and_then(|v| v.str()).unwrap().to_string(),
                n("profile_scans"),
                e.get("notices").unwrap().arr().len(),
            )
        })
        .collect();
    // `tests/test_features_notes_schema.py` checks these two documents against their schemas
    if let Some(dir) = std::env::var_os("MZLAB_JSON_SAMPLES") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::copy(tmp.join("0.notes.json"), dir.join("features-notes.json")).unwrap();
        let mut index = crate::open(&tmp.join("0.mzidx")).unwrap();
        let (estimate, _) = estimate_of(&mut index, "a.mzML", &[]).unwrap();
        std::fs::write(dir.join("estimates.json"), estimate).unwrap();
    }
    let name = |n: &str| d.join(format!("{n}.mzML")).display().to_string();
    assert_eq!(
        got,
        vec![(name("a"), 40, 1), (name("b"), 0, 0), (name("c"), 19, 0)]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_command_writes_the_notes_at_the_path_asked_for_and_nothing_without_it() {
    let (d, files) = series();
    let to = d.join("notes.json");
    let mut args = vec!["features".to_string()];
    args.extend(files.iter().map(|f| f.display().to_string()));
    let plain = crate::execute(&args);
    assert_eq!(plain.code, 0, "{}", plain.stderr);
    assert!(!to.exists());
    args.extend(["--notes".to_string(), to.display().to_string()]);
    let with = crate::execute(&args);
    assert_eq!(with.code, 0, "{}", with.stderr);
    // the table is the same bytes with and without the notes
    assert_eq!(plain.stdout, with.stdout);
    let doc = mzlab_core::json::parse(&std::fs::read(&to).unwrap()).unwrap();
    assert_eq!(
        doc.get("schema").and_then(|v| v.str()),
        Some("features-notes/1")
    );
    assert_eq!(doc.get("files").unwrap().arr().len(), 3);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_notes_of_a_file_and_the_flow_estimate_carry_the_same_notices() {
    let c = Case::new(40);
    let f = c.dir.join("q1.mzML");
    crate::features_of(&[f.clone()], &[String::new()], "", &c.dir).unwrap();
    let notes = std::fs::read_to_string(c.dir.join("0.notes.json")).unwrap();
    // the notices member of the only entry, as written, closes the document
    let at = notes.find("\"notices\":").unwrap() + "\"notices\":".len();
    let notices = notes[at..].trim_end().strip_suffix("}]}").unwrap();
    assert!(notices.starts_with("[{"), "{notes}");
    // the function behind the page's `flow_estimate` and the app's `flow run`
    let mut index = crate::open(&c.dir.join("0.mzidx")).unwrap();
    let (estimate, _) = estimate_of(&mut index, "q1.mzML", &[]).unwrap();
    assert!(
        estimate.contains(&format!("\"notices\":{notices}")),
        "{estimate}\n{notes}"
    );
}
