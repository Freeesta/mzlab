use super::*;

#[test]
fn the_clock_reads_the_epoch_as_a_date_without_zone() {
    assert_eq!(clock(0.0).as_deref(), Some("1970-01-01T00:00:00"));
    assert_eq!(
        clock(1_669_447_937.9).as_deref(),
        Some("2022-11-26T07:32:17")
    );
    assert_eq!(clock(951_782_400.0).as_deref(), Some("2000-02-29T00:00:00")); // leap day
    assert_eq!(clock(f64::NAN), None);
}

#[test]
fn identity_is_found_in_any_case() {
    let id = Ident {
        computer: "LAB-PC-7".into(),
        paths: vec!["C:\\Data\\Rossi\\run1".into()],
        ..Default::default()
    };
    assert!(id.names("Instrument on lab-pc-7") && id.names("file c:\\data\\rossi\\run1\\a.raw"));
    assert!(!id.names("Orbitrap Fusion"));
}

/// What the pass over the trailer kept of a numeric field before it stopped building the text of every value: the first text, whether all are empty, whether all
/// print the same, and the range.
fn riferimento(values: &[GenericValue]) -> (Option<String>, bool, bool, u64, u64) {
    let (mut first, mut all_empty, mut same) = (None::<String>, true, true);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in values {
        let t = shown(v);
        all_empty &= t.is_empty();
        match &first {
            None => first = Some(t.clone()),
            Some(f) => same &= *f == t,
        }
        if let Some(x) = v.as_f64() {
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    (first, all_empty, same, lo.to_bits(), hi.to_bits())
}

fn col() -> Col {
    Col {
        first: None,
        all_empty: true,
        same: true,
        numeric: true,
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
        distinct: Default::default(),
        first_bits: 0,
    }
}

#[test]
fn a_numeric_field_keeps_what_the_text_of_every_value_gave() {
    use GenericValue::{Float32, Float64, Int32, UInt16};
    let f = |xs: &[f64]| xs.iter().map(|&x| Float64(x)).collect::<Vec<_>>();
    let cases: Vec<Vec<GenericValue>> = vec![
        f(&[1.5]),
        f(&[1.5, 1.5, 1.5]),
        f(&[1.5, 1.5, 2.5, 1.5]),
        f(&[0.0, -0.0]),
        f(&[-0.0, 0.0, 0.0]),
        f(&[f64::NAN, f64::NAN]),
        f(&[f64::NAN, 1.0]),
        f(&[1.0, f64::NAN, 1.0]),
        f(&[f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY]),
        f(&[0.1 + 0.2, 0.3]),
        f(&[0.1 + 0.2, 0.1 + 0.2]),
        f(&[1e300, 1e300, 5e-324]),
        vec![Float32(0.1), Float32(0.1), Float32(0.1)],
        vec![Float32(0.1), Float32(0.2)],
        vec![Int32(-7), Int32(-7), Int32(-7), Int32(8)],
        vec![UInt16(65535), UInt16(65535)],
    ];
    for values in cases {
        let mut c = col();
        for v in &values {
            numeric_step(&mut c, v);
        }
        let got = (c.first, c.all_empty, c.same, c.lo.to_bits(), c.hi.to_bits());
        assert_eq!(got, riferimento(&values), "{values:?}");
    }
}
