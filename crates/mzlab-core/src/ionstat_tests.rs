//! `ionstat` on synthetic ion-count traces (multiples of a quantum of cps per ion, Poisson background from a fixed seed): the MAD = 0 case,
//! the all-zero trace, a counting background where the Poisson term matters, the quantum rule and the JSON keys.

use super::*;

/// Poisson counts with mean `lam` (Knuth, a fixed linear congruential generator: the same numbers on every platform).
fn poisson(n: usize, lam: f64, seed: u64) -> Vec<f64> {
    let mut s = seed;
    let mut u = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 11) as f64) / ((1u64 << 53) as f64)
    };
    (0..n)
        .map(|_| {
            let (l, mut k, mut p) = ((-lam).exp(), 0.0, 1.0);
            loop {
                p *= u();
                if p <= l {
                    break k;
                }
                k += 1.0;
            }
        })
        .collect()
}

/// A trace of 300 points every 0.02 min: background counts times `q`, plus `ions` counts added at the points of `at`.
fn trace(q: f64, lam: f64, at: &[(usize, f64)]) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..300).map(|i| i as f64 * 0.02).collect();
    let mut c = poisson(300, lam, 7);
    for &(i, ions) in at {
        c[i] += ions;
    }
    (rt, c.iter().map(|k| k * q).collect())
}

fn peak_ask() -> FloorAsk {
    FloorAsk {
        from: None,
        to: None,
        peak: Some((2.9, 3.1)),
    }
}

#[test]
fn two_ions_on_zero_background_are_one_ion_noise_and_fail_at_five() {
    // EMS-like: 2222 cps per ion, background 0.02 ion per point, so the MAD is 0
    let (rt, y) = trace(2222.0, 0.02, &[(150, 2.0), (151, 1.0)]);
    let r = measure(&rt, &y, peak_ask(), FloorParams::default()).unwrap();
    let f = &r.floor;
    assert_eq!(f.mad, 0.0);
    assert_eq!(f.one_ion, Some(2222.0));
    assert_eq!(f.one_ion_source, "auto");
    assert_eq!(f.estimator, "one_ion");
    assert_eq!(f.noise, 2222.0);
    let pk = r.peak.as_ref().unwrap();
    assert!((pk.ions.unwrap() - 2.0).abs() < 1e-9, "{pk:?}");
    assert_eq!(pk.sn_mad, None, "a pure MAD divides by zero");
    assert!((pk.sn.unwrap() - 2.0).abs() < 1e-9);
    let c = r.check.as_ref().unwrap();
    assert_eq!(c.status, "fail", "{}", c.rule);
    assert!(c.rule.contains("S/N 2.0 < 5.0 -> fail"), "{}", c.rule);
}

#[test]
fn without_the_ion_term_the_same_trace_passes_by_a_large_factor_and_is_a_warning() {
    let (rt, y) = trace(2222.0, 0.02, &[(150, 2.0), (151, 1.0)]);
    let p = FloorParams {
        one_ion: OneIon::Off,
        ..FloorParams::default()
    };
    let r = measure(&rt, &y, peak_ask(), p).unwrap();
    assert_eq!(r.floor.estimator, "sd");
    let sn = r.peak.as_ref().unwrap().sn.unwrap();
    assert!(sn > 10.0, "S/N {sn} on 0.5 SD alone");
    assert_eq!(r.check.as_ref().unwrap().status, "warn");
}

#[test]
fn twenty_ions_pass_and_a_set_value_wins_over_the_quantum() {
    let (rt, y) = trace(1000.0, 0.05, &[(149, 6.0), (150, 20.0), (151, 8.0)]);
    let p = FloorParams {
        one_ion: OneIon::Cps(1000.0),
        ..FloorParams::default()
    };
    let r = measure(&rt, &y, peak_ask(), p).unwrap();
    assert_eq!(r.floor.one_ion_source, "set");
    assert_eq!(r.check.as_ref().unwrap().status, "pass");
    assert!(r.peak.as_ref().unwrap().sn.unwrap() >= 20.0);
}

#[test]
fn an_all_zero_trace_has_no_quantum_and_no_defined_sn_until_one_ion_is_set() {
    let rt: Vec<f64> = (0..50).map(|i| i as f64 * 0.02).collect();
    let mut y = vec![0.0; 50];
    y[25] = 3000.0;
    let ask = FloorAsk {
        peak: Some((0.49, 0.51)),
        ..FloorAsk::default()
    };
    let r = measure(&rt, &y, ask, FloorParams::default()).unwrap();
    assert_eq!(r.floor.quantum, None);
    assert_eq!(r.floor.one_ion_source, "auto_rejected");
    assert_eq!(r.floor.noise, 0.0);
    let c = r.check.as_ref().unwrap();
    assert_eq!(c.status, "warn");
    assert!(c.rule.contains("S/N not defined"), "{}", c.rule);
    let set = FloorParams::from_slice(&[1000.0]);
    let r = measure(&rt, &y, ask, set).unwrap();
    assert_eq!((r.floor.noise, r.floor.estimator), (1000.0, "one_ion"));
    assert_eq!(r.check.unwrap().status, "fail");
}

#[test]
fn a_counting_background_takes_the_larger_of_mad_and_poisson() {
    // nine ions per point: the counting error is 3 ions, the MAD of Poisson counts is close to it
    let (rt, y) = trace(1000.0, 9.0, &[]);
    let r = measure(&rt, &y, FloorAsk::default(), FloorParams::default()).unwrap();
    let f = &r.floor;
    let lam = f.lambda.unwrap();
    assert!((lam - 9.0).abs() < 1.0, "lambda {lam}");
    assert!((f.ion_term.unwrap() - 1000.0 * lam.sqrt()).abs() < 1e-6);
    assert!(matches!(f.estimator, "mad" | "poisson"), "{}", f.estimator);
    assert!(f.noise >= f.ion_term.unwrap() && f.noise >= f.mad);
    assert_eq!(r.check, None, "no peak window, no check");
}

#[test]
fn the_quantum_needs_its_support() {
    let counts = quantum(&[0.0, 2222.0, 4444.0, 6666.0, 2222.0]).unwrap();
    assert!(counts.accepted && counts.q == 2222.0 && counts.support == 1.0);
    let analog = quantum(&[0.0, 103.0, 251.7, 377.2, 190.4, 333.3]).unwrap();
    assert!(!analog.accepted, "{analog:?}");
    let few = quantum(&[0.0, 2222.0, 4444.0]).unwrap();
    assert!(!few.accepted, "two positive values are too few");
    assert_eq!(quantum(&[0.0, 0.0]), None);
    // no point holds a single ion, the steps between values do
    let high = quantum(&[7000.0, 9000.0, 8000.0, 11000.0, 9000.0]).unwrap();
    assert!(high.accepted && high.q == 1000.0, "{high:?}");
}

#[test]
fn the_slice_of_the_wasm_interface() {
    let d = FloorParams::default();
    assert_eq!(FloorParams::from_slice(&[]), d);
    assert_eq!(FloorParams::from_slice(&[f64::NAN, -1.0, -1.0]), d);
    assert_eq!(FloorParams::from_slice(&[-1.0]).one_ion, OneIon::Auto);
    assert_eq!(FloorParams::from_slice(&[0.0]).one_ion, OneIon::Off);
    let p = FloorParams::from_slice(&[2222.0, 0.0, 3.0]);
    assert_eq!((p.one_ion, p.sd_weight, p.min_sn), (OneIon::Cps(2222.0), 0.0, 3.0));
}

#[test]
fn no_background_point_is_an_error_and_the_json_keys_are_stable() {
    let rt = [0.0, 0.1, 0.2];
    let y = [0.0, 1000.0, 0.0];
    let all = FloorAsk {
        peak: Some((0.0, 0.2)),
        ..FloorAsk::default()
    };
    assert_eq!(
        measure(&rt, &y, all, FloorParams::default()).unwrap_err(),
        "err.noise.noBackground"
    );
    let ask = FloorAsk {
        peak: Some((0.05, 0.15)),
        ..FloorAsk::default()
    };
    let r = measure(&rt, &y, ask, FloorParams::from_slice(&[1000.0])).unwrap();
    assert_eq!(
        floor_json(&r),
        "{\"schema\":\"noise-floor/1\",\"params\":{\"one_ion\":1000,\"sd_weight\":0.5,\"min_sn\":5,\"mad_k\":1.4826,\"from\":null,\"to\":null},\
\"background\":{\"n\":2,\"median\":0,\"mean\":0},\"quantum\":null,\"terms\":{\"mad\":0,\"sd\":0,\"sd_term\":0,\"one_ion\":1000,\
\"one_ion_source\":\"set\",\"lambda\":0,\"ion_term\":1000},\"noise\":1000,\"estimator\":\"one_ion\",\"peak\":{\"from\":0.05,\"to\":0.15,\
\"apex_rt\":0.1,\"height\":1000,\"sn\":1,\"sn_mad\":null,\"ions\":1},\"check\":{\"id\":\"noise_floor\",\"status\":\"fail\",\"value\":1,\
\"threshold\":5,\"rule\":\"noise 1000 = one_ion (MAD 0.000, 0.500 SD 0.000, ion term 1000); S/N 1.0 < 5.0 -> fail\"}}"
    );
    assert_eq!(
        floor_log(&r),
        "noise floor: 1000 cps from one_ion (2 background points, one ion 1000 [set]); noise 1000 = one_ion (MAD 0.000, 0.500 SD 0.000, ion term 1000); S/N 1.0 < 5.0 -> fail"
    );
}
