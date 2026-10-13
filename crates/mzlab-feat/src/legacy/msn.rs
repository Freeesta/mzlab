//! The MSn tree of the parent from a direct-infusion MSn file (`TP_Mine/py/tpmine/hr/msn.py::build_tree`): nodes = fragmentation paths of the
//! filter string, consensus spectra, exact precursor masses from the parent node, formulas under the hierarchical sub-formula rule, atom sets
//! constrained by the tree; then the fragment library (`hr/iimn.py::build_library`) and the localisation of a product (`hr/localize.py`).

use std::collections::HashMap;

use mzlab_core::elements::{mass_of, ELECTRON};
use mzlab_core::idxread::{Index, ReadAt};

use super::fragtree::{elements_of, fmt, substructures, vec_of, Graph, Space, Subs};
use super::ms2::consensus_arrays;
use super::Res;
use mzlab_core::msnalbero::{albero_di, Raggruppa};

mod atoms;
pub mod json;
pub mod library;
pub mod localize;
pub mod space;

use space::{candidates, count, Calibration};

/// A node with fewer clean peaks is a ghost.
const MIN_PEAKS: usize = 3;
/// Peaks above precursor + HEAVY are not products of it.
const HEAVY: f64 = 1.5;
const PREC_PPM: f64 = 5.0;
const FRAG_PPM: f64 = 5.0;
const FRAG_ABS: f64 = 0.002;

/// One scan of the MSn file: its level, the path of its filter string (m/z, activation, energy per stage) and its peaks.
#[derive(Debug, Clone, Default)]
pub struct MsnScan {
    pub level: u8,
    pub path: Option<Vec<(f64, String, f64)>>,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

/// (nominal m/z, activation, collision energy as `f64::to_bits`, so that a key can be hashed) per stage.
pub type Key = Vec<(i64, String, u64)>;

/// A clean peak of a node.
#[derive(Debug, Clone)]
pub struct Peak {
    pub mz: f64,
    pub mz_raw: f64,
    pub rel: f64,
    pub formula: String,
    pub ppm: f64,
    pub n_formulas: usize,
    pub n_scans: usize,
    pub n_atom_candidates: Option<usize>,
}

/// One node: the keys of `MsnTree.summary()`.
#[derive(Debug, Clone, Default)]
pub struct Node {
    pub id: String,
    pub path: Key,
    pub level: usize,
    pub ce: Option<f64>,
    pub n_scans: usize,
    pub prec_mz: Option<f64>,
    pub formula: Option<String>,
    pub ppm: Option<f64>,
    pub ghost: bool,
    pub parent: Option<String>,
    pub via: usize,
    pub peaks: Vec<Peak>,
    /// (n, certain atoms, possible atoms).
    pub atom_candidates: Option<(usize, Vec<usize>, Vec<usize>)>,
    /// heavy, not_subformula, no_formula (absent on the root).
    pub removed: Option<[usize; 3]>,
    pub note: Option<&'static str>,
    pub n_formulas: Option<usize>,
    pub base_peak_removed: Option<bool>,
}

/// Candidate substructures (indices into `Subs`) and their weights.
pub type Cand = (Vec<usize>, Vec<f64>);
/// The consensus of a node: m/z, relative intensity, number of scans of every peak.
type Cons = (Vec<f64>, Vec<f64>, Vec<usize>);

/// `MsnTree`.
pub struct MsnTree {
    pub nodes: Vec<Node>,
    pub els: Vec<&'static str>,
    pub space: Space,
    pub cal: Calibration,
    pub subs: Option<Subs>,
    pub cand: HashMap<String, Cand>,
    pub peak_cand: HashMap<(String, usize), Cand>,
}

/// What a node leaves to its children: its formula, precursor and calibrated spectrum.
struct Info {
    formula: Option<Vec<i64>>,
    mz: Vec<f64>,
    rel: Vec<f64>,
}

/// First index of the largest value (`np.argmax`).
fn argmax(v: &[f64], idx: &[usize]) -> usize {
    let mut best = idx[0];
    for &i in idx {
        if v[i] > v[best] {
            best = i;
        }
    }
    best
}

/// `node_key` of a path of the tree (`mzlab_core::msnalbero`): nominal m/z, activation, collision energy (as bits, to be hashed).
fn key_of(chiave: &[(f64, String, f64)]) -> Key {
    chiave
        .iter()
        .map(|(mz, act, ce)| (mz.round_ties_even() as i64, act.clone(), ce.to_bits()))
        .collect()
}

/// m/z of the singly charged cation with these counts.
fn ion_mass(v: &[i64], els: &[&str]) -> f64 {
    v.iter()
        .zip(els)
        .map(|(n, e)| *n as f64 * mass_of(e).unwrap_or(f64::NAN))
        .fold(0.0, |a, b| a + b)
        - ELECTRON
}

/// The calibration from the strong peaks of every node (and the root ion in the MS1).
fn calibration(
    keys: &[Key],
    cons: &HashMap<Key, Cons>,
    sp: &Space,
    top: &[i64],
    ion_mz: f64,
    mode: &str,
) -> (Calibration, Option<f64>) {
    let mut all = Vec::new();
    for k in keys.iter().filter(|k| !k.is_empty()) {
        let (mz, rel, _) = &cons[k];
        let mx = rel.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        all.extend(
            mz.iter()
                .zip(rel)
                .filter(|(_, &r)| r >= 0.05 * mx)
                .map(|(&m, _)| m),
        );
    }
    let mut root_obs = None;
    if let Some((mz, rel, _)) = cons.get(&Vec::new()) {
        let near: Vec<usize> = (0..mz.len())
            .filter(|&i| (mz[i] - ion_mz).abs() <= 0.5)
            .collect();
        if !near.is_empty() {
            let m = mz[argmax(rel, &near)];
            root_obs = Some(m);
            all.push(m);
        }
    }
    let cal = if mode == "none" || all.is_empty() {
        Calibration::default()
    } else {
        space::fit(&all, sp, Some(top), 10.0, mode)
    };
    (cal, root_obs)
}

/// The peaks of a node that a subformula of the precursor `fvec` explains, and the removed ones: heavier than the precursor, explained by
/// another formula only, by none.
fn sift(sp: &Space, mz: &[f64], prec: f64, fvec: &[i64]) -> (Vec<bool>, [usize; 3]) {
    let mut removed = [0usize; 3];
    let mut keep = vec![false; mz.len()];
    for (j, &m) in mz.iter().enumerate() {
        let heavy = m > prec + HEAVY;
        let n_sub = if heavy {
            0
        } else {
            count(sp, m, FRAG_PPM, FRAG_ABS, Some(fvec))
        };
        let n_any = if heavy || n_sub > 0 {
            1
        } else {
            count(sp, m, FRAG_PPM, FRAG_ABS, None)
        };
        removed[0] += usize::from(heavy);
        removed[1] += usize::from(!heavy && n_sub == 0 && n_any > 0);
        removed[2] += usize::from(!heavy && n_any == 0);
        keep[j] = !heavy && n_sub > 0;
    }
    (keep, removed)
}

/// The node of key `k` (not the root) given its parent's information.
fn grow(tree: &MsnTree, k: &Key, par: &Info, cons: &Cons, mz: &[f64]) -> (Node, Info) {
    let (mz_raw, rel, nsc) = cons;
    let mut node = Node {
        id: String::new(),
        path: k.clone(),
        level: k.len() + 1,
        ce: Some(f64::from_bits(k[k.len() - 1].2)),
        ghost: true,
        removed: Some([0; 3]),
        ..Node::default()
    };
    let mut info = Info {
        formula: None,
        mz: mz.to_vec(),
        rel: rel.clone(),
    };
    let nominal = k[k.len() - 1].0 as f64;
    let sel: Vec<usize> = (0..par.mz.len())
        .filter(|&i| (par.mz[i] - nominal).abs() <= 0.5)
        .collect();
    if sel.is_empty() {
        node.note = Some("precursor not found in the parent spectrum");
        return (node, info);
    }
    let sp = &tree.space;
    let prec = par.mz[argmax(&par.rel, &sel)];
    let cands = candidates(sp, prec, PREC_PPM, 0.0, par.formula.as_deref());
    node.prec_mz = Some(prec);
    let Some(&c0) = cands.first() else {
        node.note = Some("no formula for the precursor (subformula of the parent)");
        return (node, info);
    };
    let fvec = sp.g[c0].clone();
    node.formula = Some(fmt(&fvec, &tree.els));
    node.ppm = Some((prec - sp.mass[c0]) / sp.mass[c0] * 1e6);
    node.n_formulas = Some(cands.len());
    let (keep, removed) = sift(sp, mz, prec, &fvec);
    node.removed = Some(removed);
    let all: Vec<usize> = (0..rel.len()).collect();
    let base_removed = !rel.is_empty() && !keep[argmax(rel, &all)];
    let kidx: Vec<usize> = (0..mz.len()).filter(|&j| keep[j]).collect();
    let top_rel = if kidx.is_empty() {
        1.0
    } else {
        kidx.iter()
            .map(|&j| rel[j])
            .fold(f64::NEG_INFINITY, f64::max)
    };
    for &j in &kidx {
        let cc = candidates(sp, mz[j], FRAG_PPM, FRAG_ABS, Some(&fvec));
        let c = cc[0];
        node.peaks.push(Peak {
            mz: mz[j],
            mz_raw: mz_raw[j],
            rel: 100.0 * rel[j] / top_rel,
            formula: fmt(&sp.g[c], &tree.els),
            ppm: (mz[j] - sp.mass[c]) / sp.mass[c] * 1e6,
            n_formulas: cc.len(),
            n_scans: nsc[j],
            n_atom_candidates: None,
        });
    }
    node.ghost = node.peaks.len() < MIN_PEAKS || base_removed;
    node.base_peak_removed = Some(base_removed);
    info.formula = Some(fvec);
    (node, info)
}

/// The root node (MS1) and what it leaves to its children.
fn root(
    tree: &MsnTree,
    top: &[i64],
    n_scans: usize,
    cons: Option<&Cons>,
    root_obs: Option<f64>,
) -> (Node, Info) {
    let ion_mz = ion_mass(top, &tree.els);
    let prec = root_obs.map_or(ion_mz, |m| tree.cal.apply(m));
    let (mz, rel) = cons.map_or((Vec::new(), Vec::new()), |c| {
        (
            c.0.iter().map(|&m| tree.cal.apply(m)).collect(),
            c.1.clone(),
        )
    });
    let node = Node {
        id: "MS1".into(),
        level: 1,
        n_scans,
        prec_mz: Some(prec),
        formula: Some(fmt(top, &tree.els)),
        ppm: Some((prec - ion_mz) / ion_mz * 1e6),
        ..Node::default()
    };
    (
        node,
        Info {
            formula: Some(top.to_vec()),
            mz,
            rel,
        },
    )
}

/// `build_tree(run, ion, smiles, calibrate=...)` on the scans of the file: `ion` = the ion formula of the parent ([M+H]+), `smiles` the neutral
/// parent (the atom sets of every node and peak); `calibrate` = "auto" | "constant" | "linear" | "none".
pub fn build_tree(
    scans: &[MsnScan],
    ion: &str,
    smiles: Option<&str>,
    calibrate: &str,
) -> Res<MsnTree> {
    let els = elements_of(ion).ok_or_else(|| format!("unknown formula: {ion}"))?;
    let top = vec_of(ion, &els).ok_or_else(|| format!("unknown formula: {ion}"))?;
    let space = Space::new(&top, &els);
    let ion_mz = ion_mass(&top, &els);
    let a = albero_di(
        scans.iter().map(|s| (s.level, s.path.as_deref())),
        &Raggruppa::default(),
    );
    let mut keys: Vec<Key> = Vec::new();
    let mut groups: HashMap<Key, Vec<usize>> = HashMap::new();
    if !a.ms1.is_empty() {
        keys.push(Vec::new());
        groups.insert(Vec::new(), a.ms1.clone());
    }
    for n in &a.nodi {
        keys.push(key_of(&n.chiave));
        groups.insert(key_of(&n.chiave), n.scansioni.clone());
    }
    let cons: HashMap<Key, Cons> = keys
        .iter()
        .map(|k| {
            let spectra: Vec<(Vec<f64>, Vec<f64>)> = groups[k]
                .iter()
                .map(|&i| (scans[i].mz.clone(), scans[i].it.clone()))
                .collect();
            (k.clone(), consensus_arrays(&spectra, 5.0, 0.5, 0.0))
        })
        .collect();
    let (cal, root_obs) = calibration(&keys, &cons, &space, &top, ion_mz, calibrate);
    let mut tree = MsnTree {
        nodes: Vec::new(),
        els,
        space,
        cal,
        subs: None,
        cand: HashMap::new(),
        peak_cand: HashMap::new(),
    };
    let mut info: HashMap<Key, Info> = HashMap::new();
    let n_root = groups.get(&Vec::new()).map_or(0, Vec::len);
    let (r, ri) = root(&tree, &top, n_root, cons.get(&Vec::new()), root_obs);
    tree.nodes.push(r);
    info.insert(Vec::new(), ri);
    for n in &a.nodi {
        let k = &key_of(&n.chiave);
        let (pk, parent) = match n.genitore {
            Some(g) => (key_of(&a.nodi[g].chiave), a.nodi[g].id.clone()),
            None => (Vec::new(), "MS1".to_string()),
        };
        let c = &cons[k];
        let mz: Vec<f64> = c.0.iter().map(|&m| tree.cal.apply(m)).collect();
        let (mut node, inf) = grow(&tree, k, &info[&pk], c, &mz);
        node.id = n.id.clone();
        node.parent = Some(parent);
        node.via = usize::from(n.salto);
        node.n_scans = n.scansioni.len();
        tree.nodes.push(node);
        info.insert(k.clone(), inf);
    }
    if let Some(s) = smiles {
        tree.subs = Graph::parse(s).and_then(substructures);
        if tree.subs.is_some() {
            atoms::atom_sets(&mut tree);
        }
    }
    Ok(tree)
}

/// The scans of an MSn file from its `.mzidx`: every section is read once, a scan that two sections share (`all` and `+`) only from the first.
pub fn scans_of<R: ReadAt>(ix: &mut Index<R>) -> Res<Vec<MsnScan>> {
    let mut out: Vec<MsnScan> = ix
        .scans
        .iter()
        .map(|s| MsnScan {
            level: s.level,
            path: s.path.clone(),
            ..MsnScan::default()
        })
        .collect();
    let mut done = vec![false; out.len()];
    for si in 0..ix.sections.len() {
        let slot: Vec<Option<usize>> = ix
            .arrays(si)
            .map_err(|e| e.to_string())?
            .scan_ids
            .iter()
            .map(|&g| {
                usize::try_from(g)
                    .ok()
                    .filter(|&g| g < done.len() && !done[g])
            })
            .collect();
        if slot.iter().all(Option::is_none) {
            continue;
        }
        let n = ix.sections[si].n_peaks;
        ix.each_peak(si, 0, n, |p, mz, it| {
            if let Some(Some(g)) = slot.get(p) {
                out[*g].mz.push(mz);
                out[*g].it.push(it);
            }
        })
        .map_err(|e| e.to_string())?;
        slot.iter().flatten().for_each(|&g| done[g] = true);
    }
    Ok(out)
}
