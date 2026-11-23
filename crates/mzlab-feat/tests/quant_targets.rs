//! `quant.targets` on synthetic indexes: a clean peak is detected, a noise-only target and a small peak are reported but flagged, an apex on the
//! window edge is flagged, `offset = auto` finds a +0.3 Da shift on the standard row, a peak seen in a blank flags the samples, and every area is
//! the one `trace xic | peak auto-lr --from --to` gives on the same trace.

use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::peakq_lrauto::{lr_auto_integrate, LrAutoParams, Split};
use mzlab_core::routes::Routes;
use mzlab_core::upload::prepare;
use mzlab_feat::quant_targets::{plan, run, HEADER};

mod common_quant;
use common_quant::{mzml, sample, sparse, Ion, TARGETS};

/// The index of the synthetic file.
fn index_of(ions: &[Ion], seed: u64) -> Index<Vec<u8>> {
    index_bytes(mzml(ions, seed))
}

fn index_bytes(mzml: Vec<u8>) -> Index<Vec<u8>> {
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0;
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}

fn params(list: &[(&str, &str)]) -> Vec<(String, Value)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), Value::Str(v.to_string())))
        .collect()
}

/// The rows of the long table as cells, keyed by `file|id`.
fn rows(csv: &str) -> Vec<(String, Vec<String>)> {
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some(HEADER));
    lines
        .map(|l| {
            let c: Vec<String> = l.split(',').map(str::to_string).collect();
            (format!("{}|{}", c[0], c[1]), c)
        })
        .collect()
}

fn get<'a>(r: &'a [(String, Vec<String>)], key: &str) -> &'a [String] {
    &r.iter()
        .find(|x| x.0 == key)
        .unwrap_or_else(|| panic!("no row {key}"))
        .1
}

fn run_on(
    files: &[(&str, Vec<Ion>, u64)],
    p: &[(&str, &str)],
) -> mzlab_feat::quant_targets::Outcome {
    let names: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
    let mut open = |k: usize| Ok(index_of(&files[k].1, files[k].2));
    run(&mut open, &names, TARGETS.as_bytes(), &params(p)).unwrap()
}

#[test]
fn flags_tell_detected_small_edge_and_empty_targets_apart() {
    let o = run_on(&[("s1.mzML", sample(), 7)], &[("da", "0.2")]);
    let r = rows(&o.csv);
    assert_eq!(r.len(), 6, "{}", o.csv);
    let clean = get(&r, "s1.mzML|clean");
    assert_eq!(
        (clean[6].as_str(), clean[7].as_str()),
        ("ok", "detected"),
        "{:?}",
        clean
    );
    assert!(clean[3].parse::<f64>().unwrap() > 100.0, "S/N {:?}", clean);
    assert!((clean[5].parse::<f64>().unwrap() - 5.0).abs() < 0.06);
    // a small peak keeps its numbers, flagged
    let small = get(&r, "s1.mzML|small");
    assert!(
        small[6].contains("below_threshold")
            && !small[2].is_empty()
            && small[7] == "force_integrated",
        "{:?}",
        small
    );
    let edge = get(&r, "s1.mzML|edge");
    assert!(edge[6].split(';').any(|f| f == "edge"), "{:?}", edge);
    let empty = get(&r, "s1.mzML|empty");
    assert_ne!(empty[6], "ok", "{:?}", empty);
    assert_ne!(empty[7], "detected", "{:?}", empty);
    // every row has one of the three states
    for (_, c) in &r {
        assert!(
            matches!(c[7].as_str(), "detected" | "force_integrated" | "absent"),
            "{c:?}"
        );
        assert_eq!(c[7] == "absent", c[2].is_empty(), "{c:?}");
    }
    // without an offset the shifted ion is outside its +-0.2 Da window
    assert_ne!(get(&r, "s1.mzML|shifted")[6], "ok");
    assert!(o.log.starts_with("{\"schema\":\"quant-targets-log/1\""));
    assert!(
        o.log.contains("\"key\":\"smooth_half\",\"value\":\"auto\""),
        "{}",
        o.log
    );
}

#[test]
fn offset_auto_is_measured_on_the_standard_row() {
    let o = run_on(
        &[("s1.mzML", sample(), 7)],
        &[("da", "0.2"), ("offset", "auto")],
    );
    let r = rows(&o.csv);
    let shifted = get(&r, "s1.mzML|shifted");
    assert_eq!(shifted[6], "ok", "{:?}\n{}", shifted, o.log);
    let off: f64 = shifted[14].parse().unwrap();
    assert!((off - 0.3).abs() < 0.02, "offset {off}");
    assert!(o.log.contains("\"reference\":\"std\""), "{}", o.log);
    assert!(
        o.log
            .contains("\"key\":\"offset_reference.+\",\"value\":\"std\""),
        "{}",
        o.log
    );
    // a fixed number does the same
    let f = run_on(
        &[("s1.mzML", sample(), 7)],
        &[("da", "0.2"), ("offset", "0.3")],
    );
    assert_eq!(get(&rows(&f.csv), "s1.mzML|shifted")[6], "ok");
}

#[test]
fn a_peak_in_a_blank_flags_the_samples() {
    let blank = vec![Ion(200.0, 5.0, 5000.0, 0.1), Ion(300.0, 0.0, 0.0, 0.1)];
    let files = [("s1.mzML", sample(), 7), ("Blank_01.mzML", blank, 9)];
    let o = run_on(&files, &[("da", "0.2"), ("blank_pattern", "blank*")]);
    let r = rows(&o.csv);
    assert_eq!(get(&r, "s1.mzML|clean")[6], "in_blank");
    assert_eq!(get(&r, "Blank_01.mzML|clean")[6], "ok");
    assert!(!get(&r, "s1.mzML|edge")[6].contains("in_blank"));
    assert!(
        o.log.contains("{\"file\":\"Blank_01.mzML\",\"blank\":true"),
        "{}",
        o.log
    );
    // a pattern no file matches is a warning, not an error
    let none = run_on(&[("s1.mzML", sample(), 7)], &[("blank_pattern", "qc*")]);
    assert_eq!(
        none.warnings,
        vec!["warn.quant.targets.noBlank".to_string()]
    );
}

#[test]
fn areas_are_those_of_trace_xic_into_peak_auto_lr() {
    let o = run_on(&[("s1.mzML", sample(), 7)], &[("da", "0.2")]);
    let r = rows(&o.csv);
    let mut routes = Routes::new(index_of(&sample(), 7));
    let (rt, ys) = routes.xic_sweep(&[200.0, 240.0], 0.2, 1).unwrap();
    for (k, id) in ["clean", "edge"].iter().enumerate() {
        let p = LrAutoParams {
            half: 0,
            valley: 0.1,
            snr: 0.0,
            min_pts: 2,
            despike: true,
            split: Split::Drop,
        };
        let res = lr_auto_integrate(&rt, &ys[k], 4.0, 6.0, p);
        let best = res
            .peaks
            .iter()
            .max_by(|a, b| a.height.total_cmp(&b.height))
            .unwrap();
        let row = get(&r, &format!("s1.mzML|{id}"));
        assert_eq!(
            row[2],
            format!("{}", mzlab_core::json::round(best.area * 60.0, 1))
        );
        assert_eq!(row[5], format!("{}", rt[best.apex]));
    }
}

#[test]
fn the_same_bytes_every_run_and_a_plan_reads_no_data() {
    let a = run_on(&[("s1.mzML", sample(), 7)], &[("offset", "auto")]);
    let b = run_on(&[("s1.mzML", sample(), 7)], &[("offset", "auto")]);
    assert_eq!(a, b);
    let names = vec!["a.mzML".to_string(), "blank.mzML".to_string()];
    let pl = plan(
        &names,
        TARGETS.as_bytes(),
        &params(&[("blank_pattern", "blank*")]),
    )
    .unwrap();
    assert!(pl.starts_with("{\"schema\":\"quant-targets-plan/1\""));
    assert!(pl.contains("{\"file\":\"blank.mzML\",\"blank\":true}"));
    assert!(pl.ends_with("\"rows\":12}"));
}

#[test]
fn wrong_tables_and_parameters_are_refused_with_their_key() {
    let mut open = |_k: usize| Ok(index_of(&sample(), 7));
    let names = vec!["a.mzML".to_string()];
    let e = run(&mut open, &names, b"id,mz,rt_min,rt_max\nx,abc,1,2\n", &[]).unwrap_err();
    assert_eq!(e.key, "err.quant.targets.table");
    let e = run(
        &mut open,
        &names,
        TARGETS.as_bytes(),
        &params(&[("da", "-1")]),
    )
    .unwrap_err();
    assert_eq!(e.key, "err.quant.targets.param");
    let e = run(&mut open, &[], TARGETS.as_bytes(), &[]).unwrap_err();
    assert_eq!(e.key, "err.quant.targets.noFiles");
}

#[test]
fn sparse_ion_counts_give_a_peak_with_its_s_n_and_stray_counts_none() {
    // whole counts, most scans empty: a weak ion (apex 12 counts) at 5 min, lone counts of 1 at 300, nothing at 220
    let bytes = sparse(&[Ion(200.0, 5.0, 12.0, 0.1)], 300.0, 3);
    let names = vec!["weak.mzML".to_string()];
    let mut open = |_k: usize| Ok(index_bytes(bytes.clone()));
    let o = run(
        &mut open,
        &names,
        TARGETS.as_bytes(),
        &params(&[("da", "0.2")]),
    )
    .unwrap();
    let r = rows(&o.csv);
    let clean = get(&r, "weak.mzML|clean");
    assert_eq!(clean[7], "detected", "{:?}\n{}", clean, o.csv);
    let sn: f64 = clean[4].parse().unwrap();
    assert!(sn.is_finite() && sn >= 3.0, "an S/N per peak: {sn}");
    assert!(
        (clean[5].parse::<f64>().unwrap() - 5.0).abs() < 0.11,
        "{:?}",
        clean
    );
    // a few stray counts are never a detected peak, and an m/z with no count at all is absent
    assert_ne!(get(&r, "weak.mzML|empty")[7], "detected");
    assert_eq!(get(&r, "weak.mzML|small")[7], "absent");
}
