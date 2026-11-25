//! `quant.targets` with the internal RT reference (`rt_ref`) on synthetic batches whose retention times drift: every window follows the reference,
//! a weak or missing reference moves nothing, and the evidence is in the log.

use mzlab_core::idxread::Index;
use mzlab_core::json::Value;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::quant_targets::{run, HEADER};

mod common_quant;
use common_quant::{mzml, sample, Ion};

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

/// The cells of one CSV line (no quoted cell in these tables).
fn cells(l: &str) -> Vec<String> {
    l.split(',').map(String::from).collect()
}

/// The long table as cells, header skipped.
fn rows(csv: &str) -> Vec<Vec<String>> {
    csv.lines().skip(1).map(cells).collect()
}

fn get<'a>(r: &'a [Vec<String>], key: &str) -> &'a [String] {
    r.iter()
        .find(|x| format!("{}|{}", x[0], x[1]) == key)
        .unwrap_or_else(|| panic!("no row {key}"))
}

/// The synthetic sample with every apex moved by `dt` minutes: another batch whose retention times drifted.
fn drifted(dt: f64) -> Vec<Ion> {
    sample()
        .iter()
        .map(|o| Ion(o.0, o.1 + dt, o.2, o.3))
        .collect()
}

const NARROW: &str = "id,mz,rt_min,rt_max,polarity,role\n\
clean,200.0,4.8,5.2,+,\n\
std,400.0,1.5,2.5,+,standard\n";

#[test]
fn rt_ref_moves_every_window_by_the_shift_of_the_internal_reference_and_logs_it() {
    let files = [
        ("a.mzML", mzml(&drifted(0.0), 7)),
        ("b.mzML", mzml(&drifted(0.3), 8)),
        ("c.mzML", mzml(&drifted(0.16), 9)),
    ];
    let names: Vec<String> = files.iter().map(|f| f.0.to_string()).collect();
    let go = |p: &[(&str, &str)]| {
        let mut open = |k: usize| Ok(index_bytes(files[k].1.clone()));
        run(&mut open, &names, NARROW.as_bytes(), &params(p)).unwrap()
    };
    // off (default): the header is the old one and a drifted batch loses the peak
    let off = go(&[]);
    assert!(off.csv.starts_with(&format!("{HEADER}\n")), "{}", off.csv);
    let r = rows(&off.csv);
    assert_ne!(
        get(&r, "b.mzML|clean")[6],
        "ok",
        "{:?}",
        get(&r, "b.mzML|clean")
    );
    assert!(!off.log.contains("rt_ref"), "{}", off.log);
    // on: one more column, windows follow the reference, areas agree with the undrifted batch
    let on = go(&[("rt_ref", "auto")]);
    assert!(
        on.csv.starts_with(&format!("{HEADER},rt_shift_min\n")),
        "{}",
        on.csv
    );
    let cell = |file: &str, id: &str, col: usize| -> String {
        let line = on
            .csv
            .lines()
            .find(|l| l.starts_with(&format!("{file},{id},")))
            .unwrap();
        cells(line)[col].clone()
    };
    for (file, want) in [("a.mzML", 0.0), ("b.mzML", 0.3), ("c.mzML", 0.16)] {
        let s: f64 = cell(file, "clean", 15).parse().unwrap();
        assert!((s - want).abs() < 0.03, "{file}: shift {s}");
        assert_eq!(cell(file, "clean", 15), cell(file, "std", 15));
        assert_eq!(cell(file, "clean", 7), "detected", "{file}\n{}", on.csv);
    }
    let area = |f: &str| cell(f, "clean", 2).parse::<f64>().unwrap();
    for f in ["b.mzML", "c.mzML"] {
        assert!(
            (area(f) / area("a.mzML") - 1.0).abs() < 0.08,
            "{f}: {} vs {}",
            area(f),
            area("a.mzML")
        );
    }
    // the evidence: reference, expected and found RT, S/N, search window, shift, status; the choices say which row and which frame
    for needle in [
        "\"rt_ref\":{\"reference\":\"std\",\"expected_rt\":2",
        "\"status\":\"ok\"",
        "\"key\":\"rt_reference\",\"value\":\"std\"",
        "\"key\":\"rt_expected\",\"value\":\"2\"",
        "\"rt_ref\":\"auto\",\"rt_span\":0.6",
    ] {
        assert!(on.log.contains(needle), "{needle}\n{}", on.log);
    }
    assert!(on.json().contains("\"rt_shift_min\":"), "{}", on.json());
    // a named row and a given expected RT give the same shift in the same frame
    let named = go(&[("rt_ref", "std@2.0")]);
    assert_eq!(
        named.csv.replace("std@2.0", "auto"),
        on.csv.replace("std@2.0", "auto")
    );
    assert!(
        named.log.contains("\"reason\":\"given in rt_ref\""),
        "{}",
        named.log
    );
}

#[test]
fn rt_ref_never_moves_windows_on_noise_or_without_a_reference() {
    let mut weak = sample();
    weak[4] = Ion(400.3, 2.3, 0.0, 0.1);
    let files = [("w.mzML", mzml(&weak, 7))];
    let names = vec!["w.mzML".to_string()];
    let go = |t: &str, p: &[(&str, &str)]| {
        let mut open = |k: usize| Ok(index_bytes(files[k].1.clone()));
        run(&mut open, &names, t.as_bytes(), &params(p))
    };
    let o = go(NARROW, &[("rt_ref", "auto")]).unwrap();
    assert!(
        o.log.contains("\"status\":\"weak\"") || o.log.contains("\"status\":\"not_found\""),
        "{}",
        o.log
    );
    assert!(
        o.warnings
            .contains(&"warn.quant.targets.rtRefNotFound".to_string()),
        "{:?}",
        o.warnings
    );
    assert!(o.log.contains("\"shift_min\":0"), "{}", o.log);
    // no standard/parent row: a warning and a choice, windows stay, the column is there
    let no_ref = "id,mz,rt_min,rt_max,polarity,role\nclean,200.0,4.8,5.2,+,\n";
    let o = go(no_ref, &[("rt_ref", "auto")]).unwrap();
    assert!(
        o.warnings
            .contains(&"warn.quant.targets.noRtReference".to_string()),
        "{:?}",
        o.warnings
    );
    assert!(
        o.csv.starts_with(&format!("{HEADER},rt_shift_min\n")) && o.csv.trim_end().ends_with(",0"),
        "{}",
        o.csv
    );
    // an id that is not in the table, or a bad value, is refused like any parameter
    for bad in ["nope", "auto@x", "@2"] {
        let e = go(NARROW, &[("rt_ref", bad)]).unwrap_err();
        assert_eq!(e.key, "err.quant.targets.param", "{bad}");
    }
    let e = go(NARROW, &[("rt_ref", "auto"), ("rt_span", "9")]).unwrap_err();
    assert_eq!(e.key, "err.quant.targets.param");
}
