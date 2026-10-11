//! Tree of the fragmentation paths of an MSn file (`mzlab/chem/msntree.py`): one node per path, its consensus spectrum, the exact precursor
//! read from the parent node and the formulas (a node is a sub-formula of its parent). Candidates, never a verdict.
//! The spectra come through a callback, so the same code serves the browser, the desktop app and the tests.

use crate::composition::{compose, Opts, Ranges};
use crate::elements::MASS;
use crate::error::{Error, Result};
use crate::json::{self, fmt_g, round, Out, Value};

const MAX_SCANS: usize = 60;
const MAX_PEAKS: usize = 120;
const ROOT_ELEMENTS: [(&str, i64, i64); 5] = [
    ("C", 0, 60),
    ("H", 0, 120),
    ("N", 0, 10),
    ("O", 0, 12),
    ("S", 0, 3),
];
const ROOT_RULES: [&str; 6] = ["HC", "NC", "OC", "SC", "LEWIS", "SENIOR"];

/// What the tree needs of a scan: its place in the file, level, polarity and fragmentation path (m/z, activation, collision energy).
#[derive(Debug, Clone)]
pub struct TreeScan {
    pub index: usize,
    pub level: u8,
    pub polarity: i8,
    pub path: Vec<(f64, String, f64)>,
}

type Key = Vec<(i64, String, u64)>;

/// `([A-Z][a-z]?)(\d*)` found anywhere, counts added per element in the order of first appearance.
fn parse(f: &str) -> Ranges {
    let c: Vec<char> = f.chars().collect();
    let mut d: Vec<(String, (i64, i64))> = vec![];
    let mut i = 0;
    while i < c.len() {
        if c[i].is_ascii_uppercase() {
            let mut el = c[i].to_string();
            i += 1;
            if i < c.len() && c[i].is_ascii_lowercase() {
                el.push(c[i]);
                i += 1;
            }
            let a = i;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
            let n: i64 = if i > a {
                c[a..i].iter().collect::<String>().parse().unwrap_or(1)
            } else {
                1
            };
            match d.iter_mut().find(|(e, _)| *e == el) {
                Some(x) => x.1 .1 += n,
                None => d.push((el, (0, n))),
            }
        } else {
            i += 1;
        }
    }
    d
}

/// `re.fullmatch(r"([A-Z][a-z]?\d*)+", f)`.
fn valid(f: &str) -> bool {
    let c: Vec<char> = f.chars().collect();
    let mut i = 0;
    let mut groups = 0;
    while i < c.len() {
        if !c[i].is_ascii_uppercase() {
            return false;
        }
        i += 1;
        if i < c.len() && c[i].is_ascii_lowercase() {
            i += 1;
        }
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        groups += 1;
    }
    groups > 0
}

struct Consensus {
    mz: Vec<f64>,
    rel: Vec<f64>,
}

/// Peaks present in at least half of the scans (max-normalised per scan), merged within 5 ppm: (m/z, mean relative intensity).
fn consensus<F: FnMut(usize) -> Result<(Vec<f64>, Vec<f64>)>>(
    read: &mut F,
    idx: &[usize],
    ppm: f64,
    min_frac: f64,
) -> Result<Consensus> {
    let mut pts: Vec<(f64, f64, usize)> = vec![];
    for (k, &i) in idx.iter().enumerate() {
        let (mz, it) = read(i)?;
        let mx = it.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if mz.is_empty() || mx <= 0.0 {
            continue;
        }
        pts.extend(mz.iter().zip(&it).map(|(m, v)| (*m, v / mx, k)));
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut groups: Vec<Vec<usize>> = vec![];
    for i in 0..pts.len() {
        if i > 0 && pts[i].0 - pts[i - 1].0 <= pts[i].0 * ppm * 1e-6 {
            groups.last_mut().unwrap().push(i);
        } else {
            groups.push(vec![i]);
        }
    }
    let need = (min_frac * idx.len() as f64).max(1.0);
    let (mut mz, mut rel) = (vec![], vec![]);
    for g in groups {
        let w: f64 = g.iter().map(|&i| pts[i].1).sum();
        let cm = g.iter().map(|&i| pts[i].1 * pts[i].0).sum::<f64>() / w.max(1e-12);
        let mut sc: Vec<usize> = g.iter().map(|&i| pts[i].2).collect();
        sc.sort();
        sc.dedup();
        if sc.len() as f64 >= need {
            mz.push(cm);
            rel.push(w / idx.len() as f64);
        }
    }
    Ok(Consensus { mz, rel })
}

/// The best candidate for an exact m/z: (formula, error in ppm), or None when there is none or the request is not valid.
fn best(
    mz: f64,
    ion: &str,
    parent: Option<&str>,
    elements: &Ranges,
    ppm: f64,
    rules: &[&str],
) -> Option<(String, f64)> {
    let o = Opts {
        elements: Some(elements.clone()),
        ion,
        tol: ppm,
        rdb: (-0.5, 100.0),
        parent,
        rules: rules.iter().map(|s| s.to_string()).collect(),
        max_results: 1,
        ..Default::default()
    };
    let v = json::parse(compose(mz, &o).ok()?.as_bytes()).ok()?;
    let r = v.get("results")?.at(0)?;
    Some((
        r.get("formula")?.str()?.to_string(),
        r.get("delta_ppm")?.f64()?,
    ))
}

fn key_of(path: &[(f64, String, f64)]) -> Key {
    path.iter()
        .map(|(m, a, ce)| (m.round_ties_even() as i64, a.clone(), ce.to_bits()))
        .collect()
}

struct Node {
    parent: Option<usize>,
    path: Vec<(f64, String, f64)>,
    label: String,
    level: usize,
    scans: usize,
    prec: Option<f64>,
    formula: Option<String>,
    ppm: Option<f64>,
    n_peaks: usize,
    empty: bool,
    peaks: Vec<(f64, f64, Option<String>, Option<f64>, bool)>,
}

/// The tree as the JSON of `/api/msntree`. `scans` are all the scans of the file; `read(i)` gives the m/z and intensities of scan `i`.
pub fn msn_tree<F: FnMut(usize) -> Result<(Vec<f64>, Vec<f64>)>>(
    scans: &[TreeScan],
    mut read: F,
    formula: Option<&str>,
    ppm: f64,
) -> Result<String> {
    let formula = formula.filter(|f| !f.is_empty());
    if let Some(f) = formula {
        if !valid(f)
            || parse(f)
                .iter()
                .any(|(e, _)| !MASS.iter().any(|(m, _)| m == e))
        {
            return Err(Error::new("err.tree.formula").with("formula", f));
        }
    }
    let mut keys: Vec<Key> = vec![];
    let mut groups: Vec<Vec<usize>> = vec![];
    let mut paths: Vec<Vec<(f64, String, f64)>> = vec![];
    for s in scans.iter().filter(|s| s.level >= 2 && !s.path.is_empty()) {
        let k = key_of(&s.path);
        match keys.iter().position(|q| *q == k) {
            Some(g) => groups[g].push(s.index),
            None => {
                keys.push(k);
                groups.push(vec![s.index]);
                paths.push(s.path.clone());
            }
        }
    }
    if groups.is_empty() {
        return Err(Error::new("err.tree.noPath"));
    }
    if keys.iter().map(|k| k.len()).max().unwrap() < 2 {
        return Err(Error::new("err.tree.needStages"));
    }
    if groups.len() > 400 {
        return Err(Error::new("err.tree.tooMany").with("n", groups.len()));
    }
    let pol = scans
        .iter()
        .map(|s| s.polarity)
        .find(|p| *p != 0)
        .unwrap_or(1);
    let ion1 = if pol >= 0 { "[M]+" } else { "[M]-" };
    let ms1: Vec<usize> = scans
        .iter()
        .filter(|s| s.level == 1)
        .map(|s| s.index)
        .take(MAX_SCANS)
        .collect();
    let mut cons: Vec<Consensus> = vec![];
    for g in &groups {
        cons.push(consensus(
            &mut read,
            &g[..g.len().min(MAX_SCANS)],
            5.0,
            0.5,
        )?);
    }
    let ms1c = if ms1.is_empty() {
        None
    } else {
        Some(consensus(&mut read, &ms1, 5.0, 0.5)?)
    };
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by(|&a, &b| {
        keys[a].len().cmp(&keys[b].len()).then_with(|| {
            for (x, y) in keys[a].iter().zip(&keys[b]) {
                let c =
                    x.0.cmp(&y.0)
                        .then_with(|| x.1.cmp(&y.1))
                        .then_with(|| f64::from_bits(x.2).total_cmp(&f64::from_bits(y.2)));
                if c.is_ne() {
                    return c;
                }
            }
            std::cmp::Ordering::Equal
        })
    });
    let root_el: Ranges = ROOT_ELEMENTS
        .iter()
        .map(|(s, a, b)| (s.to_string(), (*a, *b)))
        .collect();
    let mut nodes: Vec<Node> = vec![];
    let mut pos: Vec<(usize, usize)> = vec![]; // (group, node)
    for &g in &order {
        let k = &keys[g];
        let nom = k.last().unwrap().0;
        let par = &k[..k.len() - 1];
        let mut pk = pos
            .iter()
            .find(|(q, _)| keys[*q][..] == *par)
            .map(|(q, _)| *q);
        if pk.is_none() && !par.is_empty() {
            pk = pos
                .iter()
                .find(|(q, _)| {
                    keys[*q].len() == par.len() && keys[*q].iter().zip(par).all(|(a, b)| a.0 == b.0)
                })
                .map(|(q, _)| *q);
        }
        let src = match pk {
            Some(q) => Some(&cons[q]),
            None => ms1c.as_ref(),
        };
        let mut prec: Option<f64> = None;
        if let Some(c) = src {
            let mut bi: Option<usize> = None;
            for (i, m) in c.mz.iter().enumerate() {
                if (m - nom as f64).abs() <= 0.5 && bi.is_none_or(|b| c.rel[i] > c.rel[b]) {
                    bi = Some(i);
                }
            }
            prec = bi.map(|i| c.mz[i]);
        }
        let pnode = pk.map(|q| &nodes[pos.iter().find(|(x, _)| *x == q).unwrap().1]);
        let ptxt: Option<String> = match pnode {
            Some(n) => n.formula.clone(),
            None => formula.map(String::from),
        };
        let els: Ranges = if ptxt.is_some() {
            parse(ptxt.as_deref().unwrap())
        } else {
            root_el.clone()
        };
        let parent = if pnode.is_some() {
            ptxt.as_deref()
        } else {
            formula
        };
        let bst = prec.filter(|p| *p != 0.0).and_then(|p| {
            best(
                p,
                ion1,
                parent,
                &els,
                ppm,
                if ptxt.is_some() { &[] } else { &ROOT_RULES },
            )
        });
        let form = bst.as_ref().map(|b| b.0.clone());
        let c = &cons[g];
        let mut top: Vec<usize> = (0..c.mz.len()).collect();
        top.sort_by(|&a, &b| c.rel[b].total_cmp(&c.rel[a]));
        top.truncate(MAX_PEAKS);
        top.sort_by(|&a, &b| c.mz[a].total_cmp(&c.mz[b]));
        let mx = if c.rel.is_empty() {
            1.0
        } else {
            c.rel.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        };
        let mut peaks = vec![];
        for &j in &top {
            let cand = match &form {
                Some(f) if c.rel[j] >= 0.02 * mx => {
                    best(c.mz[j], ion1, Some(f), &parse(f), ppm, &[])
                }
                _ => None,
            };
            let heavy = prec.is_some_and(|p| c.mz[j] > p + 1.5);
            let contam = heavy || (form.is_some() && c.rel[j] >= 0.05 * mx && cand.is_none());
            peaks.push((
                round(c.mz[j], 5),
                round(c.rel[j] / mx * 100.0, 1),
                cand.as_ref().map(|x| x.0.clone()),
                cand.as_ref().map(|x| x.1),
                contam,
            ));
        }
        let real = peaks
            .iter()
            .filter(|p| p.2.is_some() && p.1 >= 5.0 && prec.is_none_or(|pr| p.0 < pr - 0.5))
            .count();
        let label = paths[g]
            .iter()
            .map(|(m, a, ce)| format!("{}@{}{}", fmt_g(*m), a.to_lowercase(), fmt_g(*ce)))
            .collect::<Vec<_>>()
            .join(" > ");
        pos.push((g, nodes.len()));
        nodes.push(Node {
            parent: pk.map(|q| pos.iter().find(|(x, _)| *x == q).unwrap().1),
            path: paths[g].clone(),
            label,
            level: k.len() + 1,
            scans: groups[g].len(),
            prec: prec.map(|p| round(p, 5)),
            formula: form,
            ppm: bst.as_ref().map(|b| b.1),
            n_peaks: c.mz.len(),
            empty: real < 2,
            peaks,
        });
    }
    let mut cands: Vec<String> = vec![];
    if formula.is_none() {
        if let Some(p) = nodes.first().and_then(|n| n.prec).filter(|p| *p != 0.0) {
            let o = Opts {
                elements: Some(root_el.clone()),
                ion: ion1,
                tol: ppm,
                rdb: (-0.5, 100.0),
                rules: ROOT_RULES.iter().map(|s| s.to_string()).collect(),
                max_results: 8,
                ..Default::default()
            };
            if let Ok(s) = compose(p, &o) {
                if let Ok(v) = json::parse(s.as_bytes()) {
                    cands = v
                        .get("results")
                        .map(Value::arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(|r| r.get("formula")?.str().map(String::from))
                        .collect();
                }
            }
        }
    }
    let mut o = Out::default();
    o.raw("{\"ion\":")
        .str(ion1)
        .raw(",\"n_nodes\":")
        .int(nodes.len() as i64)
        .raw(",\"nodes\":[");
    for (i, n) in nodes.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(i as i64)
            .raw(",\"parent\":");
        match n.parent {
            Some(p) => o.int(p as i64),
            None => o.raw("null"),
        };
        o.raw(",\"path\":[");
        for (j, (m, a, ce)) in n.path.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("[")
                .num(*m)
                .raw(",")
                .str(a)
                .raw(",")
                .num(*ce)
                .raw("]");
        }
        o.raw("],\"label\":")
            .str(&n.label)
            .raw(",\"level\":")
            .int(n.level as i64)
            .raw(",\"scans\":")
            .int(n.scans as i64)
            .raw(",\"prec\":")
            .opt(n.prec)
            .raw(",\"formula\":");
        match &n.formula {
            Some(f) => o.str(f),
            None => o.raw("null"),
        };
        o.raw(",\"ppm\":")
            .opt(n.ppm)
            .raw(",\"n_peaks\":")
            .int(n.n_peaks as i64)
            .raw(",\"empty\":")
            .raw(if n.empty { "true" } else { "false" })
            .raw(",\"peaks\":[");
        for (j, (m, r, f, p, c)) in n.peaks.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" })
                .raw("{\"mz\":")
                .num(*m)
                .raw(",\"rel\":")
                .num(*r)
                .raw(",\"formula\":");
            match f {
                Some(f) => o.str(f),
                None => o.raw("null"),
            };
            o.raw(",\"ppm\":")
                .opt(*p)
                .raw(",\"contam\":")
                .raw(if *c { "true" } else { "false" })
                .raw("}");
        }
        o.raw("]}");
    }
    o.raw("],\"formula_given\":")
        .raw(if formula.is_some() { "true" } else { "false" })
        .raw(",\"root_candidates\":[");
    for (i, c) in cands.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(c);
    }
    o.raw("]}");
    Ok(o.0)
}
