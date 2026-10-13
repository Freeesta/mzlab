//! The options of the one tree builder (`msnalbero`: tolerance, energies, blocks), `/api/msn_albero` and the filter `msn:<id>` of the other
//! routes on the committed synthetic MSn file (`tests/golden/ref/msntree_rotta`), and the parent of a node without the generation before in
//! `/api/msntree` (the nearest ancestor). Synthetic data only.

use std::path::PathBuf;

use mzlab_core::idxread::Index;
use mzlab_core::msnalbero::{albero_di, blocchi, Raggruppa, Tol};
use mzlab_core::msntree::{msn_tree, TreeScan};
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

type Step = (f64, String, f64);

fn p(v: &[(f64, f64)]) -> Vec<Step> {
    v.iter()
        .map(|&(m, ce)| (m, "CID".to_string(), ce))
        .collect()
}

#[test]
fn tolerance_energies_and_blocks() {
    let (a, b, c) = (
        p(&[(195.0876, 30.0)]),
        p(&[(195.0880, 35.0)]),
        p(&[(195.2101, 30.0)]),
    );
    let d = p(&[(195.0876, 30.0), (138.0662, 35.0)]);
    let scans: Vec<(u8, Option<&[Step]>)> = vec![
        (1, None),
        (2, Some(&a)),
        (2, Some(&a)),
        (1, None),
        (2, Some(&a)),
        (2, Some(&b)),
        (2, Some(&c)),
        (3, Some(&d)),
        (2, Some(&a)),
    ];
    // the default is msn.py: nominal masses, one energy one node
    let t = albero_di(scans.clone(), &Raggruppa::default());
    let ids: Vec<&str> = t.nodi.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["195@cid30", "195@cid35", "195@cid30>138@cid35"]);
    assert_eq!(t.nodi[0].scansioni, [1, 2, 4, 6, 8]);
    // within 0.01 Da: 195.0876 and 195.0880 are one precursor, 195.2101 another
    let da = Raggruppa {
        tol: Tol::Da(0.01),
        energie: true,
    };
    let t = albero_di(scans.clone(), &da);
    let ids: Vec<&str> = t.nodi.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "195.0878@cid30",
            "195.0878@cid35",
            "195.2101@cid30",
            "195.0878@cid30>138.0662@cid35"
        ]
    );
    assert_eq!((t.nodi[3].genitore, t.nodi[3].salto), (Some(0), 0));
    // 10 ppm (2 mDa at m/z 195) does the same; energies merged: one node per precursor
    let ppm = Raggruppa {
        tol: Tol::Ppm(10.0),
        energie: false,
    };
    let t = albero_di(scans.clone(), &ppm);
    let ids: Vec<&str> = t.nodi.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        ["195.0878@cid", "195.2101@cid", "195.0878@cid>138.0662@cid"]
    );
    assert_eq!(t.nodi[0].scansioni, [1, 2, 4, 5, 8]);
    assert!(t.nodi[0].chiave[0].2.is_nan());
    // blocks: runs with no scan of another node between them (an MS1 does not break a run)
    let t = albero_di(scans, &Raggruppa::default());
    assert_eq!(blocchi(&t, 9)[0], [(1, 4), (6, 6), (8, 8)]);
}

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta")
}

fn routes() -> Routes<std::fs::File> {
    let p = golden().join("msn.mzidx");
    let len = std::fs::metadata(&p).unwrap().len();
    Routes::new(Index::open(std::fs::File::open(&p).unwrap(), len).unwrap())
}

fn get(r: &mut Routes<std::fs::File>, path: &str, q: &str) -> Value {
    serde_json::from_str(&r.handle(path, &Query::parse(q)).unwrap().unwrap()).unwrap()
}

#[test]
fn tree_route_and_node_filter() {
    let mut r = routes();
    let t = get(&mut r, "/api/msn_albero", "k=0");
    assert_eq!(t["formato"], "msn_albero/1");
    let nodes = t["nodi"].as_array().unwrap();
    assert!(nodes.len() >= 3, "{t}");
    assert_eq!(t["livelli"], serde_json::json!([2, 3]));
    let tree = mzlab_core::msnalbero::albero(&r.idx.scans, &Raggruppa::default());
    for (n, want) in nodes.iter().zip(&tree.nodi) {
        let (id, level) = (n["id"].as_str().unwrap(), n["livello"].as_u64().unwrap());
        assert_eq!(id, want.id);
        assert_eq!(n["n"].as_u64().unwrap() as usize, want.scansioni.len());
        // a cycled method: one block per scan
        assert_eq!(n["blocchi"].as_array().unwrap().len(), want.scansioni.len());
        let q = format!("k=0&level={level}&filt=msn:{id}");
        let list = get(&mut r, "/api/scanlist", &q);
        let sids: Vec<usize> = list["sid"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(
            sids, want.scansioni,
            "{id}: the scans of the filter are the scans of the node"
        );
        // the TIC of the node: one point per scan of the node, the TIC of the level at those scans
        let tic = get(&mut r, "/api/chrom", &format!("{q}&kind=tic"));
        let all = get(&mut r, "/api/chrom", &format!("k=0&level={level}&kind=tic"));
        let (rt, y) = (tic["rt"].as_array().unwrap(), tic["y"].as_array().unwrap());
        assert_eq!(rt.len(), want.scansioni.len());
        let at = |t: &Value| {
            all["rt"]
                .as_array()
                .unwrap()
                .iter()
                .position(|v| v == t)
                .unwrap()
        };
        let sum: f64 = y.iter().map(|v| v.as_f64().unwrap()).sum();
        let want_sum: f64 = rt.iter().map(|t| all["y"][at(t)].as_f64().unwrap()).sum();
        assert!(
            (sum - want_sum).abs() <= 1e-9 * want_sum.max(1.0),
            "{id}: {sum} against {want_sum}"
        );
        let near = get(
            &mut r,
            "/api/nearest_scan",
            &format!("k=0&level={level}&rt=0&filter=msn:{id}"),
        );
        assert_eq!(
            near["sid"].as_u64().unwrap() as usize,
            want.scansioni[0],
            "{id}: {near}"
        );
    }
    // energies merged: the ids lose the energy; an unknown node and a bad tolerance are keyed errors
    let merged = get(&mut r, "/api/msn_albero", "k=0&tol=0.01&ce=0");
    assert!(merged["nodi"][0]["ce"].is_null(), "{merged}");
    let err = |r: &mut Routes<std::fs::File>, q: &str| {
        r.handle("/api/scanlist", &Query::parse(q)).unwrap_err().key
    };
    assert_eq!(err(&mut r, "k=0&level=2&filt=msn:999@cid1"), "err.msn.node");
    assert_eq!(
        err(&mut r, "k=0&level=2&filt=msn:305@cid35&mtol=abc"),
        "err.msn.tol"
    );
    // without `msn:` the routes answer as before
    let plain = get(&mut r, "/api/scanlist", "k=0&level=2");
    assert_eq!(
        plain["n"].as_u64().unwrap() as usize,
        r.idx.scans.iter().filter(|s| s.level == 2).count()
    );
}

#[test]
fn msntree_hangs_a_lone_node_from_the_nearest_ancestor() {
    let paths = [
        (1u8, vec![]),
        (2, p(&[(195.0, 30.0)])),
        (4, p(&[(195.0, 30.0), (156.0, 30.0), (138.0, 35.0)])),
        (3, p(&[(195.0, 30.0), (138.0, 35.0)])),
    ];
    let scans: Vec<TreeScan> = paths
        .iter()
        .enumerate()
        .flat_map(|(i, (level, path))| {
            (0..3).map(move |k| TreeScan {
                index: i * 3 + k,
                level: *level,
                polarity: 1,
                path: path.clone(),
            })
        })
        .collect();
    let read = |_| {
        Ok((
            vec![110.0, 138.06, 156.07, 195.09],
            vec![10.0, 50.0, 30.0, 100.0],
        ))
    };
    let t: Value = serde_json::from_str(&msn_tree(&scans, read, None, 5.0).unwrap()).unwrap();
    let parents: Vec<&Value> = t["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| &n["parent"])
        .collect();
    // 195 > 156 > 138 has no 195 > 156: its parent is 195 (node 0), not the MS1
    assert_eq!(parents, [&Value::Null, &Value::from(0), &Value::from(0)]);
}
