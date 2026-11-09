//! Elemental composition in positive and negative mode: the electron mass is taken off a cation and added to an anion, so the same neutral
//! (glucose, C6H12O6, M = 180.063388) gives the same formula and a near-zero error from [M+H]+, [M-H]-, [M+Na]+ and [M+NH4]+ (synthetic values).

use mzlab_core::composition::{compose, parse_elements, Opts};
use serde_json::Value;

const ELECTRON: f64 = 0.000548579909;

/// The best candidate (formula, error in ppm) for an m/z, ion and the elements of glucose plus Na and N (the tolerance in ppm is the last argument).
fn best(mz: f64, ion: &str, tol: f64) -> (String, f64) {
    let o = Opts {
        elements: Some(parse_elements("C:0-6,H:0-16,O:0-6,N:0-1,Na:0-1").unwrap()),
        ion,
        tol,
        max_results: 5,
        ..Opts::default()
    };
    let j: Value = serde_json::from_str(&compose(mz, &o).unwrap()).unwrap();
    let r = &j["results"][0];
    if r.is_null() {
        panic!("{ion} {mz}: {j}");
    }
    (
        r["formula"].as_str().unwrap().to_string(),
        r["delta_ppm"].as_f64().unwrap(),
    )
}

#[test]
fn positive_and_negative_modes_agree() {
    // (ion, m/z computed by hand from the masses of C, H, O, N, Na and the electron)
    for (ion, mz, formula) in [
        ("[M+H]+", 181.070665, "C6H13O6"),
        ("[M-H]-", 179.056112, "C6H11O6"),
        ("[M+Na]+", 203.052609, "C6H12NaO6"),
        ("[M+NH4]+", 198.097214, "C6H16NO6"),
    ] {
        let (f, ppm) = best(mz, ion, 2.0);
        assert_eq!(f, formula, "{ion}");
        assert!(ppm.abs() < 0.02, "{ion}: {ppm} ppm");
    }
}

#[test]
fn the_electron_has_the_right_sign() {
    // an m/z with the electron on the wrong side of the ion is about 6 ppm off at m/z 181 and 179 (2 x 0.55 mDa)
    let (_, plus) = best(181.070665 + 2.0 * ELECTRON, "[M+H]+", 10.0);
    let (_, minus) = best(179.056112 - 2.0 * ELECTRON, "[M-H]-", 10.0);
    assert!(
        (plus.abs() - 2.0 * ELECTRON / 181.07 * 1e6).abs() < 0.1,
        "{plus}"
    );
    assert!(
        (minus.abs() - 2.0 * ELECTRON / 179.06 * 1e6).abs() < 0.1,
        "{minus}"
    );
    // and the error of the same candidate has the sign of the m/z that was typed, in both modes
    assert!(plus < 0.0 && minus > 0.0, "{plus} {minus}");
}

/// The isotope pool says how many candidates it scored when it left some out, and stays silent otherwise (synthetic glucose isotopes).
#[test]
fn the_isotope_pool_names_how_many_it_scored() {
    let run = |pool: usize, obs: Vec<(f64, f64)>| -> Value {
        let o = Opts {
            elements: Some(parse_elements("C:0-12,H:0-30,O:0-8,N:0-4,S:0-2,P:0-1").unwrap()),
            tol: 50.0,
            max_results: 2,
            pool,
            obs,
            ..Opts::default()
        };
        serde_json::from_str(&compose(181.070665, &o).unwrap()).unwrap()
    };
    let obs = vec![(181.070665, 100.0), (182.074, 6.7)];
    let cut = run(3, obs.clone());
    let n = cut["n_candidates"].as_i64().unwrap();
    assert!(n > 3, "{cut}");
    assert_eq!(cut["scored"].as_i64(), Some(3), "{cut}");
    assert!(
        run(n as usize, obs)["scored"].is_null(),
        "a pool that holds every candidate is not a cut"
    );
    assert!(
        run(3, vec![])["scored"].is_null(),
        "no observed peaks: nothing is scored, nothing to report"
    );
}
