//! Proofs and points (`evid/1`): the table of points is the one of `livelli/1` (the JSON of the crate is what `tools/esporta_livelli.py` writes from the
//! data file of the web page), the points are stepped and explain the order, a proof against is never offset, 1/N, the reason codes and the schema.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{self, Out, Value};
use mzlab_feat::evid::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn table() -> PointsTable {
    PointsTable::bundled().unwrap()
}

fn proof(t: &PointsTable, kind: Kind, value: f64, support: Option<f64>, opposed: bool) -> Evidence {
    Evidence::graded(t, kind, value, support, opposed, "test", 1, vec![])
}

#[test]
fn json_of_the_crate_is_what_the_data_file_gives() {
    let ok = Command::new("python3")
        .arg(root().join("tools/esporta_livelli.py"))
        .arg("--controlla")
        .status()
        .map(|s| s.success());
    let node = Command::new("node").arg("--version").output().is_ok();
    match ok {
        Ok(true) => {}
        _ if !node && std::env::var("MZLAB_GOLDEN_STRICT").is_err() => {}
        other => panic!("livelli-1.json: {other:?}"),
    }
}

#[test]
fn steps_of_the_table() {
    let t = table();
    let g = |k, v, s| t.grade(k, v, s).points;
    assert_eq!(
        (
            g(Kind::Mass, 1.5, None),
            g(Kind::Mass, -1.9, None),
            g(Kind::Mass, 3.0, None),
            g(Kind::Mass, 6.0, None)
        ),
        (2.0, 2.0, 1.0, 0.0)
    );
    // MS2: 3 points from the entropy of the profile (0.75) with 6 peaks, 1 from the provisional one (0.60); fewer peaks than the minimum, none
    assert_eq!(
        (
            g(Kind::Ms2, 0.8, Some(6.0)),
            g(Kind::Ms2, 0.65, Some(8.0)),
            g(Kind::Ms2, 0.8, Some(5.0)),
            g(Kind::Ms2, 0.8, None)
        ),
        (3.0, 1.0, 0.0, 0.0)
    );
    assert_eq!(g(Kind::Ms2, 0.5, Some(9.0)), 0.0);
    assert_eq!(g(Kind::Mass, f64::NAN, None), 0.0);
    assert_eq!(t.grade(Kind::Mass, 1.0, None).threshold, Some(2.0));
    assert_eq!(
        t.grade(Kind::Mass, 9.0, None).threshold,
        Some(5.0),
        "the lowest step when none is reached"
    );
    assert_eq!(t.grade(Kind::Controls, 1.0, None).threshold, None);
    assert_eq!((t.mass_ppm, t.ms2_margin), (5.0, 0.05));
}

#[test]
fn kinetics_is_informative_until_it_is_weighed() {
    let mut t = table();
    assert_eq!(t.grade(Kind::Kinetics, 0.1, None).points, 0.0);
    t.weighed = true;
    assert_eq!(t.grade(Kind::Kinetics, 0.1, None).points, 1.0);
    assert_eq!(t.grade(Kind::Kinetics, 1.5, None).points, 0.0);
}

#[test]
fn a_proof_against_is_never_offset() {
    let t = table();
    let good = vec![
        proof(&t, Kind::Mass, 1.0, None, false),
        proof(&t, Kind::Ms2, 0.9, Some(8.0), false),
        proof(&t, Kind::External, 0.9, None, false),
    ];
    let mut with_blank = good.clone();
    with_blank.push(proof(&t, Kind::Controls, 4.0, None, true));
    let weak = vec![proof(&t, Kind::Mass, 4.0, None, false)];
    let (a, b, c) = (
        breakdown(&t, good, 1),
        breakdown(&t, with_blank, 1),
        breakdown(&t, weak, 1),
    );
    assert_eq!(
        (a.total, b.total, c.total),
        (7.0, 7.0, 1.0),
        "the proof against takes nothing away and gives nothing"
    );
    assert_eq!((a.against, b.against), (0, 1));
    assert!(b.reasons.iter().any(|r| r.code == R_AGAINST_OPEN));
    // the order is by total; with the same total the one with nothing against comes first
    assert_eq!(b.rank(&c), std::cmp::Ordering::Less);
    assert_eq!(c.rank(&b), std::cmp::Ordering::Greater);
    assert_eq!(a.rank(&b), std::cmp::Ordering::Less);
    let mut forced = proof(&t, Kind::Mass, 1.0, None, true);
    assert_eq!((forced.points, forced.sign), (0.0, Sign::Against));
    forced.points = -5.0;
    assert_eq!(
        breakdown(&t, vec![forced], 1).total,
        0.0,
        "a stray negative number does not offset either"
    );
}

#[test]
fn the_kinetic_proof_enters_with_the_points_of_the_table() {
    use mzlab_feat::kinetic::{Evidence as K, Sign as KS};
    let k = |sign| K {
        kind: "cinetica",
        value: 0.2,
        threshold: 0.693,
        points: 0.0,
        sign,
        source: "mzlab-feat/kinetic",
        version: 1,
        params: vec![("points_if_weighed", 1.0)],
    };
    let mut t = table();
    let e = Evidence::from_kinetic(&t, &k(KS::For));
    assert_eq!(
        (e.kind, e.points, e.sign),
        (Kind::Kinetics, 0.0, Sign::Neutral)
    );
    assert!(breakdown(&t, vec![e], 1)
        .reasons
        .iter()
        .any(|r| r.code == R_KINETICS_INFO));
    assert_eq!(
        Evidence::from_kinetic(&t, &k(KS::Against)).sign,
        Sign::Against
    );
    assert_eq!(Evidence::from_kinetic(&t, &k(KS::Info)).points, 0.0);
    t.weighed = true;
    assert_eq!(Evidence::from_kinetic(&t, &k(KS::For)).points, 1.0);
    assert_eq!(
        Evidence::from_kinetic(&t, &k(KS::Info)).points,
        0.0,
        "an informative record does not earn points even when weighed"
    );
}

#[test]
fn one_over_n_counts_the_candidates_the_proofs_do_not_separate() {
    let t = table();
    let c = |mass, ms2| Candidate { mass, ms2 };
    let list = [
        c(300.1000, Some(0.80)),
        c(300.1001, Some(0.78)),
        c(300.1002, Some(0.40)),
        c(300.5, Some(0.80)),
        c(300.1000, None),
    ];
    assert_eq!(
        indistinguishable(&t, &list, 0),
        3,
        "itself, the close score and the one without MS2"
    );
    assert_eq!(
        indistinguishable(&t, &list, 2),
        2,
        "itself and the one without MS2: the others differ by more than the margin"
    );
    assert_eq!(indistinguishable(&t, &list, 3), 1, "another mass");
    assert_eq!(
        (one_over_n(1), one_over_n(4), one_over_n(0)),
        (1.0, 0.25, 1.0)
    );
}

#[test]
fn reason_codes() {
    let t = table();
    let b = breakdown(&t, vec![proof(&t, Kind::Mass, 1.0, None, false)], 3);
    let codes: Vec<_> = b.reasons.iter().map(|r| r.code).collect();
    assert_eq!(codes, [R_INDISTINGUISHABLE, R_NO_MS2]);
    assert_eq!(b.n, 3);
    let mut ms2 = proof(&t, Kind::Ms2, 0.9, Some(8.0), false);
    ms2.params.push(("purity", 0.3));
    let b = breakdown(&t, vec![ms2], 1);
    assert_eq!(
        b.reasons.iter().map(|r| r.code).collect::<Vec<_>>(),
        [R_WEAK_MS2]
    );
    assert_eq!(b.total, 3.0, "a weak MS2 is not discarded");
}

#[test]
fn the_frequency_alone_is_not_a_candidate() {
    let t = table();
    let freq = proof(&t, Kind::Frequency, 1.0, None, false);
    assert!(freq.points > 0.0);
    let b = breakdown(&t, vec![freq.clone()], 1);
    assert_eq!(b.total, 0.0);
    assert!(b.reasons.iter().any(|r| r.code == R_ONLY_FREQUENCY));
    let b = breakdown(&t, vec![freq, proof(&t, Kind::Mass, 1.0, None, false)], 1);
    assert!(b.total > 1.0, "with another proof the frequency counts");
    assert!(b.reasons.iter().all(|r| r.code != R_ONLY_FREQUENCY));
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.map(Value::arr)
        .unwrap_or(&[])
        .iter()
        .map(|x| x.str().unwrap().to_string())
        .collect()
}

#[test]
fn every_identity_has_its_table_of_contributions_and_it_follows_the_schema() {
    let t = table();
    let schema =
        json::parse(&std::fs::read(root().join("docs/schemi/evid-1.json")).unwrap()).unwrap();
    let defs = schema.get("$defs").unwrap();
    let kinds: Vec<_> = Kind::ALL.iter().map(|k| k.name().to_string()).collect();
    let signs: Vec<_> = [Sign::For, Sign::Against, Sign::Neutral]
        .iter()
        .map(|s| s.name().to_string())
        .collect();
    assert_eq!(strings(defs.get("tipo").unwrap().get("enum")), kinds);
    assert_eq!(strings(defs.get("segno").unwrap().get("enum")), signs);
    let required = strings(defs.get("prova").unwrap().get("required"));
    let identity_required = [
        "id",
        "contributi",
        "totale",
        "contro",
        "n",
        "probabilita",
        "motivi",
    ];
    let cases = [
        vec![
            proof(&t, Kind::Rt, 1.0, None, false),
            proof(&t, Kind::Mass, 1.0, None, false),
            proof(&t, Kind::Ms2, 0.9, Some(7.0), false),
        ],
        vec![
            proof(&t, Kind::Mass, f64::NAN, None, false),
            proof(&t, Kind::Controls, 3.0, None, true),
        ],
        vec![proof(&t, Kind::Frequency, 0.9, None, false)],
    ];
    for (id, rows) in cases.into_iter().enumerate() {
        let n = rows.len();
        let b = breakdown(&t, rows, id + 1);
        assert_eq!(b.rows.len(), n, "one row per proof");
        assert!(
            b.rows.windows(2).all(|w| w[0].kind <= w[1].kind),
            "in the order of the types"
        );
        let mut o = Out::default();
        breakdown_json(&mut o, id as u32, &b);
        let v = json::parse(o.0.as_bytes()).unwrap();
        for k in identity_required {
            assert!(v.get(k).is_some(), "identity without `{k}`");
        }
        assert_eq!(v.get("contributi").unwrap().arr().len(), n);
        for p in v.get("contributi").unwrap().arr() {
            for k in &required {
                assert!(p.get(k).is_some(), "proof without `{k}`");
            }
            assert!(kinds.contains(&p.get("tipo").unwrap().str().unwrap().to_string()));
            assert!(signs.contains(&p.get("segno").unwrap().str().unwrap().to_string()));
            assert!(p.get("punti").unwrap().f64().unwrap() >= 0.0);
        }
        assert!(
            (v.get("probabilita").unwrap().f64().unwrap() - 1.0 / (id + 1) as f64).abs() < 1e-6
        );
    }
}

#[test]
fn a_file_without_points_or_of_another_format_is_refused() {
    assert!(PointsTable::parse(b"{\"formato\":\"livelli/2\"}").is_err());
    assert!(PointsTable::parse(
        b"{\"formato\":\"livelli/1\",\"predefinito\":\"x\",\"profili\":{\"x\":{\"soglie\":{}}}}"
    )
    .is_err());
    assert!(PointsTable::parse(b"not json").is_err());
}
