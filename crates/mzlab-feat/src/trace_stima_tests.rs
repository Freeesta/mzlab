use super::*;

#[test]
fn a_detection_that_does_not_fit_is_refused_with_the_phase_and_its_numbers() {
    let r = Recipe::default();
    let x = Inputs {
        scans: 20_000,
        peaks: 40_000_000,
        kept: 8_000_000,
    };
    let need = bytes(&x, &r);
    assert!(check(&x, &r, need).is_ok());
    let e = check(&x, &r, need - 1).unwrap_err();
    let j = e.to_json();
    assert!(
        j.contains("\"error.memoryBudget\"") && j.contains("\"fase\":\"detect\""),
        "{j}"
    );
}

#[test]
fn only_the_sampled_intensities_at_or_above_the_floor_count() {
    let mut h = Hist::new();
    for v in [10.0, 10.0, 10.0, 1e5] {
        h.add(v);
    }
    assert_eq!(kept_of(&h, 400, 1e4), 100);
    assert_eq!(kept_of(&h, 400, 1.0), 400);
    assert_eq!(kept_of(&Hist::new(), 400, 1.0), 0);
}
