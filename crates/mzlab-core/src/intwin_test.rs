//! `move_end` on a trace of ten points 0.1 min apart: a drag snaps to the nearest point, an end never reaches the other one,
//! a neighbour's border stops it (touching is fine), the keyboard moves whole points, and the JSON carries the time and the index.

use super::{move_end, move_end_json};

fn rt() -> Vec<f64> {
    (0..10).map(|i| i as f64 / 10.0).collect()
}

#[test]
fn a_drag_snaps_to_the_nearest_point() {
    let rt = rt();
    assert_eq!(move_end(&rt, 0.2, 0.7, Some(0.33), 0, None, None), Some(3));
    assert_eq!(move_end(&rt, 0.7, 0.2, Some(0.86), 0, None, None), Some(9));
}

#[test]
fn an_end_never_reaches_the_other_one() {
    let rt = rt();
    assert_eq!(move_end(&rt, 0.2, 0.7, Some(0.9), 0, None, None), Some(6));
    assert_eq!(move_end(&rt, 0.7, 0.2, Some(-1.0), 0, None, None), Some(3));
    assert_eq!(
        move_end(&rt, 0.0, 0.0, Some(0.5), 0, None, None),
        None,
        "no point before the first one"
    );
}

#[test]
fn the_trace_and_a_neighbour_stop_the_end() {
    let rt = rt();
    assert_eq!(move_end(&rt, 0.2, 0.7, Some(-3.0), 0, None, None), Some(0));
    assert_eq!(move_end(&rt, 0.7, 0.2, Some(3.0), 0, None, None), Some(9));
    assert_eq!(
        move_end(&rt, 0.4, 0.7, Some(0.0), 0, Some(0.2), None),
        Some(2),
        "touching the neighbour"
    );
    assert_eq!(
        move_end(&rt, 0.4, 0.7, Some(0.0), 0, Some(0.25), None),
        Some(3)
    );
    assert_eq!(
        move_end(&rt, 0.4, 0.2, Some(0.9), 0, None, Some(0.6)),
        Some(6)
    );
}

#[test]
fn the_keyboard_moves_whole_points() {
    let rt = rt();
    assert_eq!(move_end(&rt, 0.2, 0.7, None, 1, None, None), Some(3));
    assert_eq!(move_end(&rt, 0.21, 0.7, None, -5, None, None), Some(0));
    assert_eq!(move_end(&rt, 0.7, 0.2, None, 5, None, None), Some(9));
    assert_eq!(move_end(&rt, 0.7, 0.2, None, -5, None, None), Some(3));
}

#[test]
fn the_json_carries_the_time_and_the_index() {
    let rt = rt();
    assert_eq!(
        move_end_json(&rt, 0.2, 0.7, Some(0.33), 0, None, None).unwrap(),
        "{\"t\":0.3,\"i\":3}"
    );
    assert_eq!(
        move_end_json(&rt[..1], 0.0, 0.0, Some(0.0), 0, None, None),
        None
    );
    assert_eq!(
        move_end_json(&rt, 0.2, 0.7, Some(f64::NAN), 0, None, None),
        None
    );
}
