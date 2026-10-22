use super::posa::{albero, baricentro_pos, incroci, Rete};
use super::posa_radiale::radiale;
use super::tests::*;
use super::vista::{leggi, percorsi, ArcoD, Dati};
use super::*;
use crate::json::{parse, Value};

fn con(formula: Option<&str>) -> Opzioni {
    Opzioni {
        formula: formula.map(String::from),
        ..Opzioni::default()
    }
}

fn sintetico_json(ordine: &[usize]) -> String {
    grafo_spostato(&con(Some("C8H10N4O2+H")), ordine, false, 0.0).1
}

fn vista_di(j: &str) -> Value {
    parse(j.as_bytes()).unwrap().get("vista").unwrap().clone()
}

fn nodo<'a>(v: &'a Value, id: &str) -> &'a Value {
    v.get("nodi")
        .unwrap()
        .arr()
        .iter()
        .find(|n| n.get("id").and_then(|x| x.str()) == Some(id))
        .unwrap()
}

#[test]
fn the_view_says_levels_intensities_and_the_best_path_of_each_ion() {
    let v = vista_di(&sintetico_json(&[0, 1, 2, 3]));
    assert_eq!(
        v.get("algoritmo").and_then(|x| x.str()),
        Some("best-path/1")
    );
    let n = |id: &str, k: &str| nodo(&v, id).get(k).unwrap().clone();
    // the root: MS1 only, no parent
    assert_eq!(n("n0", "livelli").arr().len(), 1);
    assert!(nodo(&v, "n0")
        .get("best")
        .unwrap()
        .get("da")
        .unwrap()
        .is_null());
    // n1 is seen in the MS2 of the root and in the MS3 of n3: two levels, two precursors, the nearest path wins
    let n1 = nodo(&v, "n1");
    let l: Vec<f64> = n1
        .get("livelli")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|x| x.f64())
        .collect();
    assert_eq!(l, [2.0, 3.0]);
    assert_eq!(n1.get("entranti").and_then(|x| x.f64()), Some(2.0));
    assert_eq!(n1.get("comune").and_then(|x| x.bool()), Some(true));
    assert_eq!(
        n1.get("best").unwrap().get("da").and_then(|x| x.str()),
        Some("n0")
    );
    // rel = the most intense measure on the arc of its best path (the same number as `misure[].rel`)
    let g = parse(sintetico_json(&[0, 1, 2, 3]).as_bytes()).unwrap();
    let arco = g
        .get("archi")
        .unwrap()
        .arr()
        .iter()
        .find(|a| {
            a.get("da").and_then(|x| x.str()) == Some("n0")
                && a.get("a").and_then(|x| x.str()) == Some("n1")
        })
        .unwrap();
    let rel = arco
        .get("misure")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|m| m.get("rel")?.f64())
        .fold(0.0, f64::max);
    assert_eq!(n1.get("rel").and_then(|x| x.f64()), Some(rel));
}

#[test]
fn the_same_inputs_give_the_same_bytes_and_the_view_completes_an_old_graph() {
    let a = sintetico_json(&[0, 1, 2, 3]);
    assert_eq!(a, sintetico_json(&[0, 1, 2, 3]));
    assert_eq!(a, sintetico_json(&[3, 2, 1, 0]));
    assert_eq!(a, sintetico_json(&[2, 0, 3, 1]));
    let senza = &a[..a.find(",\"vista\":").unwrap()];
    let senza = format!("{senza}}}");
    assert_eq!(completa(&senza).unwrap(), a);
    // a graph that has the view is returned as it is; a keyed error for what is not an msngrafo/1
    assert_eq!(completa(&a).unwrap(), a);
    for brutto in [
        "",
        "[]",
        "{\"formato\":\"msntree/1\"}",
        "{\"formato\":\"msngrafo/1\"}",
    ] {
        assert_eq!(
            completa(brutto).err().unwrap().key,
            "err.msngrafo.badGraph",
            "{brutto}"
        );
    }
}

#[test]
fn keys_added_later_stay_before_the_view() {
    let o = con(Some("C8H10N4O2+H"));
    let (g, j) = grafo_spostato(&o, &[0, 1, 2, 3], false, 0.0);
    let k = con_interpretazione(&j, &g, PPM_PERDITA);
    assert!(k.ends_with(&j[j.find(",\"vista\":").unwrap()..]));
    assert_eq!(completa(&k).unwrap(), k);
}

fn arco(da: usize, a: usize, rel: f64, isolato: bool) -> ArcoD {
    ArcoD {
        da,
        a,
        isolato,
        rel,
        livelli: vec![2],
    }
}

fn rete(n: usize, archi: Vec<ArcoD>) -> Dati {
    Dati {
        id: (0..n).map(|i| format!("n{i}")).collect(),
        mz: (0..n).map(|i| 500.0 - i as f64).collect(),
        livello: vec![1; n],
        rel_max: vec![100.0; n],
        archi,
    }
}

#[test]
fn the_best_path_takes_the_fewest_steps_then_the_strongest_weakest_arc_then_isolation_then_the_smaller_id(
) {
    // n0 -> n1, n0 -> n2, n0 -> n3 (n3 also reached in two steps), n4 from n1 and n2 and n3
    let d = rete(
        6,
        vec![
            arco(0, 1, 50.0, false),
            arco(0, 2, 80.0, false),
            arco(0, 3, 80.0, false),
            arco(1, 4, 90.0, false),
            arco(2, 4, 60.0, false),
            arco(3, 4, 60.0, true),
            arco(0, 5, 10.0, false),
            arco(3, 5, 99.0, false),
        ],
    );
    let b = percorsi(&d);
    // n5: one step from n0 beats two steps even when the second arc is far stronger
    assert_eq!((b[5].da, b[5].passi), (Some(0), 1));
    // n4: all three paths have two steps; the smallest intensities are 50 (via n1), 60 (via n2), 60 (via n3): n2 and n3 tie, n3 is isolated
    assert_eq!((b[4].da, b[4].passi), (Some(3), 2));
    // without the isolation the smaller id wins
    let mut d2 = rete(
        5,
        vec![
            arco(0, 1, 80.0, false),
            arco(0, 2, 80.0, false),
            arco(1, 4, 60.0, false),
            arco(2, 4, 60.0, false),
            arco(0, 3, 1.0, false),
        ],
    );
    assert_eq!(percorsi(&d2)[4].da, Some(1));
    d2.archi[2].rel = 59.0;
    assert_eq!(percorsi(&d2)[4].da, Some(2));
    assert_eq!(b[0].da, None);
}

/// How many nodes would change parent if each took its single most intense entering arc (the arborescence of least cost of Chu-Liu/Edmonds
/// on a DAG degenerates to this local choice): the number that tells how much the rule of the best path changes the picture.
fn cambiano_genitore(d: &Dati) -> usize {
    let b = percorsi(d);
    (0..d.id.len())
        .filter(|&v| {
            let mut ent: Vec<&ArcoD> = d.archi.iter().filter(|a| a.a == v).collect();
            crate::sortc::any_by(&mut ent, |x, y| {
                y.rel.total_cmp(&x.rel).then(x.da.cmp(&y.da))
            });
            ent.first().map(|a| a.da) != b[v].da
        })
        .count()
}

#[test]
fn how_many_nodes_the_local_best_arc_would_move_in_the_synthetic_infusion() {
    let j = sintetico_json(&[0, 1, 2, 3]);
    let d = leggi(&parse(j.as_bytes()).unwrap()).unwrap();
    assert_eq!(cambiano_genitore(&d), 2);
}

/// A layered graph of `n` nodes per level made with a fixed generator: every node has a precursor in the level above and one in five a second one.
fn a_strati(livelli: usize, per_livello: usize) -> (Vec<u8>, Vec<f64>, Vec<(usize, usize)>) {
    let mut s = 12345u64;
    let mut next = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (s >> 33) as usize
    };
    let (mut lv, mut mz, mut archi) = (vec![], vec![], vec![]);
    for l in 0..livelli {
        for k in 0..per_livello {
            lv.push(l as u8 + 1);
            mz.push(400.0 - l as f64 * 40.0 - k as f64 * 3.0);
        }
    }
    for l in 0..livelli - 1 {
        for k in 0..per_livello {
            // one precursor in the level above, and a second one for one ion in five (the shared fragments)
            let da = l * per_livello + next() % per_livello;
            let a = (l + 1) * per_livello + k;
            archi.push((da, a));
            if next() % 5 == 0 {
                let altro = l * per_livello + next() % per_livello;
                if altro != da {
                    archi.push((altro, a));
                }
            }
        }
    }
    (lv, mz, archi)
}

#[test]
fn the_swaps_remove_crossings_and_no_two_nodes_share_a_place() {
    for (livelli, per) in [(5, 6), (8, 10)] {
        let (livello, mz, archi) = a_strati(livelli, per);
        let r = Rete {
            livello: &livello,
            mz: &mz,
            archi: &archi,
        };
        let (prima, dopo) = (baricentro_pos(&r), albero(&r));
        let (a, b) = (incroci(&r, &prima), incroci(&r, &dopo));
        println!("{} nodes: crossings {a} -> {b}", livello.len());
        assert!(b <= a / 2 + 1, "{a} -> {b}");
        let mut p = dopo.clone();
        p.sort();
        p.dedup();
        assert_eq!(p.len(), livello.len());
        assert_eq!(albero(&r), dopo);
    }
}

#[test]
fn the_radial_angles_follow_the_leaves_of_the_best_path_tree() {
    // 0 -> 1 (three leaves 3, 4, 5), 0 -> 2 (one leaf): the sectors are 270 and 90 degrees
    let padre = [None, Some(0), Some(0), Some(1), Some(1), Some(1)];
    let passi = [0, 1, 1, 2, 2, 2];
    let mz = [500.0, 400.0, 300.0, 390.0, 380.0, 370.0];
    let r = radiale(&padre, &passi, &mz);
    assert_eq!(r[0], (0, 0.0));
    assert_eq!(r[1], (1, 135.0));
    assert_eq!(r[2], (1, 315.0));
    assert_eq!([r[3].1, r[4].1, r[5].1], [45.0, 135.0, 225.0]);
    assert!(r.iter().all(|x| (0.0..360.0).contains(&x.1)));
}

#[test]
fn the_usual_name_of_a_loss_is_written_not_hill_order() {
    use crate::elements::parse_formula;
    let e = candidati::elementi(&parse_formula("C20H30ClFN3O6PS").unwrap(), 1);
    for (d, nome) in [
        ("H3N", "NH3"),
        ("CHN", "HCN"),
        ("CHNO", "HNCO"),
        ("FH", "HF"),
        ("ClH", "HCl"),
        ("H3O4P", "H3PO4"),
        ("H2O", "H2O"),
        ("C4H8", "C4H8"),
        ("O2S", "SO2"),
    ] {
        let c = e.conta(&parse_formula(d).unwrap()).unwrap();
        assert_eq!(e.testo_perdita(&c), nome, "{d}");
    }
}
