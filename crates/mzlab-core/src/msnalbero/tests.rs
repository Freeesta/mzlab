use super::*;

type Step = (f64, String, f64);

fn p(v: &[(f64, f64)]) -> Vec<(f64, String, f64)> {
    v.iter()
        .map(|&(m, ce)| (m, "CID".to_string(), ce))
        .collect()
}

#[test]
fn nodes_parents_and_skipped_generations() {
    let a = p(&[(195.09, 30.0)]);
    let b = p(&[(195.1, 35.0)]);
    let c = p(&[(195.0, 35.0), (138.0, 35.0)]);
    let d = p(&[(195.0, 30.0), (138.0, 35.0), (110.0, 35.0)]);
    let e = p(&[(195.0, 30.0), (150.0, 35.0), (110.0, 35.0)]);
    let bad = p(&[(195.0, 30.0)]);
    let scans: Vec<(u8, Option<&[Step]>)> = vec![
        (1, None),
        (2, Some(&a)),
        (2, Some(&a)),
        (2, Some(&b)),
        (3, Some(&c)),
        (4, Some(&d)),
        (4, Some(&e)),
        (3, Some(&bad)),
        (2, None),
    ];
    let t = albero_di(scans, &Raggruppa::default());
    assert_eq!((t.ms1.clone(), t.scartate), (vec![0], 2));
    let ids: Vec<&str> = t.nodi.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "195@cid30",
            "195@cid35",
            "195@cid35>138@cid35",
            "195@cid30>138@cid35>110@cid35",
            "195@cid30>150@cid35>110@cid35"
        ]
    );
    assert_eq!(t.nodi[0].scansioni, [1, 2]);
    // the same nominal path with another energy is the parent; with no node of that path, the nearest ancestor (one generation skipped)
    assert_eq!(t.nodi[2].genitore, Some(1));
    assert_eq!(
        (t.nodi[3].genitore, t.nodi[3].salto, t.nodi[3].livello),
        (Some(2), 0, 4)
    );
    assert_eq!((t.nodi[4].genitore, t.nodi[4].salto), (Some(0), 1));
}

#[test]
fn consensus_keeps_the_peaks_of_half_the_scans() {
    let s = vec![
        (vec![100.0, 150.0], vec![10.0, 5.0]),
        (vec![100.0005], vec![8.0]),
    ];
    let (mz, rel, n) = consenso(&s, 10.0, 0.5, 0.0);
    assert_eq!(n, [2, 1]);
    assert!(
        (mz[0] - 100.00025).abs() < 1e-9
            && (rel[0] - 1.0).abs() < 1e-12
            && (rel[1] - 0.25).abs() < 1e-12
    );
}
