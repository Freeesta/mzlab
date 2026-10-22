use super::interpreta::PPM;
use super::tests::{arco, grafo_spostato, trova};
use super::*;

fn con(formula: Option<&str>) -> Opzioni {
    Opzioni {
        formula: formula.map(String::from),
        ..Opzioni::default()
    }
}

fn dell_arco<'a>(g: &Grafo, v: &'a [InterpArco], da: &str, a: &str) -> &'a InterpArco {
    let (p, f) = (trova(g, da), trova(g, a));
    v.iter().find(|x| x.da == p && x.a == f).expect("arc")
}

#[test]
fn the_table_has_unique_formulas_and_exact_masses() {
    let t = perdite_note();
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
    let (g, _) = grafo_spostato(&con(Some("C8H10N4O2+H")), &[0, 1, 2, 3], false, 0.0);
    let v = interpreta(&g, PPM);
    assert_eq!(v.len(), g.archi.len());
    let co = dell_arco(&g, &v, "C6H8N3O", "C5H8N3");
    let p = co.perdita.as_ref().unwrap();
    assert_eq!(
        (p.nome, p.fonte, p.confidenza),
        (Some("carbon monoxide"), "formula", "high")
    );
    assert_eq!(p.formula.as_deref(), Some("CO"));
    assert!(p.errore_ppm.abs() < 0.1);
    assert_eq!(co.attivazioni, ["CID", "HCD"]);
    let lista = dell_arco(&g, &v, "C8H11N4O2", "C6H8N3O");
    let q = lista.perdita.as_ref().unwrap();
    assert_eq!((q.nome, q.confidenza), (None, "low"));
    assert_eq!(q.formula.as_deref(), Some("C2H3NO"));
    assert!(arco(&g, "C8H11N4O2", "C6H8N3O").is_some());
}

#[test]
fn without_formulas_the_loss_is_matched_by_mass_only() {
    let (g, _) = grafo_spostato(&con(None), &[0, 1, 2, 3], false, 0.0);
    let v = interpreta(&g, PPM);
    let p = dell_arco(&g, &v, "C6H8N3O", "C5H8N3")
        .perdita
        .clone()
        .unwrap();
    assert_eq!(
        (p.formula.as_deref(), p.fonte, p.confidenza),
        (Some("CO"), "mass", "medium")
    );
    assert!(p.errore_ppm.abs() < 1.0);
}

#[test]
fn the_json_key_is_deterministic_and_leaves_the_graph_untouched() {
    let (g, j) = grafo_spostato(&con(Some("C8H10N4O2+H")), &[0, 1, 2, 3], false, 0.0);
    let a = con_interpretazione(&j, &g, PPM);
    let (g2, j2) = grafo_spostato(&con(Some("C8H10N4O2+H")), &[3, 2, 1, 0], false, 0.0);
    assert_eq!(a, con_interpretazione(&j2, &g2, PPM));
    assert!(a.starts_with(j.split(",\"vista\":").next().unwrap()));
    assert!(a.ends_with(&j[j.find(",\"vista\":").unwrap()..]));
    assert!(a.contains(",\"interpretazione\":{\"versione\":1,\"ppm\":10,\"archi\":["));
    assert!(a.contains("\"nome\":\"carbon monoxide\""));
}
