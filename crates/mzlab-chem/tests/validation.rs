//! Validation of the reactivity engine against the experimental sites of `tests/data/reattivita/validazione.json` (task C33):
//! runs `reactivity_json` on every molecule and counts how often the experimental site is in the top 1, 3 and 5 of the predicted
//! ranking, per reaction type and tier. The numbers are a starting point, not a verdict; the baseline is locked in the same file.

use mzlab_chem::reatt::reactivity_json;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::symmetry_classes;
use serde_json::Value;
use std::collections::BTreeMap;

fn data() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/reattivita/validazione.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// What the engine says about one molecule.
struct Pred {
    /// Symmetry-class groups of heavy atoms, best mean f(−) first.
    atom_groups: Vec<Vec<usize>>,
    /// C–H sites as published in `ch_sites` (heavy atoms of each class, in rank order).
    ch_groups: Vec<Vec<usize>>,
}

fn predict(smiles: &str, opts: &str) -> Pred {
    let json: Value = serde_json::from_str(&reactivity_json(smiles, opts).unwrap()).unwrap();
    assert_eq!(json["method"], "GFN2-xTB", "{smiles}");
    assert!(
        json["scc"]["converged"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c == true),
        "{smiles}"
    );
    let mol = Mol::parse(smiles).unwrap();
    let cls = symmetry_classes(&mol);
    let mut groups: BTreeMap<u32, (Vec<usize>, f64)> = BTreeMap::new();
    for a in json["atoms"].as_array().unwrap() {
        let Some(h) = a["heavy_index"].as_u64() else {
            continue;
        };
        let g = groups.entry(cls[h as usize]).or_default();
        g.0.push(h as usize);
        g.1 += a["f_minus"].as_f64().unwrap();
    }
    let mut atom_groups: Vec<(Vec<usize>, f64)> = groups
        .into_values()
        .map(|(v, s)| {
            let m = s / v.len() as f64;
            (v, m)
        })
        .collect();
    atom_groups.sort_by(|a, b| b.1.total_cmp(&a.1));
    let ch_groups = json["ch_sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            s["class"]
                .as_str()
                .unwrap()
                .split(", ")
                .map(|c| c[1..].parse::<usize>().unwrap() - 1)
                .collect()
        })
        .collect();
    Pred {
        atom_groups: atom_groups.into_iter().map(|g| g.0).collect(),
        ch_groups,
    }
}

/// 1-based rank of the best group that holds one of the experimental atoms.
fn rank(groups: &[Vec<usize>], atoms: &[usize]) -> Option<usize> {
    groups
        .iter()
        .position(|g| g.iter().any(|a| atoms.contains(a)))
        .map(|p| p + 1)
}

#[derive(Default)]
struct Tally {
    n_ch: usize,
    ch: [usize; 3],
    n_atom: usize,
    atom: [usize; 3],
}

fn add(hits: &mut [usize; 3], r: Option<usize>) {
    for (h, k) in hits.iter_mut().zip([1, 3, 5]) {
        *h += r.is_some_and(|r| r <= k) as usize;
    }
}

/// Counts for every reaction/tier with the engine options `opts`.
fn measure(opts: &str) -> Value {
    let d = data();
    let mols = d["molecules"].as_object().unwrap();
    let preds: BTreeMap<&str, Pred> = mols
        .iter()
        .map(|(k, v)| (k.as_str(), predict(v.as_str().unwrap(), opts)))
        .collect();
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for row in d["rows"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let p = &preds[row["molecule"].as_str().unwrap()];
        let atoms: Vec<usize> = row["atoms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_u64().unwrap() as usize)
            .collect();
        let key = format!(
            "{}/{}",
            row["reaction"].as_str().unwrap(),
            row["tier"].as_str().unwrap()
        );
        let t = tallies.entry(key).or_default();
        let r_atom = rank(&p.atom_groups, &atoms);
        assert!(r_atom.is_some(), "{id}: site atoms are not heavy atoms");
        t.n_atom += 1;
        add(&mut t.atom, r_atom);
        let mut r_ch = None;
        if row["scope"] == "ch" {
            r_ch = rank(&p.ch_groups, &atoms);
            assert!(r_ch.is_some(), "{id}: site is not a C–H site of the engine");
            t.n_ch += 1;
            add(&mut t.ch, r_ch);
        }
        println!(
            "{id:34} ch rank {:>4} | atom rank {:>2}/{} | {}",
            r_ch.map_or("-".to_string(), |r| format!("{r}/{}", p.ch_groups.len())),
            r_atom.unwrap(),
            p.atom_groups.len(),
            row["product"].as_str().unwrap()
        );
    }
    let mut out = BTreeMap::new();
    for (k, t) in &tallies {
        println!(
            "{k:22} C-H sites n={:2} top1/3/5 {:?} | all heavy atoms n={:2} top1/3/5 {:?}",
            t.n_ch, t.ch, t.n_atom, t.atom
        );
        out.insert(
            k.clone(),
            serde_json::json!({"n_ch": t.n_ch, "ch": t.ch, "n_atom": t.n_atom, "atom": t.atom}),
        );
    }
    println!("BASELINE {}", serde_json::to_string(&out).unwrap());
    serde_json::to_value(&out).unwrap()
}

#[test]
fn experimental_sites_ranking() {
    assert_eq!(
        measure("{\"protomer\":\"none\"}"),
        data()["baseline"],
        "the measured counts differ from the baseline in validazione.json: update it if the engine changed on purpose"
    );
}

/// The same measure on the form at pH 7 (task C34): locked in `baseline_ph7`.
#[test]
fn experimental_sites_ranking_ph7() {
    assert_eq!(
        measure("{\"protomer\":\"ph7\"}"),
        data()["baseline_ph7"],
        "the counts at pH 7 differ from baseline_ph7 in validazione.json"
    );
}

#[test]
fn dataset_is_well_formed() {
    let d = data();
    let mols = d["molecules"].as_object().unwrap();
    assert!(mols.len() >= 20);
    for row in d["rows"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        assert!(mols.contains_key(row["molecule"].as_str().unwrap()), "{id}");
        assert!(
            ["ch", "atom"].contains(&row["scope"].as_str().unwrap()),
            "{id}"
        );
        assert!(
            ["OH", "O3", "metabolism"].contains(&row["reaction"].as_str().unwrap()),
            "{id}"
        );
        match row["tier"].as_str().unwrap() {
            "literatura" => assert!(row["doi"].as_str().unwrap().starts_with("10."), "{id}"),
            "bozza" => assert!(row["doi"].is_null(), "{id}"),
            t => panic!("{id}: tier {t}"),
        }
    }
}

/// Lists the heavy atoms of every molecule with their neighbours, to check the indices of the dataset by eye.
#[test]
#[ignore]
fn list_atoms() {
    for (name, s) in data()["molecules"].as_object().unwrap() {
        let mol = Mol::parse(s.as_str().unwrap()).unwrap();
        let m = mzlab_chem::Mol3d::from_mol(&mol).unwrap();
        println!("{name} {}", s.as_str().unwrap());
        for (i, hi) in m.heavy_index.iter().enumerate() {
            let Some(h) = hi else { continue };
            let nb: Vec<String> = m
                .bonds
                .iter()
                .filter_map(|b| {
                    let o = if b.0 == i {
                        b.1
                    } else if b.1 == i {
                        b.0
                    } else {
                        return None;
                    };
                    m.heavy_index[o].map(|x| format!("{x}"))
                })
                .collect();
            println!(
                "  {h:2} Z{} [{}] H{}",
                m.el[i],
                nb.join(","),
                m.bonds
                    .iter()
                    .filter(|b| (b.0 == i && m.el[b.1] == 1) || (b.1 == i && m.el[b.0] == 1))
                    .count()
            );
        }
    }
}
