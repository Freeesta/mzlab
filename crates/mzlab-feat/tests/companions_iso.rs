//! `features.companions@1` with `iso_group` on a synthetic unit-mass table (invented masses): compound A has M, M+1 at 11 % and M+2 at 4 %; B has a +1
//! channel at 65 % of M (spill-over); C has a +1 ion at 11 % with an unrelated time profile; D has a +1 ion at 90 % of M. Grouping needs the correlation
//! floor and the ratio bounds, the spill-over flag carries its ratio and r, every choice is in the text, and the default text has no `isotopes` section.

use mzlab_core::json::{parse, Value};
use mzlab_feat::companions::companions_text;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const LEVELS: [[f64; 5]; 3] = [
    [1.0, 2.0, 3.0, 4.0, 5.0],
    [5.0, 1.0, 4.0, 2.0, 3.0],
    [2.0, 5.0, 1.0, 3.0, 4.0],
];

/// (m/z, RT, level set, share): rows 0-2 are A's M, M+1, M+2; 3-4 are B; 5-6 are C; 7-8 are D.
const ROWS: [(f64, f64, usize, f64); 9] = [
    (301.0, 3.00, 0, 1.0),
    (302.0, 3.00, 0, 0.11),
    (303.0, 3.00, 0, 0.04),
    (401.0, 4.00, 1, 1.0),
    (402.0, 4.00, 1, 0.65),
    (501.0, 5.00, 0, 1.0),
    (502.0, 5.00, 2, 0.11),
    (601.0, 6.00, 1, 1.0),
    (602.0, 6.00, 1, 0.90),
];

fn table(files: usize) -> Vec<u8> {
    let mut t = Aligned::with_capacity(files, ROWS.len());
    for (mz, rt, c, share) in ROWS {
        t.mz.push(mz);
        t.rt.push(rt);
        for f in 0..files {
            let h = 1e5 * LEVELS[c][f % 5] * share * (1.0 + 0.001 * (f / 5) as f64);
            t.area.push(h * 6.0);
            t.height.push(h as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.08);
            t.state.push(CELL_DETECTED);
        }
    }
    let origin = Origin {
        inputs: (0..files)
            .map(|k| (format!("c{k}.mzML"), format!("{k:02x}").repeat(32)))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

fn set(k: &str, v: Value) -> (String, Value) {
    (k.to_string(), v)
}

fn on() -> Vec<(String, Value)> {
    vec![set("iso_group", Value::Bool(true))]
}

fn run(params: &[(String, Value)]) -> Value {
    parse(companions_text(params, &table(5)).unwrap().text.as_bytes()).unwrap()
}

fn ints(v: &Value, k: &str) -> Vec<Option<i64>> {
    v.get(k)
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().map(|n| n as i64))
        .collect()
}

fn verdict_of(iso: &Value, from: i64, to: i64) -> String {
    iso.get("pairs")
        .unwrap()
        .arr()
        .iter()
        .find(|p| {
            p.get("from").unwrap().f64() == Some(from as f64)
                && p.get("to").unwrap().f64() == Some(to as f64)
        })
        .and_then(|p| p.get("verdict").and_then(Value::str).map(String::from))
        .unwrap_or_default()
}

#[test]
fn off_by_default_the_text_has_no_isotopes_section() {
    let v = run(&[]);
    assert!(v.get("isotopes").is_none());
}

#[test]
fn m_m1_m2_are_one_group_and_the_spillover_is_flagged_not_grouped() {
    let v = run(&on());
    let iso = v.get("isotopes").unwrap();
    // ids are 1-based rows: A = 1, 2, 3; B = 4, 5; C = 6, 7; D = 8, 9
    assert_eq!(
        ints(iso, "iso_group"),
        [
            Some(1),
            Some(1),
            Some(1),
            None,
            None,
            None,
            None,
            None,
            None
        ]
    );
    assert_eq!(
        ints(iso, "iso_step"),
        [
            Some(0),
            Some(1),
            Some(2),
            None,
            None,
            None,
            None,
            None,
            None
        ]
    );
    assert_eq!(verdict_of(iso, 1, 2), "isotope");
    assert_eq!(verdict_of(iso, 1, 3), "isotope");
    assert_eq!(verdict_of(iso, 4, 5), "spillover");
    assert_eq!(verdict_of(iso, 6, 7), "rejected_corr");
    assert_eq!(verdict_of(iso, 8, 9), "rejected_ratio");
    // the flag names the M it spills from and its pair carries the ratio and r
    assert_eq!(
        ints(iso, "spill_of"),
        [None, None, None, None, Some(4), None, None, None, None]
    );
    let pair = iso
        .get("pairs")
        .unwrap()
        .arr()
        .iter()
        .find(|p| p.get("verdict").and_then(Value::str) == Some("spillover"))
        .unwrap();
    let ratio = pair
        .get("ratio")
        .unwrap()
        .get("median")
        .unwrap()
        .f64()
        .unwrap();
    assert!((ratio - 0.65).abs() < 1e-3, "{ratio}");
    assert!(pair.get("r").unwrap().f64().unwrap() > 0.999);
    let c = iso.get("counts").unwrap();
    let n = |k: &str| c.get(k).unwrap().f64().unwrap() as i64;
    assert_eq!(
        (
            n("pairs"),
            n("isotope"),
            n("spillover"),
            n("rejected_corr"),
            n("rejected_ratio"),
            n("groups"),
            n("grouped_rows")
        ),
        // A has three isotope pairs (M-M+1, M-M+2 and M+1-M+2) that make one group of three rows
        (6, 3, 1, 1, 1, 1, 3)
    );
}

#[test]
fn the_rules_are_parameters_and_the_text_echoes_them() {
    let v = run(&[
        set("iso_group", Value::Bool(true)),
        set("spill_min", Value::Num(0.7)),
        set("iso_ratio_max", Value::Num(0.7)),
        set("iso_min_corr", Value::Num(-1.0)),
    ]);
    let iso = v.get("isotopes").unwrap();
    // 0.65 is now inside the isotope bounds and under the spill band; the unrelated profile passes a floor of -1
    assert_eq!(verdict_of(iso, 4, 5), "isotope");
    assert_eq!(verdict_of(iso, 6, 7), "isotope");
    let p = iso.get("params").unwrap();
    assert_eq!(p.get("spill_min").unwrap().f64(), Some(0.7));
    assert_eq!(p.get("iso_min_corr").unwrap().f64(), Some(-1.0));
    assert_eq!(p.get("iso_ratio_max").unwrap().f64(), Some(0.7));
}

#[test]
fn the_text_is_the_same_bytes_and_the_edges_do_not_change() {
    let a = companions_text(&on(), &table(5)).unwrap().text;
    assert_eq!(a, companions_text(&on(), &table(5)).unwrap().text);
    let plain = parse(companions_text(&[], &table(5)).unwrap().text.as_bytes()).unwrap();
    let with = parse(a.as_bytes()).unwrap();
    for k in ["edges", "counts", "companion_of", "weak"] {
        assert_eq!(plain.get(k), with.get(k), "{k}");
    }
}
