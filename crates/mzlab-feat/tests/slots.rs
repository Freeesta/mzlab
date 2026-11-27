//! `feature-slots/1`: the fixed slots of one row are filled from what the table holds, empty with the step that fills them, or na; never a made-up value.

use mzlab_core::json::{parse, Value};
use mzlab_feat::samples::{propose, Sample};
use mzlab_feat::slots::{slots, SLOTS};
use mzlab_feat::table::{
    Aligned, RowIdent, RowQ, CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED,
};
const STATES: [u8; 3] = [CELL_DETECTED, CELL_FILLED, CELL_TARGETED];

fn table() -> Aligned {
    let mut t = Aligned::with_capacity(3, 5);
    for r in 0..5usize {
        let (mz, rt) = (200.0 + 12.5 * r as f64, 3.0 + r as f64);
        t.mz.push(mz);
        t.rt.push(rt);
        for k in 0..3usize {
            let absent = (r + k) % 4 == 3;
            t.area.push(if absent {
                0.0
            } else {
                1e5 * (1 + r + 2 * k) as f64
            });
            t.height.push(1e4 * (1 + r) as f32);
            t.apex_rt.push((rt + 0.01 * k as f64) as f32);
            t.width.push(0.1 + 0.01 * k as f32);
            t.state.push(if absent {
                CELL_ABSENT
            } else {
                STATES[(r + k) % 3]
            });
        }
        let q = if r == 4 {
            RowQ {
                rt_lo: (rt - 0.1) as f32,
                rt_hi: (rt + 0.1) as f32,
                ..RowQ::NONE
            }
        } else {
            RowQ {
                rt_lo: (rt - 0.1) as f32,
                rt_hi: (rt + 0.2) as f32,
                charge: 1,
                q_shape: 0.9,
                q_snr: 12.5,
                q_fwhm: 6.0,
                qreason: if r == 2 { 5 } else { 0 },
                iso_m1: 0.25,
                ms2_file: (r % 3) as u32,
                ms2_purity: 0.75,
            }
        };
        t.meta.push(&q);
    }
    t.ident = RowIdent {
        neutral: (0..5).map(|r| 199.0 + 12.5 * r as f64).collect(),
        ion: vec![1, 1, 2, 0, 1],
        src: vec![1, 3, 5, 0, 1],
    };
    t
}

fn samples() -> Vec<Sample> {
    let f: Vec<(String, String)> = ["blank.mzML", "t10.mzML", "t20.mzML"]
        .iter()
        .map(|n| (n.to_string(), "00".repeat(32)))
        .collect();
    propose(&f)
}

fn slot<'a>(v: &'a Value, name: &str) -> &'a Value {
    v.get("slots")
        .unwrap()
        .arr()
        .iter()
        .find(|s| s.get("slot").unwrap().str() == Some(name))
        .unwrap()
}

fn state(v: &Value, name: &str) -> String {
    slot(v, name)
        .get("state")
        .unwrap()
        .str()
        .unwrap()
        .to_string()
}

fn of(t: &Aligned, id: u32) -> Value {
    parse(slots(t, &samples(), id, "t.mzfeat").unwrap().as_bytes()).unwrap()
}

#[test]
fn the_slots_come_in_a_fixed_order_and_each_says_what_fills_it_or_where_it_came_from() {
    let t = table();
    for id in 1..=5 {
        let v = of(&t, id);
        assert_eq!(v.get("schema").unwrap().str(), Some("feature-slots/1"));
        let names: Vec<&str> = v
            .get("slots")
            .unwrap()
            .arr()
            .iter()
            .map(|s| s.get("slot").unwrap().str().unwrap())
            .collect();
        assert_eq!(names, SLOTS.iter().map(|s| s.0).collect::<Vec<_>>());
        for s in v.get("slots").unwrap().arr() {
            let filled = s.get("state").unwrap().str() == Some("filled");
            assert_eq!(
                !s.get("evidence").unwrap().is_null(),
                filled,
                "evidence only when filled: {s:?}"
            );
            assert_eq!(!s.get("source").unwrap().is_null(), filled);
            assert_eq!(
                !s.get("fill").unwrap().is_null(),
                !filled,
                "a slot without evidence names what fills it"
            );
            if filled {
                let src = s.get("source").unwrap();
                assert_eq!(src.get("output").unwrap().str(), Some("t.mzfeat"));
                for f in src.get("files").unwrap().arr() {
                    assert_eq!(
                        f.get("sha256").unwrap().str(),
                        Some("00".repeat(32).as_str())
                    );
                }
            }
        }
    }
}

#[test]
fn a_row_with_peaks_in_timed_files_fills_xic_and_kinetics_with_the_files_behind_them() {
    let v = of(&table(), 1);
    assert_eq!(state(&v, "xic"), "filled");
    let xic = slot(&v, "xic");
    assert_eq!(
        xic.get("evidence").unwrap().get("files").unwrap().f64(),
        Some(3.0)
    );
    assert_eq!(
        xic.get("source").unwrap().get("files").unwrap().arr().len(),
        3
    );
    assert_eq!(state(&v, "kinetics"), "filled");
    let kin = slot(&v, "kinetics").get("evidence").unwrap();
    assert_eq!(
        kin.get("times_min")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.f64().unwrap())
            .collect::<Vec<_>>(),
        vec![10.0, 20.0]
    );
}

#[test]
fn kinetics_is_empty_with_one_timed_point_and_na_without_a_time_axis() {
    // Row 2 is absent in the last file: only the t10 file has a time and an area.
    assert_eq!(state(&of(&table(), 2), "kinetics"), "empty");
    let t = table();
    let mut s = samples();
    for x in &mut s {
        x.time_min = None;
    }
    let v = parse(slots(&t, &s, 1, "").unwrap().as_bytes()).unwrap();
    let k = slot(&v, "kinetics");
    assert_eq!(k.get("state").unwrap().str(), Some("na"));
    assert_eq!(k.get("reason").unwrap().str(), Some("no_time_axis"));
    assert!(v.get("output").unwrap().is_null());
}

#[test]
fn ms2_isotopes_and_structure_follow_the_summaries_and_the_identity_of_the_row() {
    let t = table();
    let v = of(&t, 2);
    assert_eq!(state(&v, "ms2"), "filled");
    assert_eq!(
        slot(&v, "ms2")
            .get("source")
            .unwrap()
            .get("files")
            .unwrap()
            .arr()
            .len(),
        1
    );
    assert_eq!(state(&v, "isotopes"), "filled");
    assert_eq!(
        state(&v, "structure"),
        "filled",
        "source 3 carries a hypothesis"
    );
    // Row 1: detected only, no hypothesis. Row 5: no summaries at all.
    assert_eq!(state(&of(&t, 1), "structure"), "empty");
    let last = of(&t, 5);
    assert_eq!(state(&last, "ms2"), "empty");
    assert_eq!(
        slot(&last, "ms2")
            .get("fill")
            .unwrap()
            .get("step")
            .unwrap()
            .str(),
        Some("features.ms2_link")
    );
    assert_eq!(state(&last, "isotopes"), "filled", "its adduct is known");
}

#[test]
fn uv_is_empty_and_sim_is_na_for_a_table_of_ions() {
    let v = of(&table(), 1);
    let uv = slot(&v, "uv");
    assert_eq!(uv.get("state").unwrap().str(), Some("empty"));
    assert_eq!(
        uv.get("fill").unwrap().get("command").unwrap().str(),
        Some("mzlab-cli dad spectrum")
    );
    assert_eq!(
        slot(&v, "sim").get("reason").unwrap().str(),
        Some("not_a_transition_table")
    );
}

#[test]
fn a_row_of_a_transition_table_fills_sim_and_an_all_na_row_is_na() {
    let mut t = table();
    t.q3 = (0..5).map(|r| 100.0 + r as f64).collect();
    let v = of(&t, 2);
    assert_eq!(state(&v, "sim"), "filled");
    assert_eq!(
        slot(&v, "sim")
            .get("evidence")
            .unwrap()
            .get("q3")
            .unwrap()
            .f64(),
        Some(101.0)
    );
    let n = t.n_files;
    for k in 0..n {
        let c = t.cell(0, k);
        t.state[c] = mzlab_feat::table::CELL_NA;
    }
    assert_eq!(state(&of(&t, 1), "xic"), "na");
}

#[test]
fn an_unknown_id_is_row_not_found_and_the_bytes_are_stable() {
    let t = table();
    let e = slots(&t, &samples(), 99, "").unwrap_err();
    assert_eq!(e.key, "error.rowNotFound");
    assert_eq!(
        slots(&t, &samples(), 3, "t.mzfeat").unwrap(),
        slots(&t, &samples(), 3, "t.mzfeat").unwrap()
    );
}
