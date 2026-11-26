use super::*;

// a fragment the profile splits between two integers: two grid points at 106.7 and 107.0 (the centre of the peak is between them)
const MZ: [f64; 5] = [106.1, 106.7, 107.0, 107.4, 108.2];
const Y: [f64; 5] = [10.0, 800.0, 600.0, 20.0, 5.0];

#[test]
fn a_fragment_split_between_two_integers_is_one_nominal_bin() {
    let b = rebin(&MZ, &Y, Rule::Nominal, 0.0);
    let k107 = b.iter().find(|x| x.centre == 107.0).unwrap();
    // 106.7, 107.0 and 107.4 are all within [106.5, 107.5)
    assert_eq!(k107.points, 3);
    assert_eq!(k107.y, 1420.0);
    let com = (106.7 * 800.0 + 107.0 * 600.0 + 107.4 * 20.0) / 1420.0;
    assert!((k107.mz - com).abs() < 1e-12, "{}", k107.mz);
    assert_eq!(
        b.iter().map(|x| x.centre).collect::<Vec<_>>(),
        vec![106.0, 107.0, 108.0]
    );
}

#[test]
fn the_offset_is_taken_off_before_the_bins_are_cut() {
    // with +0.24 Da the same points sit at 106.46 and 106.76: they now fall in two bins, and the answer says so by its bins
    let b = rebin(&MZ, &Y, Rule::Nominal, 0.24);
    let k: Vec<f64> = b.iter().map(|x| x.centre).collect();
    assert_eq!(k, vec![106.0, 107.0, 108.0]);
    assert_eq!(b[0].points, 2); // 105.86 and 106.46
    assert!((b[0].mz - (105.86 * 10.0 + 106.46 * 800.0) / 810.0).abs() < 1e-12);
    // a smaller offset keeps 106.7 and 107.0 together: 106.6 and 106.9 are both in [106.5, 107.5)
    let one = rebin(&MZ[1..3], &Y[1..3], Rule::Nominal, 0.1);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].centre, 107.0);
}

#[test]
fn bins_are_half_open_at_the_lower_edge() {
    let b = rebin(
        &[106.5, 107.4999, 107.5],
        &[1.0, 1.0, 1.0],
        Rule::Nominal,
        0.0,
    );
    assert_eq!(
        b.iter().map(|x| (x.centre, x.points)).collect::<Vec<_>>(),
        vec![(107.0, 2), (108.0, 1)]
    );
}

#[test]
fn tenths_are_cut_around_multiples_of_a_tenth() {
    let b = rebin(
        &[106.68, 106.72, 106.76, 106.81],
        &[1.0, 1.0, 1.0, 1.0],
        Rule::Tenth,
        0.0,
    );
    let k: Vec<(f64, usize)> = b
        .iter()
        .map(|x| ((x.centre * 10.0).round() / 10.0, x.points))
        .collect();
    assert_eq!(k, vec![(106.7, 2), (106.8, 2)]);
}

#[test]
fn a_bin_of_zero_intensity_keeps_the_mean_mz_and_the_rule_is_parsed_strictly() {
    let b = rebin(&[106.8, 107.2], &[0.0, 0.0], Rule::Nominal, 0.0);
    assert_eq!(b.len(), 1);
    assert!((b[0].mz - 107.0).abs() < 1e-12);
    assert_eq!(Rule::parse("nominal"), Some(Rule::Nominal));
    assert_eq!(Rule::parse("0.1"), Some(Rule::Tenth));
    assert_eq!(Rule::parse("1"), None);
    assert!(rebin(&[], &[], Rule::Nominal, 0.3).is_empty());
}
