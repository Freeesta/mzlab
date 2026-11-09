//! First C–H site of f(−) (mean over symmetry classes, ties within 0.005) on the 10 molecules of the CC-05 draft: the same class as
//! tblite 0.7.0 on the reference geometry, the experimental site in at least 9 of 10, and the same class on the ETKDG geometry of `embed`.

use mzlab_chem::fukui::{ch_sites, Site};
use mzlab_chem::reatt::gfn2;
use mzlab_chem::{embed, Mol3d, DEFAULT_CONFORMERS};
use mzlab_feat::chem::grafo::Mol;

/// (name in `cc_ref.json`, heavy atoms of the experimental site of •OH attack, draft CC-05 §2.1).
const DRAFT: [(&str, &[usize]); 10] = [
    ("phenol", &[2, 4, 6]),
    ("anisole", &[3, 5, 7]),
    ("toluene", &[2, 4, 6]),
    ("benzoic_acid", &[5, 7, 4, 8]),
    ("nitrobenzene", &[5, 7]),
    ("chlorobenzene", &[2, 4, 6]),
    ("aniline", &[2, 4, 6]),
    ("naphthalene", &[2, 4, 7, 9]),
    ("carbamazepine", &[10, 11]),
    ("paracetamol", &[6, 9]),
];

fn reference() -> serde_json::Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cc_ref.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn vec(v: &serde_json::Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn heavy(s: &Site) -> Vec<usize> {
    s.class
        .split(", ")
        .map(|c| c[1..].parse::<usize>().unwrap() - 1)
        .collect()
}

/// The classes tied with the first one.
fn first(sites: &[Site]) -> Vec<String> {
    sites
        .iter()
        .filter(|s| s.tie_first)
        .map(|s| s.class.clone())
        .collect()
}

fn sites_of(mol: &Mol, m: &Mol3d, xyz: &[[f64; 3]], water: bool) -> Vec<Site> {
    let (f, ..) = gfn2(m, xyz, water).unwrap();
    ch_sites(mol, m, &f.f_minus)
}

fn xyz(rec: &serde_json::Value) -> Vec<[f64; 3]> {
    rec["xyz"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let v = vec(p);
            [v[0], v[1], v[2]]
        })
        .collect()
}

#[test]
fn first_site_as_tblite_and_experiment() {
    let all = reference();
    let (mut same, mut exp, mut geom) = (0, 0, 0);
    for (name, site) in DRAFT {
        let rec = &all["molecules"][name];
        let mol = Mol::parse(rec["smiles"].as_str().unwrap()).unwrap();
        let m = Mol3d::from_mol(&mol).unwrap();
        let ours = sites_of(&mol, &m, &xyz(rec), true);
        let theirs = ch_sites(&mol, &m, &vec(&rec["alpb"]["f_minus"]));
        let ok_t =
            first(&theirs).contains(&ours[0].class) || first(&ours).contains(&theirs[0].class);
        let ok_e = heavy(&ours[0]).iter().any(|h| site.contains(h));
        let conf = embed(&m, 42, DEFAULT_CONFORMERS).unwrap();
        let on_etkdg = sites_of(&mol, &m, &conf[0].xyz, true);
        let ok_g =
            first(&on_etkdg).contains(&ours[0].class) || first(&ours).contains(&on_etkdg[0].class);
        println!("{name:14} ours {} {:.4} | tblite {} {:.4} | ETKDG {} {:.4} | tblite {ok_t} exp {ok_e} geom {ok_g}", ours[0].class, ours[0].f_minus, theirs[0].class, theirs[0].f_minus, on_etkdg[0].class, on_etkdg[0].f_minus);
        same += ok_t as usize;
        exp += ok_e as usize;
        geom += ok_g as usize;
    }
    println!("same first class as tblite {same}/10, experimental site {exp}/10, same class on ETKDG geometry {geom}/10");
    assert_eq!(same, 10);
    assert!(exp >= 9);
    assert_eq!(geom, 10);
}
