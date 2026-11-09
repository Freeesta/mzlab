use super::interpreta::PPM;
use super::tests::{edge, find, shifted_graph};
use super::*;

fn con(formula: Option<&str>) -> Options {
    Options {
        formula: formula.map(String::from),
        ..Options::default()
    }
}

fn of_edge<'a>(g: &Graph, v: &'a [EdgeInterp], da: &str, a: &str) -> &'a EdgeInterp {
    let (p, f) = (find(g, da), find(g, a));
    v.iter().find(|x| x.da == p && x.a == f).expect("arc")
}

#[test]
fn the_table_has_unique_formulas_and_exact_masses() {
    let t = known_losses();
    assert_eq!(t.len(), 34);
    let mut f: Vec<_> = t.iter().map(|n| n.formula.clone()).collect();
    f.sort();
    f.dedup();
    assert_eq!(f.len(), 34);
    let h2o = t.iter().find(|n| n.formula == "H2O").unwrap();
    assert!((h2o.massa - 18.010565).abs() < 1e-5);
}

#[test]
fn with_formulas_a_loss_is_named_or_left_unlisted() {
    let (g, _) = shifted_graph(&con(Some("C8H10N4O2+H")), &[0, 1, 2, 3], false, 0.0);
    let v = interpreta(&g, PPM);
    assert_eq!(v.len(), g.archi.len());
    let co = of_edge(&g, &v, "C6H8N3O", "C5H8N3");
    let p = co.loss.as_ref().unwrap();
    assert_eq!(
        (p.name, p.provenance, p.confidence),
        (Some("carbon monoxide"), "formula", "high")
    );
    assert_eq!(p.formula.as_deref(), Some("CO"));
    assert!(p.ppm_error.abs() < 0.1);
    assert_eq!(co.attivazioni, ["CID", "HCD"]);
    let list = of_edge(&g, &v, "C8H11N4O2", "C6H8N3O");
    let q = list.loss.as_ref().unwrap();
    assert_eq!((q.name, q.confidence), (None, "low"));
    assert_eq!(q.formula.as_deref(), Some("C2H3NO"));
    assert!(edge(&g, "C8H11N4O2", "C6H8N3O").is_some());
}

#[test]
fn without_formulas_the_loss_is_matched_by_mass_only() {
    let (g, _) = shifted_graph(&con(None), &[0, 1, 2, 3], false, 0.0);
    let v = interpreta(&g, PPM);
    let p = of_edge(&g, &v, "C6H8N3O", "C5H8N3").loss.clone().unwrap();
    assert_eq!(
        (p.formula.as_deref(), p.provenance, p.confidence),
        (Some("CO"), "mass", "medium")
    );
    assert!(p.ppm_error.abs() < 1.0);
}

#[test]
fn the_json_key_is_deterministic_and_leaves_the_graph_untouched() {
    let (g, j) = shifted_graph(&con(Some("C8H10N4O2+H")), &[0, 1, 2, 3], false, 0.0);
    let a = with_interpretation(&j, &g, PPM);
    let (g2, j2) = shifted_graph(&con(Some("C8H10N4O2+H")), &[3, 2, 1, 0], false, 0.0);
    assert_eq!(a, with_interpretation(&j2, &g2, PPM));
    assert!(a.starts_with(j.split(",\"vista\":").next().unwrap()));
    assert!(a.ends_with(&j[j.find(",\"vista\":").unwrap()..]));
    assert!(a.contains(",\"interpretazione\":{\"versione\":1,\"ppm\":10,\"archi\":["));
    assert!(a.contains("\"nome\":\"carbon monoxide\""));
}
