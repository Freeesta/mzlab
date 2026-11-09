use super::*;
use crate::elements::{mass, parse_formula, ELECTRON};
use crate::msnalbero::{albero_di, Grouping};

type Step = (f64, String, f64);
/// Paths of the scans, spectra by node id of the tree, MS1.
type Sintetico = (Vec<Vec<Step>>, Vec<(&'static str, Spettro)>, Spettro);

/// m/z of the singly charged cation of a formula.
pub(super) fn ion(f: &str) -> f64 {
    mass(&parse_formula(f).unwrap()) - ELECTRON
}

fn sp(peaks: &[(f64, f64)]) -> Spettro {
    Spettro {
        mz: peaks.iter().map(|p| p.0).collect(),
        rel: peaks.iter().map(|p| p.1).collect(),
    }
}

fn path(v: &[(f64, &str, f64)]) -> Vec<Step> {
    v.iter().map(|&(m, a, ce)| (m, a.to_string(), ce)).collect()
}

/// A caffeine-like infusion (formulas as sub-formulas of C8H11N4O2, as `msn_synth.py`): MS2 of 195 at CID 30, MS3 of 138 at CID 30 and HCD 45,
/// MS4 of 110 from the CID MS3. The spectra by node id of the tree.
pub(super) fn sintetico() -> Sintetico {
    let paths = vec![
        path(&[(195.0, "CID", 30.0)]),
        path(&[(195.0, "CID", 30.0), (138.0, "CID", 30.0)]),
        path(&[(195.0, "CID", 30.0), (138.0, "HCD", 45.0)]),
        path(&[
            (195.0, "CID", 30.0),
            (138.0, "CID", 30.0),
            (110.0, "CID", 35.0),
        ]),
    ];
    let ms1 = sp(&[(ion("C8H11N4O2"), 100.0), (301.0, 3.0)]);
    let ms2 = sp(&[
        (ion("C8H11N4O2"), 20.0),
        (ion("C6H8N3O"), 100.0),
        (ion("C6H8N3O") + 1.003355, 7.0),
        (ion("C5H8N3"), 12.0),
        (ion("C5H6N3O"), 8.0),
        (ion("C6H10N3O2"), 1.5),
    ]);
    let ms3 = sp(&[
        (ion("C6H8N3O"), 40.0),
        (ion("C5H8N3"), 100.0),
        (ion("C5H5N2O"), 14.0),
        (ion("C4H7N2"), 9.0),
        (173.497, 20.0),
    ]);
    let ms3h = sp(&[(ion("C5H8N3"), 100.0), (ion("C4H7N2"), 20.0)]);
    let ms4 = sp(&[
        (ion("C5H8N3"), 100.0),
        (ion("C4H7N2"), 21.0),
        (ion("C3H5N2"), 6.0),
        (ion("C2H4N"), 11.0),
    ]);
    let spettri = vec![
        ("195@cid30", ms2),
        ("195@cid30>138@cid30", ms3),
        ("195@cid30>138@hcd45", ms3h),
        ("195@cid30>138@cid30>110@cid35", ms4),
    ];
    (paths, spettri, ms1)
}

/// The graph of the synthetic infusion, its scans given in `ordine` (indices of the paths, any order; 3 scans each).
fn graph(o: &Options, order: &[usize], inverti: bool) -> (Graph, String) {
    shifted_graph(o, order, inverti, 0.0)
}

/// The same, every m/z of the spectra off by `ppm` (an uncalibrated instrument).
pub(super) fn shifted_graph(
    o: &Options,
    order: &[usize],
    inverti: bool,
    ppm: f64,
) -> (Graph, String) {
    graph_with(o, order, inverti, ppm, &Strumento::default())
}

/// How the synthetic infusion was acquired: the isolation window of every node, the polarity (in negative the anions of the same
/// formulas, 2 electrons heavier).
#[derive(Default)]
pub(super) struct Strumento {
    pub width: Option<f64>,
    pub negativo: bool,
}

pub(super) fn graph_with(
    o: &Options,
    order: &[usize],
    inverti: bool,
    ppm: f64,
    st: &Strumento,
) -> (Graph, String) {
    let (paths, mut spettri, mut ms1) = sintetico();
    let e = if st.negativo { 2.0 * ELECTRON } else { 0.0 };
    for s in spettri.iter_mut().map(|x| &mut x.1).chain([&mut ms1]) {
        s.mz.iter_mut()
            .for_each(|m| *m = (*m + e) * (1.0 + ppm * 1e-6));
    }
    let mut scans: Vec<(u8, Option<&[Step]>)> = vec![(1, None)];
    for &i in order {
        for _ in 0..3 {
            scans.push(((paths[i].len() + 1) as u8, Some(&paths[i][..])));
        }
    }
    let al = albero_di(scans, &Grouping::default());
    let per_node: Vec<Spettro> = al
        .nodi
        .iter()
        .map(|n| {
            let mut s = spettri
                .iter()
                .find(|(id, _)| *id == n.id)
                .unwrap()
                .1
                .clone();
            if inverti {
                s.mz.reverse();
                s.rel.reverse();
            }
            s
        })
        .collect();
    let g = build(
        &Input {
            albero: &al,
            ms1: Some(&ms1),
            spettri: &per_node,
            widths: &vec![st.width; al.nodi.len()],
            polarita: if st.negativo { -1 } else { 1 },
        },
        o,
    );
    let j = json(&g, o, None);
    (g, j)
}

pub(super) fn find(g: &Graph, f: &str) -> usize {
    let m = ion(f);
    g.nodi
        .iter()
        .position(|n| (n.mz - m).abs() < 1e-4)
        .unwrap_or_else(|| panic!("no node for {f}"))
}

pub(super) fn edge<'a>(g: &'a Graph, da: &str, a: &str) -> Option<&'a Edge> {
    let (p, f) = (find(g, da), find(g, a));
    g.archi.iter().find(|x| x.da == p && x.a == f)
}

#[test]
fn nodes_levels_and_counts() {
    let (g, _) = graph(&Options::default(), &[0, 1, 2, 3], false);
    let levels: Vec<(&str, u8)> = vec![
        ("C8H11N4O2", 1),
        ("C6H8N3O", 2),
        ("C5H8N3", 2),
        ("C5H6N3O", 2),
        ("C5H5N2O", 3),
        ("C4H7N2", 3),
        ("C3H5N2", 4),
        ("C2H4N", 4),
    ];
    assert_eq!(g.nodi.len(), levels.len());
    for (f, l) in &levels {
        assert_eq!(g.nodi[find(&g, f)].livello, *l, "{f}");
    }
    assert_eq!(g.radici, vec![find(&g, "C8H11N4O2")]);
    assert_eq!(g.scartati.pesanti, 1);
    assert_eq!(g.scartati.isotopi, 1);
    assert_eq!(g.scartati.deboli, 1);
    assert!(g.avvisi.is_empty(), "{:?}", g.avvisi);
    let n138 = &g.nodi[find(&g, "C6H8N3O")];
    assert_eq!(
        n138.albero,
        vec!["195@cid30>138@cid30", "195@cid30>138@hcd45"]
    );
    assert!(g.nodi[find(&g, "C5H5N2O")].albero.is_empty());
}

#[test]
fn arcs_losses_measures_and_dag() {
    let (g, _) = graph(&Options::default(), &[0, 1, 2, 3], false);
    let a = edge(&g, "C6H8N3O", "C5H8N3").unwrap();
    assert_eq!(a.compatibili, vec!["CO"]);
    assert!(a.isolamento);
    let at: Vec<(&str, f64)> = a.misure.iter().map(|m| (m.att.as_str(), m.ce)).collect();
    assert_eq!(at, vec![("CID", 30.0), ("HCD", 45.0)]);
    assert!(edge(&g, "C8H11N4O2", "C6H8N3O").unwrap().isolamento);
    assert!(!edge(&g, "C8H11N4O2", "C5H6N3O").unwrap().isolamento);
    // the same ion from two precursors: a DAG
    assert!(edge(&g, "C6H8N3O", "C4H7N2").is_some() && edge(&g, "C5H8N3", "C4H7N2").is_some());
    assert_eq!(
        edge(&g, "C5H8N3", "C4H7N2").unwrap().compatibili,
        vec!["HCN"]
    );
    for x in &g.archi {
        assert!(g.nodi[x.a].mz < g.nodi[x.da].mz);
    }
}

#[test]
fn same_bytes_whatever_the_order_of_the_input() {
    let (_, a) = graph(&Options::default(), &[0, 1, 2, 3], false);
    let (_, b) = graph(&Options::default(), &[3, 2, 1, 0], true);
    assert_eq!(a, b);
    let v = crate::json::parse(a.as_bytes()).unwrap();
    assert_eq!(v.get("formato").unwrap().str(), Some("msngrafo/2"));
    assert_eq!(v.get("nodi").unwrap().arr().len(), 8);
}

#[test]
fn isotopes_kept_on_request() {
    let o = Options {
        isotopi: true,
        ..Options::default()
    };
    let (g, _) = graph(&o, &[0, 1, 2, 3], false);
    assert_eq!(g.scartati.isotopi, 0);
    let m = ion("C6H8N3O") + 1.003355;
    let i = g.nodi.iter().position(|n| (n.mz - m).abs() < 1e-4).unwrap();
    assert_eq!(g.nodi[i].isotope_of, Some(find(&g, "C6H8N3O")));
}

#[test]
fn one_activation_only() {
    let o = Options {
        activation: Some("hcd".into()),
        ..Options::default()
    };
    let (g, _) = graph(&o, &[0, 1, 2, 3], false);
    let a = edge(&g, "C6H8N3O", "C5H8N3").unwrap();
    assert_eq!(a.misure.len(), 1);
    assert_eq!(a.misure[0].att, "HCD");
    // a fragment seen only in the CID spectra is not in the HCD graph
    assert!(!g.nodi.iter().any(|n| (n.mz - ion("C5H5N2O")).abs() < 1e-4));
}

#[test]
fn the_drawing_is_one_column_per_level() {
    let (g, _) = graph(&Options::default(), &[0, 1, 2, 3], false);
    let mut seen = std::collections::BTreeSet::new();
    for n in &g.nodi {
        assert_eq!(n.pos.0, n.livello as usize - 1);
        assert!(seen.insert(n.pos));
    }
}

#[test]
fn a_chain_of_close_peaks_is_split() {
    // four MS2 energies of a 1500 ion: a fragment drifting 4 mDa per energy (tolerance 5 mDa at m/z 1000)
    let paths: Vec<Vec<Step>> = (0..4)
        .map(|k| path(&[(1500.0, "CID", 20.0 + k as f64)]))
        .collect();
    let scans: Vec<(u8, Option<&[Step]>)> = paths.iter().map(|p| (2, Some(&p[..]))).collect();
    let al = albero_di(scans, &Grouping::default());
    let spettri: Vec<Spettro> = (0..4)
        .map(|k| sp(&[(1000.0 + 0.004 * k as f64, 100.0)]))
        .collect();
    let ms1 = sp(&[(1500.0, 100.0)]);
    let g = build(
        &Input {
            albero: &al,
            ms1: Some(&ms1),
            spettri: &spettri,
            widths: &[],
            polarita: 1,
        },
        &Options::default(),
    );
    let near: Vec<f64> = g
        .nodi
        .iter()
        .map(|n| n.mz)
        .filter(|m| (m - 1000.0).abs() < 0.1)
        .collect();
    assert_eq!(near.len(), 2, "{near:?}");
}

#[test]
fn a_precursor_missing_from_its_parent_is_noted() {
    let paths = [
        path(&[(195.0, "CID", 30.0)]),
        path(&[(195.0, "CID", 30.0), (156.0, "CID", 30.0)]),
    ];
    let scans: Vec<(u8, Option<&[Step]>)> = paths
        .iter()
        .map(|p| ((p.len() + 1) as u8, Some(&p[..])))
        .collect();
    let al = albero_di(scans, &Grouping::default());
    let spettri = vec![
        sp(&[(ion("C6H8N3O"), 100.0)]),
        sp(&[(ion("C5H8N3"), 100.0)]),
    ];
    let ms1 = sp(&[(ion("C8H11N4O2"), 100.0), (ion("C8H11N4O2") + 0.3, 50.0)]);
    let g = build(
        &Input {
            albero: &al,
            ms1: Some(&ms1),
            spettri: &spettri,
            widths: &[],
            polarita: 1,
        },
        &Options::default(),
    );
    let keys: Vec<(&str, &str)> = g
        .avvisi
        .iter()
        .map(|a| (a.chiave, a.albero.as_str()))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("warn.msngrafo.isolation", "195@cid30"),
            ("warn.msngrafo.precNotFound", "195@cid30>156@cid30")
        ]
    );
    let n156 = g.nodi.iter().position(|n| n.mz == 156.0).unwrap();
    let a = g.archi.iter().find(|x| x.a == n156).unwrap();
    assert!(a.isolamento && a.misure[0].rel == 0.0);
}

#[test]
fn without_the_ion_formula_the_root_gets_candidates_and_with_it_none() {
    let (g, json) = graph(&Options::default(), &[0, 1, 2, 3], false);
    assert!(g.ione.is_none() && !g.candidati_radice.is_empty() && g.candidati_radice.len() <= 8);
    let c = g
        .candidati_radice
        .iter()
        .find(|c| c.formula == "C8H11N4O2+");
    assert!(
        c.is_some_and(|c| c.ppm.abs() < 5.0),
        "{:?}",
        g.candidati_radice
    );
    assert!(
        json.contains("\"candidati_radice\":[{\"formula\":"),
        "{json}"
    );
    let o = Options {
        formula: Some("C8H11N4O2+".into()),
        ..Options::default()
    };
    let (g, json) = graph(&o, &[0, 1, 2, 3], false);
    assert!(g.candidati_radice.is_empty() && json.contains("\"candidati_radice\":[]"));
    let (g, _) = graph_with(
        &Options::default(),
        &[0, 1, 2, 3],
        false,
        0.0,
        &Strumento {
            negativo: true,
            ..Strumento::default()
        },
    );
    assert!(g.candidati_radice.iter().all(|c| c.formula.ends_with('-')));
}
