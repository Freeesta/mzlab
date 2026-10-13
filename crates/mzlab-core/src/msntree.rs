//! Tree of the fragmentation paths of an MSn file (`mzlab/chem/msntree.py`): one node per path, its consensus spectrum, the exact precursor
//! read from the parent node and the formulas (a node is a sub-formula of its parent). Candidates, never a verdict.
//! The spectra come through a callback, so the same code serves the browser, the desktop app and the tests.

use crate::composition::{compose, Opts, Ranges};
use crate::elements::MASS;
use crate::error::{Error, Result};
use crate::json::{self, fmt_g, round, Out, Value};
use crate::msnalbero::{albero_di, consenso, Raggruppa};

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
    // nodes and parents from the one builder of the tree (`msnalbero`): a node without the generation before hangs from the nearest ancestor
    let tree = albero_di(
        scans.iter().map(|s| (s.level, Some(&s.path[..]))),
        &Raggruppa::default(),
    );
    if tree.nodi.is_empty() {
        return Err(Error::new("err.tree.noPath"));
    }
    if tree.nodi.iter().map(|n| n.chiave.len()).max().unwrap() < 2 {
        return Err(Error::new("err.tree.needStages"));
    }
    if tree.nodi.len() > 400 {
        return Err(Error::new("err.tree.tooMany").with("n", tree.nodi.len()));
    }
    let pol = scans
        .iter()
        .map(|s| s.polarity)
        .find(|p| *p != 0)
        .unwrap_or(1);
    let ion1 = if pol >= 0 { "[M]+" } else { "[M]-" };
    let mut cons_of = |idx: &[usize]| -> Result<Consensus> {
        let spectra = idx
            .iter()
            .take(MAX_SCANS)
            .map(|&i| read(scans[i].index))
            .collect::<Result<Vec<_>>>()?;
        let (mz, rel, _) = consenso(&spectra, 5.0, 0.5, 0.0);
        Ok(Consensus { mz, rel })
    };
    let mut cons: Vec<Consensus> = vec![];
    for n in &tree.nodi {
        cons.push(cons_of(&n.scansioni)?);
    }
    let ms1c = if tree.ms1.is_empty() {
        None
    } else {
        Some(cons_of(&tree.ms1)?)
    };
    let root_el: Ranges = ROOT_ELEMENTS
        .iter()
        .map(|(s, a, b)| (s.to_string(), (*a, *b)))
        .collect();
    let mut nodes: Vec<Node> = vec![];
    for (g, tn) in tree.nodi.iter().enumerate() {
        let nom = tn.chiave.last().unwrap().0;
        let path = &scans[tn.scansioni[0]].path;
        let pk = tn.genitore;
        let src = match pk {
            Some(q) => Some(&cons[q]),
            None => ms1c.as_ref(),
        };
        let mut prec: Option<f64> = None;
        if let Some(c) = src {
            let mut bi: Option<usize> = None;
            for (i, m) in c.mz.iter().enumerate() {
                if (m - nom).abs() <= 0.5 && bi.is_none_or(|b| c.rel[i] > c.rel[b]) {
                    bi = Some(i);
                }
            }
            prec = bi.map(|i| c.mz[i]);
        }
        let pnode = pk.map(|q| &nodes[q]);
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
        let label = path
            .iter()
            .map(|(m, a, ce)| format!("{}@{}{}", fmt_g(*m), a.to_lowercase(), fmt_g(*ce)))
            .collect::<Vec<_>>()
            .join(" > ");
        nodes.push(Node {
            parent: pk,
            path: path.clone(),
            label,
            level: tn.chiave.len() + 1,
            scans: tn.scansioni.len(),
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
