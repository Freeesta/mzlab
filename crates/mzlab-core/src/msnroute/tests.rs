use super::*;
use crate::idxread::Index;

fn routes() -> Routes<std::fs::File> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/ref/msntree_rotta/msn.mzidx");
    let len = std::fs::metadata(&p).unwrap().len();
    Routes::new(Index::open(std::fs::File::open(&p).unwrap(), len).unwrap())
}

#[test]
fn sum_is_mean_times_n() {
    let mut r = routes();
    let tree = albero(&r.idx.scans, &Raggruppa::default());
    for n in &tree.nodi {
        let f = format!("msn:{}", n.id);
        let mean = r
            .binned(n.livello, 0.0, 1e9, None, 0.1, Some(&f))
            .unwrap()
            .unwrap();
        let sum = r
            .con_opz(&Query::parse("modo=somma"), |r| {
                r.binned(n.livello, 0.0, 1e9, None, 0.1, Some(&f))
            })
            .unwrap()
            .unwrap();
        assert_eq!((mean.n, sum.n), (n.scansioni.len(), n.scansioni.len()));
        assert_eq!(mean.mz, sum.mz);
        for (a, b) in mean.y.iter().zip(&sum.y) {
            assert!(
                (a * mean.n as f64 - b).abs() <= 1e-9 * b.abs().max(1.0),
                "{}: {a} x {} against {b}",
                n.id,
                mean.n
            );
        }
    }
    // the option lasts one request
    assert!(!r.opz.somma);
}

#[test]
fn scanavg_sums_too() {
    let mut r = routes();
    let tree = albero(&r.idx.scans, &Raggruppa::default());
    let sids: Vec<String> = tree.nodi[0]
        .scansioni
        .iter()
        .map(|i| i.to_string())
        .collect();
    let mut mzml = std::fs::File::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden/ref/msntree_rotta/msn.mzML"),
    )
    .unwrap();
    let mut ask = |q: &str| -> crate::json::Value {
        crate::json::parse(
            r.mzml_route("/api/scanavg", &Query::parse(q), &mut mzml)
                .unwrap()
                .unwrap()
                .as_bytes(),
        )
        .unwrap()
    };
    let q = format!("sids={}", sids.join(","));
    let (mean, sum) = (ask(&q), ask(&format!("{q}&modo=somma")));
    let n = sids.len() as f64;
    let ys = |v: &crate::json::Value| -> Vec<f64> {
        v.get("y")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.f64().unwrap())
            .collect()
    };
    for (a, b) in ys(&mean).iter().zip(ys(&sum)) {
        // the answer is rounded to 0.1
        assert!(
            (a * n - b).abs() <= 0.05 * n + 0.05,
            "{a} x {n} against {b}"
        );
    }
}
