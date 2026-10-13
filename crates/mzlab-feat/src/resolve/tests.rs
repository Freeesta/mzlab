use super::*;

fn gauss(n: usize, c: f64, h: f64, s: f64) -> impl Fn(usize) -> f64 {
    move |i| {
        let d = (i as f64 - c) / s;
        let _ = n;
        h * libm::exp(-0.5 * d * d)
    }
}

/// One feature spanning the whole trace `y` (scans every 0.01 min).
fn row(y: &[f64]) -> (FileFeatures, Vec<f64>) {
    let n = y.len();
    let mut f = FileFeatures {
        dt: 0.6,
        ..Default::default()
    };
    macro_rules! one {
        ($($c:ident = $v:expr),*) => { $( f.$c.push($v); )* };
    }
    one!(
        mz = 300.0,
        rt = 0.0,
        left = 0.0,
        right = 0.0,
        apex = 0,
        scan_lo = 0,
        scan_hi = (n - 1) as u32,
        area = 0.0,
        height = 0.0
    );
    one!(
        npts = n as u16,
        flags = 0,
        iso_group = 0,
        charge = 0,
        shape = 0.5,
        snr = 10.0,
        fwhm = 0.0,
        asym = 0.0,
        gaps = 0,
        qreason = 0
    );
    one!(ms2_n = 0, purity = -1.0, area_win = 0.0);
    (f, (0..n).map(|i| i as f64 * 0.01).collect())
}

fn run(y: &[f64]) -> Found {
    let (f, rt) = row(y);
    let mut out = Found::default();
    look(
        &f,
        0,
        &rt,
        y,
        0,
        &Params::default(),
        &QParams::default(),
        &ResolveParams::default(),
        &mut out,
    )
    .unwrap();
    out
}

fn two(sep: f64, h2: f64, s: f64) -> Vec<f64> {
    let (a, b) = (gauss(0, 40.0, 1000.0, s), gauss(0, 40.0 + sep, h2, s));
    (0..100 + sep as usize).map(|i| a(i) + b(i) + 3.0).collect()
}

#[test]
fn separated_ions_become_two_rows() {
    let f = run(&two(30.0, 800.0, 4.0));
    assert_eq!(f.pieces.len(), 2);
    assert!(f.multi.is_empty());
    let (a, b) = (&f.pieces[0].1, &f.pieces[1].1);
    assert!(a.flags[0] & FLAG_RESOLVED != 0 && a.apex[0] < b.apex[0]);
    assert!((a.apex[0] as i64 - 40).abs() <= 1 && (b.apex[0] as i64 - 70).abs() <= 1);
    assert!(a.scan_hi[0] <= b.scan_lo[0] + 1);
}

#[test]
fn apexes_closer_than_the_limit_stay_one() {
    // 3 scans = 0.03 min apart with a deep valley: not two ions
    let f = run(&two(3.0, 900.0, 0.7));
    assert!(f.pieces.is_empty());
}

#[test]
fn shallow_valley_is_a_possible_mixture() {
    // two maxima 0.16 min apart whose valley stays above half of the lower one
    let f = run(&two(16.0, 950.0, 5.0));
    assert!(f.pieces.is_empty());
    assert_eq!(f.multi.len(), 1);
}

#[test]
fn one_gaussian_is_left_alone() {
    let g = gauss(0, 50.0, 1000.0, 4.0);
    let y: Vec<f64> = (0..100).map(|i| g(i) + 3.0).collect();
    let f = run(&y);
    assert!(f.pieces.is_empty() && f.multi.is_empty());
}

#[test]
fn cosine_of_equal_spectra_is_one() {
    let a = vec![(1, 4.0), (5, 3.0)];
    assert!((cosine(&a, &a) - 1.0).abs() < 1e-12);
    assert_eq!(cosine(&a, &[(2, 1.0)]), 0.0);
    let mut m = vec![(1, 1.0)];
    add_into(&mut m, &[(0, 2.0), (1, 1.0), (7, 1.0)]);
    assert_eq!(m, vec![(0, 2.0), (1, 2.0), (7, 1.0)]);
}
