//! The shape gates of `quant.targets` (`min_half`, `max_fwhm`, `max_asym`) on a synthetic set: one real peak, weak noise blips at S/N 3 to 5 in a
//! matrix control and a blank, and the same files with the gates on. The defaults (gates off) give the same states and flags as before the gates.

use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::quant_targets::run;

mod common_quant;
use common_quant::{mzml, Ion};

fn index_of(ions: &[Ion], seed: u64) -> Index<Vec<u8>> {
    let m = mzml(ions, seed);
    let mut src = m.clone();
    let p = prepare(&mut src, m.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &m, 64 << 20).unwrap().0;
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

/// A real peak plus eight one-scan blips of rising height (S/N about 1 to 7 on the 100 +- 20 baseline) at their own m/z.
fn ions(real: bool) -> Vec<Ion> {
    let mut v = vec![Ion(200.0, 5.0, if real { 5000.0 } else { 0.0 }, 0.1)];
    v.extend((0..8).map(|i| {
        Ion(
            210.0 + 10.0 * f64::from(i),
            5.0,
            30.0 + 8.0 * f64::from(i),
            0.02,
        )
    }));
    v
}

fn targets() -> String {
    let mut t = String::from("id,mz,rt_min,rt_max\nreal,200.0,4.0,6.0\n");
    for i in 0..8 {
        t += &format!("blip{i},{},4.0,6.0\n", 210 + 10 * i);
    }
    t
}

/// The matrix control (real peak and blips) and a blank (blips only), the table and the log for the given parameters.
fn go(p: &[(&str, &str)]) -> (String, String) {
    let files = [
        ("Matrix.mzML", ions(true), 7),
        ("Blank_01.mzML", ions(false), 9),
    ];
    let names: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
    let mut open = |k: usize| Ok(index_of(&files[k].1, files[k].2));
    let p: Vec<(String, Value)> = p
        .iter()
        .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
        .collect();
    let o = run(&mut open, &names, targets().as_bytes(), &p).unwrap();
    (o.csv, o.log)
}

/// `file|id` to (state, flag, S/N) of every row of a table.
fn rows(csv: &str) -> std::collections::BTreeMap<String, (String, String, String)> {
    csv.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            (
                format!("{}|{}", c[0], c[1]),
                (c[7].into(), c[6].into(), c[4].into()),
            )
        })
        .collect()
}

fn detected(csv: &str) -> Vec<String> {
    let mut v: Vec<String> = rows(csv)
        .into_iter()
        .filter(|(_, r)| r.0 == "detected")
        .map(|(k, _)| k)
        .collect();
    v.sort();
    v
}

#[test]
fn gates_off_is_the_old_behaviour_and_blips_are_detected_in_matrix_and_blank() {
    let (old, _) = go(&[]);
    // explicit zeros are the defaults: the same bytes
    let (zero, _) = go(&[("min_half", "0"), ("max_fwhm", "0"), ("max_asym", "0")]);
    assert_eq!(old, zero);
    let d = detected(&old);
    // the old behaviour: weak one-scan blips pass S/N 3 and five points, in the matrix and in the blank
    assert!(
        d.contains(&"Matrix.mzML|blip1".into()) && d.contains(&"Matrix.mzML|blip2".into()),
        "{d:?}"
    );
    assert!(
        d.iter().filter(|k| k.starts_with("Blank_01")).count() >= 3,
        "{d:?}"
    );
    let r = rows(&old);
    let sn = |k: &str| r[k].2.parse::<f64>().unwrap();
    assert!(
        (3.0..5.0).contains(&sn("Matrix.mzML|blip2"))
            && (3.0..5.0).contains(&sn("Matrix.mzML|blip1"))
    );
}

#[test]
fn sn_alone_keeps_taller_blips_and_a_half_height_gate_drops_them_all() {
    let (sn5, _) = go(&[("min_sn", "5")]);
    let d = detected(&sn5);
    assert!(
        !d.contains(&"Matrix.mzML|blip1".into()) && !d.contains(&"Matrix.mzML|blip2".into()),
        "{d:?}"
    );
    assert!(
        d.contains(&"Blank_01.mzML|blip5".into()),
        "S/N alone still lets taller blips through: {d:?}"
    );
    let (half, log) = go(&[("min_half", "3")]);
    assert_eq!(detected(&half), ["Matrix.mzML|real"], "{half}");
    // every rejected blip keeps its numbers (state forced, never hidden) and says which check failed with the evidence
    let r = rows(&half);
    assert_eq!(r["Matrix.mzML|blip1"].0, "forced");
    assert!(r["Matrix.mzML|blip1"].1.contains("bad_shape"));
    assert!(log.contains("\"name\":\"half_pts\",\"value\":"), "{log}");
    assert!(
        log.contains("\"limit\":3,\"op\":\">=\",\"pass\":false"),
        "{log}"
    );
    // the real peak of the matrix passed everything: it is not in `decisions`
    assert!(
        !log.contains("\"file\":\"Matrix.mzML\",\"id\":\"real\",\"state\""),
        "{log}"
    );
}

#[test]
fn the_width_and_symmetry_gates_judge_the_real_peak_with_their_numbers() {
    // the real peak is about 14 s wide and symmetric
    let (ok, _) = go(&[("max_fwhm", "30"), ("max_asym", "1.5")]);
    assert_eq!(rows(&ok)["Matrix.mzML|real"].0, "detected", "{ok}");
    let (narrow, log) = go(&[("max_fwhm", "5")]);
    let r = rows(&narrow);
    assert_eq!(
        (
            r["Matrix.mzML|real"].0.as_str(),
            r["Matrix.mzML|real"].1.as_str()
        ),
        ("forced", "bad_shape")
    );
    assert!(log.contains("\"name\":\"fwhm_s\""), "{log}");
    // a limit below 1 is not an asymmetry: refused, never clamped
    let files = [("A.mzML", ions(true), 7)];
    let mut open = |k: usize| Ok(index_of(&files[k].1, files[k].2));
    let mut bad = |k: &str, v: &str| {
        let p = vec![(k.to_string(), Value::Str(v.to_string()))];
        run(&mut open, &["A.mzML".to_string()], targets().as_bytes(), &p)
            .unwrap_err()
            .key
    };
    assert_eq!(bad("max_asym", "0.5"), "err.quant.targets.param");
    assert_eq!(bad("min_half", "-1"), "err.quant.targets.param");
    assert_eq!(bad("max_fwhm", "-2"), "err.quant.targets.param");
}

#[test]
fn a_blip_in_the_blank_flags_the_matrix_only_while_the_blank_detects_it() {
    let p = [("blank_pattern", "blank*")];
    let (old, _) = go(&p);
    assert!(
        old.lines()
            .any(|l| l.starts_with("Matrix.mzML,blip") && l.contains("in_blank")),
        "{old}"
    );
    let (new, _) = go(&[("blank_pattern", "blank*"), ("min_half", "3")]);
    assert!(!new.contains("in_blank"), "{new}");
}
