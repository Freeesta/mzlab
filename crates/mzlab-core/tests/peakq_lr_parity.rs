//! `peakq_lr` against the values the page's JS (`autoEdges`, `sgFilter`, `noiseSD` of picchi.js / cromato.js) gave on synthetic traces before it
//! moved to Rust (tests/golden/picchi_lr.json: 672 edge cases over 6 traces, 7 parameter sets, MRM or not, with and without a reference width).
use mzlab_core::peakq_lr::{lr_edges, noise_sd, sg_filter, LrParams};
use serde_json::Value;

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn golden() -> Value {
    serde_json::from_str(include_str!("golden/picchi_lr.json")).unwrap()
}

#[test]
fn edges_equal_the_page_values() {
    let g = golden();
    let traces: Vec<(Vec<f64>, Vec<f64>)> = g["traces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (nums(&t["xs"]), nums(&t["ys"])))
        .collect();
    let (mut some, mut none) = (0, 0);
    for (i, c) in g["cases"].as_array().unwrap().iter().enumerate() {
        let (xs, ys) = &traces[c["t"].as_u64().unwrap() as usize];
        let pk = LrParams::from_slice(&nums(&c["p"])).unwrap();
        let got = lr_edges(
            xs,
            ys,
            c["at"].as_f64().unwrap(),
            c["ref"].as_f64().unwrap(),
            c["mrm"].as_bool().unwrap(),
            &pk,
        );
        match (&c["e"], got) {
            (Value::Null, None) => none += 1,
            (e, Some((a, b))) if !e.is_null() => {
                let e = nums(e);
                assert!(
                    (a - e[0]).abs() < 1e-9 && (b - e[1]).abs() < 1e-9,
                    "case {i}: {a} {b} vs {e:?}"
                );
                some += 1;
            }
            (e, got) => panic!("case {i}: {got:?} vs {e:?}"),
        }
    }
    assert!(some > 300 && none > 10, "{some} {none}");
}

#[test]
fn smoothing_and_noise_equal_the_page_values() {
    let g = golden();
    let traces = g["traces"].as_array().unwrap();
    for s in g["sg"].as_array().unwrap() {
        let y = nums(&traces[s["t"].as_u64().unwrap() as usize]["ys"]);
        let got = sg_filter(
            &y[..300],
            s["m"].as_u64().unwrap() as usize,
            s["d"].as_bool().unwrap(),
        );
        for (a, b) in got.iter().zip(nums(&s["o"])) {
            assert!((a - b).abs() < 1e-9 * (1.0 + b.abs()), "{a} {b}");
        }
    }
    for (t, z) in traces.iter().zip(g["nz"].as_array().unwrap()) {
        let got = noise_sd(
            &nums(&t["xs"]),
            &nums(&t["ys"]),
            z["a"].as_f64().unwrap(),
            z["b"].as_f64().unwrap(),
        )
        .unwrap();
        assert!((got - z["o"].as_f64().unwrap()).abs() < 1e-9, "{got}");
    }
}
