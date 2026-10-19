//! Local-maximum mass detector: parity with the independent numpy reference (`tools/localmax_riferimento.py`) and small hand cases.

use mzlab_feat::untargeted::localmax::{detect, max_mz_gap, Params};
use mzlab_feat::untargeted::smooth::Method;
use serde_json::Value;

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn matches_the_numpy_reference() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/localmax/casi.json"
    );
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let smoothing = match c["method"].as_str().unwrap() {
            "gauss" => Method::Gaussian,
            "sg" => Method::SavitzkyGolay,
            _ => Method::None,
        };
        let p = Params {
            noise: c["noise"].as_f64().unwrap(),
            area: c["area"].as_bool().unwrap(),
            smoothing,
            window: c["window"].as_u64().unwrap() as usize,
            ..Params::default()
        };
        let (mz, y) = detect(&vec_of(&c["mz"]), &vec_of(&c["y"]), &p);
        let (want_mz, want_y) = (vec_of(&c["peak_mz"]), vec_of(&c["peak_y"]));
        assert_eq!(mz.len(), want_mz.len(), "{name}: number of peaks");
        for i in 0..mz.len() {
            assert!(
                (mz[i] - want_mz[i]).abs() <= 1e-9 * want_mz[i],
                "{name}: m/z {} vs {}",
                mz[i],
                want_mz[i]
            );
            assert!(
                (y[i] - want_y[i]).abs() <= 1e-9 * want_y[i].max(1.0),
                "{name}: intensity {} vs {}",
                y[i],
                want_y[i]
            );
        }
    }
}

#[test]
fn the_gap_is_the_top_step_or_the_start_step_if_larger() {
    let mz = [100.0, 100.1, 100.3, 100.7];
    assert!((max_mz_gap(&mz, &[1.0, 1.0, 1.0, 1.0]) - 0.4).abs() < 1e-12);
    // no pair of non-zero points: the default 0.1
    assert_eq!(max_mz_gap(&mz, &[1.0, 0.0, 1.0, 0.0]), 0.1);
}

#[test]
fn a_hand_spectrum_gives_one_centroid() {
    // growing steps (a uniform grid would split at every point: each step equals the largest one); the weighted mean of the points over
    // 0.4 x apex within the symmetric window: (100.1 x 50 + 100.21 x 100 + 100.33 x 50) / 200
    let mz = [100.0, 100.1, 100.21, 100.33, 100.46];
    let y = [10.0, 50.0, 100.0, 50.0, 10.0];
    let (m, h) = detect(&mz, &y, &Params::default());
    assert_eq!(h, vec![100.0]);
    assert!((m[0] - 100.2125).abs() < 1e-12);
    let (_, area) = detect(
        &mz,
        &y,
        &Params {
            area: true,
            ..Params::default()
        },
    );
    // the last step equals the largest one, so as in mzmine the last point closes the run on its own: area of the first four points
    assert_eq!(area, vec![210.0]);
}

#[test]
fn too_few_points_or_all_below_the_noise_give_nothing() {
    assert!(detect(&[100.0, 100.1], &[5.0, 9.0], &Params::default())
        .0
        .is_empty());
    let mz = [100.0, 100.1, 100.2, 100.3];
    assert!(detect(
        &mz,
        &[1.0, 2.0, 1.0, 1.0],
        &Params {
            noise: 5.0,
            ..Params::default()
        }
    )
    .0
    .is_empty());
}
