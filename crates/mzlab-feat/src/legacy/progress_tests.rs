use super::*;

fn frac(kind: &str, phase: &str, done: u32, total: u32) -> f64 {
    let j = json(kind, phase, done, total);
    let k = j.find("\"frac\":").unwrap() + 7;
    j[k..j.len() - 1].parse().unwrap()
}

#[test]
fn weights_add_up_to_one() {
    assert!((LR.iter().map(|p| p.weight).sum::<f64>() - 1.0).abs() < 1e-9);
    assert!((HR.iter().map(|p| p.weight).sum::<f64>() - 1.0).abs() < 1e-9);
}

#[test]
fn never_goes_down_through_a_run() {
    for (kind, list) in [("lr", &LR[..]), ("hr", &HR[..])] {
        let mut last = -1.0;
        for p in list {
            for d in 0..=5 {
                let f = frac(kind, p.key, d, 5);
                assert!(f >= last, "{kind} {} {d}: {f} < {last}", p.key);
                last = f;
            }
        }
        assert!((last - 1.0).abs() < 1e-3, "{kind} ends at {last}");
    }
}

#[test]
fn file_by_file_phase_names_the_file() {
    let j = json("lr", "calibration", 3, 40);
    assert!(
        j.contains("\"label\":\"Detecting peaks: file 3 of 40\""),
        "{j}"
    );
    assert!(j.contains("\"done\":3,\"total\":40"), "{j}");
    // a phase with one job shows no «file 1 of 1», and a count above the total is held at the total
    assert!(json("lr", "xic", 0, 1).contains("\"label\":\"Aligning peaks and extracting XICs\""));
    assert!((frac("lr", "calibration", 9, 4) - 0.08).abs() < 1e-9);
}

#[test]
fn unknown_names_give_null() {
    assert_eq!(json("lr", "nope", 0, 0), "null");
    assert_eq!(json("py", "xic", 0, 0), "null");
}
