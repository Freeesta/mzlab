//! `mzlab-cli msn-graph compare A.json B.json`: the validation of an MSn tree (`docs/agenti/msnlib.md` §4). Two `msngrafo/1` graphs of the same standard (two days, same options)
//! are checked on five criteria: the isolated nodes (same paths of precursors), the arcs with `rel` >= 10 % in both, the formulas proposed, the MS2 entropy per energy and the
//! m/z shift of the matched nodes. The answer is a JSON with the numbers; `convalidata` is true when every criterion holds. Used by `lib.rs` (`msn_graph_cmd`).

use std::path::PathBuf;

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, round, Out, Value};
use mzlab_lib::scores::{entropy_from_pairs, match_peaks};

/// Matching window of nodes, arcs and paths, ppm.
const PPM: f64 = 5.0;
/// Least relative intensity of an arc to count.
const EDGE_REL: f64 = 10.0;
/// Least MS2 entropy similarity of two spectra of the same energy.
const ENTROPY_MIN: f64 = 0.8;
/// Largest m/z shift of a matched node, ppm.
const DEVIATION_MAX: f64 = 3.0;

struct Node {
    mz: f64,
    isolated: bool,
    path: Vec<String>,
    formula: Option<String>,
}

struct Edge {
    da: f64,
    a: f64,
    rel: f64,
}

struct Spettro {
    att: String,
    ce: f64,
    path: Vec<String>,
    peaks: Vec<(f64, f64)>,
}

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    spettri: Vec<Spettro>,
}

fn bad() -> Error {
    Error::new("err.msnlib.bad")
}

/// The m/z labels of the steps of a tree path (`305@cid35>244@cid35` → ["305", "244"]): the labels are rounded, the precise m/z is the one of the node.
fn steps(albero: &str) -> Vec<String> {
    albero
        .split('>')
        .filter_map(|s| Some(s.split('@').next()?.to_string()))
        .collect()
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.f64()).unwrap_or(0.0)
}

fn load(text: &str) -> Result<Graph> {
    let v = parse(text.as_bytes())?;
    mzlab_core::msngrafo::versione(&v).map_err(|e| {
        if e.key == "error.newerFormat" {
            e
        } else {
            bad()
        }
    })?;
    let list = |k: &str| v.get(k).map_or(&[][..], |x| x.arr());
    let nodi = list("nodi")
        .iter()
        .map(|n| Node {
            mz: num(n, "mz"),
            isolated: n.get("isolato").and_then(|b| b.bool()).unwrap_or(false),
            path: n
                .get("albero")
                .and_then(|a| a.arr().first())
                .and_then(|p| p.str())
                .map(steps)
                .unwrap_or_default(),
            formula: n
                .get("formula")
                .and_then(|f| f.get("scelta")?.str())
                .map(str::to_string),
        })
        .collect::<Vec<_>>();
    let mz_di = |id: Option<&str>| {
        let k = id?.strip_prefix('n')?.parse::<usize>().ok()?;
        nodi.get(k).map(|n| n.mz)
    };
    let archi = list("archi")
        .iter()
        .filter_map(|a| {
            let rel = a
                .get("misure")
                .map_or(&[][..], |m| m.arr())
                .iter()
                .map(|m| num(m, "rel"))
                .fold(0.0, f64::max);
            Some(Edge {
                da: mz_di(a.get("da")?.str())?,
                a: mz_di(a.get("a")?.str())?,
                rel,
            })
        })
        .collect();
    let spettri = list("spettri")
        .iter()
        .filter(|s| num(s, "livello") == 2.0)
        .map(|s| {
            let mut picchi: Vec<(f64, f64)> = s
                .get("picchi")
                .map_or(&[][..], |p| p.arr())
                .iter()
                .map(|p| (num(p, "mz"), num(p, "rel")))
                .collect();
            picchi.sort_by(|x, y| x.0.total_cmp(&y.0));
            Spettro {
                att: s.get("att").and_then(|x| x.str()).unwrap_or("").into(),
                ce: num(s, "ce"),
                path: steps(s.get("albero").and_then(|x| x.str()).unwrap_or("")),
                peaks: picchi,
            }
        })
        .collect();
    Ok(Graph {
        nodes: nodi,
        edges: archi,
        spettri,
    })
}

fn ppm(a: f64, b: f64) -> f64 {
    (a - b) / b * 1e6
}

/// One-to-one pairs `(i, j)` whose cost is known, the cheapest first.
fn accoppia(
    na: usize,
    nb: usize,
    cost: impl Fn(usize, usize) -> Option<f64>,
) -> Vec<(usize, usize, f64)> {
    let mut c: Vec<(usize, usize, f64)> = (0..na)
        .flat_map(|i| (0..nb).map(move |j| (i, j)))
        .filter_map(|(i, j)| Some((i, j, cost(i, j)?)))
        .collect();
    c.sort_by(|x, y| x.2.total_cmp(&y.2));
    let (mut ua, mut ub) = (vec![false; na], vec![false; nb]);
    c.retain(|&(i, j, _)| {
        !(ua[i] || ub[j]) && {
            ua[i] = true;
            ub[j] = true;
            true
        }
    });
    c
}

fn criterio(o: &mut Out, name: &str, ok: bool, fields: &[(&str, f64)]) {
    o.raw(",\"").raw(name).raw("\":{");
    for (k, v) in fields {
        o.raw("\"").raw(k).raw("\":").num(round(*v, 4)).raw(",");
    }
    o.raw("\"ok\":")
        .raw(if ok { "true" } else { "false" })
        .raw("}");
}

fn forti(g: &Graph) -> Vec<&Edge> {
    g.edges.iter().filter(|a| a.rel >= EDGE_REL).collect()
}

fn c_nodes(ga: &Graph, gb: &Graph, corpo: &mut Out) -> bool {
    let (ia, ib): (Vec<&Node>, Vec<&Node>) = (
        ga.nodes.iter().filter(|n| n.isolated).collect(),
        gb.nodes.iter().filter(|n| n.isolated).collect(),
    );
    let common = accoppia(ia.len(), ib.len(), |i, j| {
        let m = ppm(ia[i].mz, ib[j].mz).abs();
        (!ia[i].path.is_empty() && ia[i].path == ib[j].path && m <= PPM).then_some(m)
    })
    .len();
    let ok = common == ia.len() && common == ib.len();
    let fields = [
        ("a", ia.len() as f64),
        ("b", ib.len() as f64),
        ("comuni", common as f64),
    ];
    criterio(corpo, "nodi_isolati", ok, &fields);
    ok
}

fn c_edges(ga: &Graph, gb: &Graph, corpo: &mut Out) -> bool {
    let (fa, fb) = (forti(ga), forti(gb));
    let common = accoppia(fa.len(), fb.len(), |i, j| {
        let m = ppm(fa[i].da, fb[j].da)
            .abs()
            .max(ppm(fa[i].a, fb[j].a).abs());
        (m <= PPM).then_some(m)
    })
    .len();
    let ok = common == fa.len() && common == fb.len();
    let fields = [
        ("forti_a", fa.len() as f64),
        ("forti_b", fb.len() as f64),
        ("comuni", common as f64),
    ];
    criterio(corpo, "archi", ok, &fields);
    ok
}

/// Nodes paired by m/z, shared by the formula and the shift checks.
fn node_pairs(ga: &Graph, gb: &Graph) -> Vec<(usize, usize, f64)> {
    accoppia(ga.nodes.len(), gb.nodes.len(), |i, j| {
        let m = ppm(ga.nodes[i].mz, gb.nodes[j].mz).abs();
        (m <= PPM).then_some(m)
    })
}

fn c_formule(ga: &Graph, gb: &Graph, pairs: &[(usize, usize, f64)], corpo: &mut Out) -> bool {
    let (mut seen, mut different) = (0, 0);
    for &(i, j, _) in pairs {
        let (x, y) = (&ga.nodes[i].formula, &gb.nodes[j].formula);
        if x.is_some() || y.is_some() {
            seen += 1;
            different += usize::from(x != y);
        }
    }
    let fields = [("confrontati", seen as f64), ("diversi", different as f64)];
    criterio(corpo, "formule", different == 0, &fields);
    different == 0
}

fn c_entropy(ga: &Graph, gb: &Graph, corpo: &mut Out) -> bool {
    let sp = accoppia(ga.spettri.len(), gb.spettri.len(), |i, j| {
        let (x, y) = (&ga.spettri[i], &gb.spettri[j]);
        (x.path == y.path && x.att == y.att && (x.ce - y.ce).abs() < 0.01).then_some(0.0)
    });
    let ent: Vec<f64> = sp
        .iter()
        .map(|&(i, j, _)| entropy(&ga.spettri[i], &gb.spettri[j]))
        .collect();
    let minimum = ent.iter().copied().fold(1.0, f64::min);
    let without = ga.spettri.len() + gb.spettri.len() - 2 * sp.len();
    let ok = without == 0 && minimum >= ENTROPY_MIN;
    let fields = [
        ("minimo", minimum),
        ("soglia", ENTROPY_MIN),
        ("coppie", sp.len() as f64),
        ("senza_controparte", without as f64),
    ];
    criterio(corpo, "entropia_ms2", ok, &fields);
    ok
}

fn c_deviation(pairs: &[(usize, usize, f64)], corpo: &mut Out) -> bool {
    let maximum = pairs.iter().map(|c| c.2).fold(0.0, f64::max);
    let ok = maximum <= DEVIATION_MAX;
    let fields = [
        ("massimo", maximum),
        ("soglia", DEVIATION_MAX),
        ("coppie", pairs.len() as f64),
    ];
    criterio(corpo, "scarto_ppm", ok, &fields);
    ok
}

/// The comparison text for two graph texts.
pub fn compare(a: &str, b: &str) -> Result<String> {
    let (ga, gb) = (load(a)?, load(b)?);
    let mut corpo = Out::default();
    let pairs = node_pairs(&ga, &gb);
    // Every criterion is evaluated (no short-circuit) so the outcome lists all numbers.
    let oks = [
        c_nodes(&ga, &gb, &mut corpo),
        c_edges(&ga, &gb, &mut corpo),
        c_formule(&ga, &gb, &pairs, &mut corpo),
        c_entropy(&ga, &gb, &mut corpo),
        c_deviation(&pairs, &mut corpo),
    ];
    let all = oks.iter().all(|&k| k);
    let mut o = Out::default();
    o.raw("{\"formato\":\"msnconfronto/1\",\"ppm\":").num(PPM);
    o.raw(",\"convalidata\":")
        .raw(if all { "true" } else { "false" });
    o.raw(&corpo.0).raw("}\n");
    Ok(o.0)
}

/// Entropy similarity of two MS2 spectra, peaks matched within `PPM` of the largest m/z.
fn entropy(a: &Spettro, b: &Spettro) -> f64 {
    let (am, ai): (Vec<f64>, Vec<f64>) = a.peaks.iter().copied().unzip();
    let (bm, bi): (Vec<f64>, Vec<f64>) = b.peaks.iter().copied().unzip();
    let top = am.iter().chain(&bm).copied().fold(0.0, f64::max);
    let pairs = match_peaks(&am, &ai, &bm, &bi, top * PPM * 1e-6);
    entropy_from_pairs(&ai, &bi, &pairs)
}

/// The arguments after `compare`: the two graph files and `--out`.
pub fn esegui(args: impl Iterator<Item = String>) -> Result<(String, Option<PathBuf>)> {
    let usage = |m: &str| Error::new("err.flow.param.type").with("detail", m);
    let (mut files, mut out) = (Vec::new(), None);
    let mut it = args;
    while let Some(x) = it.next() {
        match x.as_str() {
            "--out" => out = it.next().map(PathBuf::from),
            f if !f.starts_with("--") => files.push(PathBuf::from(f)),
            f => return Err(usage(&format!("unknown argument {f}"))),
        }
    }
    let [a, b] = files.as_slice() else {
        return Err(usage(
            "usage: mzlab-cli msn-graph compare A.json B.json [--out PATH]",
        ));
    };
    let read = |p: &PathBuf| {
        std::fs::read_to_string(p)
            .map_err(|e| Error::new("err.flow.param.type").with("detail", e.to_string()))
    };
    Ok((compare(&read(a)?, &read(b)?)?, out))
}
