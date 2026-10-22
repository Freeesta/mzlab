//! Tree search in an MSn library (`docs/agenti/msnlib.md` §2): an unknown graph `msngrafo/1` against the entries of a library `msnlib/1`. The entries whose precursor
//! is within a ppm window (same polarity) become candidates with three scores that stay apart (MS2 entropy, tree, shared losses) and the proofs behind them.
//! Nothing is called a match: the page and `spectra.match` (algorithm `tree`) show the numbers. Text in, text out like `msn-lib`.

use std::collections::HashMap;

use crate::entropy::entropy_similarity;
use crate::error::{Error, Result};
use crate::json::{parse, round, Out, Value};

/// The knobs of the search.
#[derive(Debug, Clone, PartialEq)]
pub struct Opzioni {
    /// Window of the precursor, in ppm of the unknown.
    pub ppm_prec: f64,
    /// Window of the peaks, of the precursors in the tree and of the losses by mass, in Da.
    pub tol_mz: f64,
    /// A candidate needs at least this many peaks shared with the unknown in its best MS2 pair.
    pub min_prove: usize,
    /// The candidates kept.
    pub top: usize,
    /// Analogues: entries whose precursor is up to this many Da away (0 = not searched), with the fragments split in shifted and not shifted.
    pub delta_max: f64,
}

impl Default for Opzioni {
    fn default() -> Self {
        Opzioni {
            ppm_prec: 10.0,
            tol_mz: 0.01,
            min_prove: 1,
            top: 10,
            delta_max: 0.0,
        }
    }
}

/// One spectrum of a graph: its path of precursors, activation, energy, precursor node and peaks `(mz, rel, node)`.
struct Sp {
    path: String,
    level: i64,
    act: String,
    ce: f64,
    prec: String,
    peaks: Vec<(f64, f64, Option<String>)>,
}

/// A neutral loss between two ions: the formula when the graph proposed one.
struct Loss {
    formula: Option<String>,
    mass: f64,
}

/// What the search needs of a graph.
struct G {
    pol: f64,
    mz: f64,
    nodes: HashMap<String, f64>,
    spectra: Vec<Sp>,
    losses: Vec<Loss>,
}

/// The options of the search from a request (`ppm_prec`, `tol_mz`, `min_prove`, `top`, `delta_max`; the route `api/msnlib/cerca`): absent = the default, not a number = `err.request.badNumber`.
pub fn opzioni_da_richiesta(get: &dyn Fn(&str) -> Option<String>) -> Result<Opzioni> {
    let num = |k: &str, por: f64| match get(k).filter(|v| !v.is_empty()) {
        None => Ok(por),
        Some(v) => v
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite() && *x >= 0.0)
            .ok_or_else(|| Error::new("err.request.badNumber").with("name", k)),
    };
    let d = Opzioni::default();
    Ok(Opzioni {
        ppm_prec: num("ppm_prec", d.ppm_prec)?,
        tol_mz: num("tol_mz", d.tol_mz)?,
        min_prove: num("min_prove", d.min_prove as f64)? as usize,
        top: num("top", d.top as f64)? as usize,
        delta_max: num("delta_max", d.delta_max)?,
    })
}

fn bad() -> Error {
    Error::new("err.msnlib.bad")
}

fn text(v: Option<&Value>) -> String {
    v.and_then(|x| x.str()).unwrap_or("").to_string()
}

fn read_graph(g: &Value) -> Result<G> {
    let nodes: HashMap<String, f64> = g
        .get("nodi")
        .map_or(&[][..], |x| x.arr())
        .iter()
        .filter_map(|n| Some((n.get("id")?.str()?.to_string(), n.get("mz")?.f64()?)))
        .collect();
    let root = g
        .get("radici")
        .and_then(|r| r.at(0))
        .and_then(|r| r.str())
        .and_then(|r| nodes.get(r).copied());
    let ion = g
        .get("ione")
        .and_then(|i| i.get("mz_teorico"))
        .and_then(|x| x.f64());
    let mz = root.or(ion).ok_or_else(bad)?;
    let spectra = g
        .get("spettri")
        .map_or(&[][..], |x| x.arr())
        .iter()
        .map(|s| Sp {
            path: text(s.get("albero")),
            level: s.get("livello").and_then(|x| x.f64()).unwrap_or(0.0) as i64,
            act: text(s.get("att")),
            ce: s.get("ce").and_then(|x| x.f64()).unwrap_or(0.0),
            prec: text(s.get("precursore")),
            peaks: s
                .get("picchi")
                .map_or(&[][..], |x| x.arr())
                .iter()
                .filter_map(|p| {
                    let node = p.get("nodo").and_then(|n| n.str()).map(str::to_string);
                    Some((p.get("mz")?.f64()?, p.get("rel")?.f64()?, node))
                })
                .collect(),
        })
        .collect();
    let losses = g
        .get("archi")
        .map_or(&[][..], |x| x.arr())
        .iter()
        .filter_map(|a| {
            let l = a.get("perdita")?;
            let formula = l.get("formula").and_then(|f| f.str()).map(str::to_string);
            Some(Loss {
                formula,
                mass: l.get("da_massa")?.f64()?,
            })
        })
        .collect();
    Ok(G {
        pol: g.get("polarita").and_then(|p| p.f64()).unwrap_or(1.0),
        mz,
        nodes,
        spectra,
        losses,
    })
}

/// The peaks of a spectrum as sorted m/z and intensities.
fn columns(s: &Sp) -> (Vec<f64>, Vec<f64>) {
    let mut p: Vec<(f64, f64)> = s.peaks.iter().map(|x| (x.0, x.1)).collect();
    crate::sortc::slice_by(&mut p, |a, b| a.0.total_cmp(&b.0));
    (
        p.iter().map(|x| x.0).collect(),
        p.iter().map(|x| x.1).collect(),
    )
}

/// How many peaks of `a` have a peak of `b` within `tol`.
fn shared_peaks(a: &Sp, b: &Sp, tol: f64) -> usize {
    let near = |x: f64| b.peaks.iter().any(|y| (y.0 - x).abs() <= tol);
    a.peaks.iter().filter(|p| near(p.0)).count()
}

/// The path without the energies: `305@cid35>261@cid35` → `305@cid>261@cid`, so the same ions in two energies line up.
fn shape(path: &str) -> String {
    let step = |s: &str| {
        s.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.')
            .to_string()
    };
    path.split('>').map(step).collect::<Vec<_>>().join(">")
}

/// The entropy similarity and the shared peaks of `u` against the spectrum of `list` at the nearest energy; among spectra at the same distance
/// (the same ion reached by different upstream energies) the most similar one, so a tree always finds itself.
fn best_of<'a>(u: &Sp, list: impl Iterator<Item = &'a Sp>, tol: f64) -> Option<(f64, usize)> {
    let cand: Vec<&Sp> = list.filter(|e| !e.peaks.is_empty()).collect();
    let near = cand
        .iter()
        .map(|e| (e.ce - u.ce).abs())
        .min_by(f64::total_cmp)?;
    if u.peaks.is_empty() {
        return None;
    }
    let am = columns(u);
    cand.iter()
        .filter(|e| (e.ce - u.ce).abs() - near <= 1e-9)
        .map(|e| {
            let bm = columns(e);
            (
                entropy_similarity(&am.0, &am.1, &bm.0, &bm.1, tol),
                shared_peaks(u, e, tol),
            )
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// The precursor m/z of a spectrum of the graph.
fn prec_mz(g: &G, s: &Sp) -> f64 {
    g.nodes.get(&s.prec).copied().unwrap_or(0.0)
}

/// How much the isolated ion weighs: its relative intensity in the spectrum above it (1 when that spectrum does not show it).
fn weight(g: &G, s: &Sp) -> f64 {
    let up = s.path.rsplit_once('>').map(|x| x.0);
    let peak = g
        .spectra
        .iter()
        .filter(|p| Some(p.path.as_str()) == up)
        .flat_map(|p| p.peaks.iter())
        .find(|p| p.2.as_deref() == Some(s.prec.as_str()));
    peak.map_or(1.0, |p| p.1.max(1e-9))
}

struct Scores {
    ms2: Option<f64>,
    shared: usize,
    tree: Option<f64>,
    aligned: usize,
    total: usize,
    losses: Option<f64>,
    common: usize,
    unknown: usize,
}

fn same_loss(a: &Loss, b: &Loss, tol: f64) -> bool {
    match (&a.formula, &b.formula) {
        (Some(x), Some(y)) => x == y,
        _ => (a.mass - b.mass).abs() <= tol,
    }
}

fn score(u: &G, e: &G, tol: f64) -> Scores {
    let mut ms2 = None;
    let mut shared = 0;
    for s in u.spectra.iter().filter(|s| s.level == 2) {
        let same = e.spectra.iter().filter(|x| x.level == 2 && x.act == s.act);
        if let Some((h, n)) = best_of(s, same, tol) {
            if ms2.is_none_or(|b| h > b) {
                (ms2, shared) = (Some(h), n);
            }
        }
    }
    let deep: Vec<&Sp> = u.spectra.iter().filter(|s| s.level >= 3).collect();
    let (mut sum, mut wsum, mut aligned) = (0.0, 0.0, 0);
    for s in &deep {
        let (w, key, p) = (weight(u, s), shape(&s.path), prec_mz(u, s));
        let same = e.spectra.iter().filter(|x| {
            shape(&x.path) == key && x.act == s.act && (prec_mz(e, x) - p).abs() <= tol
        });
        if let Some((h, _)) = best_of(s, same, tol) {
            (sum, wsum, aligned) = (sum + w * h, wsum + w, aligned + 1);
        }
    }
    let mut ls: Vec<&Loss> = Vec::new();
    for l in &u.losses {
        if !ls.iter().any(|k| same_loss(k, l, tol)) {
            ls.push(l);
        }
    }
    let common = ls
        .iter()
        .filter(|l| e.losses.iter().any(|k| same_loss(k, l, tol)))
        .count();
    Scores {
        ms2,
        shared,
        tree: (wsum > 0.0).then(|| sum / wsum),
        aligned,
        total: deep.len(),
        losses: (!ls.is_empty()).then(|| common as f64 / ls.len() as f64),
        common,
        unknown: ls.len(),
    }
}

/// The peaks of the unknown explained by the entry in the best MS2 pair: at the same m/z (`unshifted`) or moved by the precursor difference (`shifted`).
struct Analog {
    delta: f64,
    shifted: usize,
    unshifted: usize,
    cover: f64,
}

fn analog(u: &G, e: &G, tol: f64) -> Option<Analog> {
    let delta = u.mz - e.mz;
    let mut best: Option<Analog> = None;
    for s in u.spectra.iter().filter(|s| s.level == 2) {
        let same = e.spectra.iter().filter(|x| x.level == 2 && x.act == s.act);
        let Some(x) = same.min_by(|a, b| (a.ce - s.ce).abs().total_cmp(&(b.ce - s.ce).abs()))
        else {
            continue;
        };
        let (mut shifted, mut unshifted, mut got, mut all) = (0, 0, 0.0, 0.0);
        for p in &s.peaks {
            all += p.1;
            if x.peaks.iter().any(|y| (y.0 - p.0).abs() <= tol) {
                unshifted += 1;
                got += p.1;
            } else if x.peaks.iter().any(|y| (y.0 + delta - p.0).abs() <= tol) {
                shifted += 1;
                got += p.1;
            }
        }
        let cover = if all > 0.0 { got / all } else { 0.0 };
        if best.as_ref().is_none_or(|b| cover > b.cover) {
            best = Some(Analog {
                delta,
                shifted,
                unshifted,
                cover,
            });
        }
    }
    best
}

/// A candidate: the entry, its distance in precursor and its scores.
struct Hit {
    id: String,
    ion: String,
    act: Vec<String>,
    ppm: f64,
    s: Scores,
    analog: Option<Analog>,
}

fn rank(a: &Option<f64>, b: &Option<f64>) -> std::cmp::Ordering {
    // the larger score first, a missing one last
    match (a, b) {
        (Some(x), Some(y)) => y.total_cmp(x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn hits(lib: &Value, u: &G, o: &Opzioni) -> Result<Vec<Hit>> {
    let mut out = Vec::new();
    for v in lib.get("voci").map_or(&[][..], |x| x.arr()) {
        let e = read_graph(v.get("grafo").ok_or_else(bad)?)?;
        let ppm = (e.mz - u.mz) / u.mz * 1e6;
        if (e.pol < 0.0) != (u.pol < 0.0) {
            continue;
        }
        let s = score(u, &e, o.tol_mz);
        let analog = if ppm.abs() <= o.ppm_prec {
            if s.shared < o.min_prove {
                continue;
            }
            None
        } else if o.delta_max > 0.0 && (u.mz - e.mz).abs() <= o.delta_max {
            match analog(u, &e, o.tol_mz) {
                Some(a) if a.shifted + a.unshifted >= o.min_prove && a.shifted > 0 => Some(a),
                _ => continue,
            }
        } else {
            continue;
        };
        let act = v.get("attivazione").map_or(&[][..], |a| a.arr());
        out.push(Hit {
            id: text(v.get("id")),
            ion: text(v.get("ione").and_then(|i| i.get("formula"))),
            act: act
                .iter()
                .filter_map(|a| a.str())
                .map(str::to_string)
                .collect(),
            ppm,
            s,
            analog,
        });
    }
    crate::sortc::slice_by(&mut out, |a, b| {
        a.analog
            .is_some()
            .cmp(&b.analog.is_some())
            .then_with(|| match (&a.analog, &b.analog) {
                (Some(x), Some(y)) => y.cover.total_cmp(&x.cover),
                _ => std::cmp::Ordering::Equal,
            })
            .then(rank(&a.s.tree, &b.s.tree))
            .then(rank(&a.s.ms2, &b.s.ms2))
            .then(rank(&a.s.losses, &b.s.losses))
            .then(a.id.cmp(&b.id))
    });
    out.truncate(o.top);
    Ok(out)
}

/// An MSn library entry whose MS2 resembles the MS2 of a feature: the entropy similarity of its best MS2 and the peaks they share.
pub struct Ms2Hit {
    pub id: String,
    pub ion: String,
    pub entropy: f64,
    pub shared: usize,
}

/// The entries of an `msnlib/1` text whose precursor is within `ppm` of `mz` (same polarity) and whose best MS2, at any activation or energy, shares
/// at least `min_shared` peaks with `peaks` `(m/z, intensity)`; most similar first. A feature has no activation label, so every MS2 of the entry counts.
pub fn cerca_ms2(
    libreria: &str,
    mz: f64,
    positive: bool,
    peaks: &[(f64, f64)],
    ppm: f64,
    tol: f64,
    min_shared: usize,
) -> Result<Vec<Ms2Hit>> {
    let lib = parse(libreria.as_bytes())?;
    let u = Sp {
        path: String::new(),
        level: 2,
        act: String::new(),
        ce: 0.0,
        prec: String::new(),
        peaks: peaks.iter().map(|p| (p.0, p.1, None)).collect(),
    };
    let mut out = Vec::new();
    for v in lib.get("voci").map_or(&[][..], |x| x.arr()) {
        let e = read_graph(v.get("grafo").ok_or_else(bad)?)?;
        if (e.pol >= 0.0) != positive || ((e.mz - mz) / mz * 1e6).abs() > ppm {
            continue;
        }
        let best = e
            .spectra
            .iter()
            .filter(|x| x.level == 2)
            .filter_map(|x| best_of(&u, std::iter::once(x), tol))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((entropy, shared)) = best.filter(|b| b.1 >= min_shared) {
            out.push(Ms2Hit {
                id: text(v.get("id")),
                ion: text(v.get("ione").and_then(|i| i.get("formula"))),
                entropy,
                shared,
            });
        }
    }
    crate::sortc::slice_by(&mut out, |a, b| {
        b.entropy.total_cmp(&a.entropy).then(a.id.cmp(&b.id))
    });
    Ok(out)
}

fn write(o: &mut Out, h: &Hit) {
    let opt = |o: &mut Out, v: Option<f64>| {
        o.opt(v.map(|x| round(x, 4)));
    };
    o.raw("{\"id\":")
        .str(&h.id)
        .raw(",\"ione\":")
        .str(&h.ion)
        .raw(",\"att\":[");
    for (i, a) in h.act.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"delta_ppm\":")
        .num(round(h.ppm, 2))
        .raw(",\"punteggi\":{\"ms2\":");
    opt(o, h.s.ms2);
    o.raw(",\"albero\":");
    opt(o, h.s.tree);
    o.raw(",\"perdite\":");
    opt(o, h.s.losses);
    o.raw("},\"prove\":{\"picchi_comuni\":")
        .int(h.s.shared as i64);
    o.raw(",\"nodi_allineati\":").int(h.s.aligned as i64);
    o.raw(",\"nodi_ignoto\":").int(h.s.total as i64);
    o.raw(",\"perdite_comuni\":").int(h.s.common as i64);
    o.raw(",\"perdite_ignoto\":").int(h.s.unknown as i64);
    match &h.analog {
        None => o.raw("},\"analogo\":null}"),
        Some(a) => o
            .raw("},\"analogo\":{\"delta_da\":")
            .num(round(a.delta, 4))
            .raw(",\"spostati\":")
            .int(a.shifted as i64)
            .raw(",\"non_spostati\":")
            .int(a.unshifted as i64)
            .raw(",\"copertura\":")
            .num(round(a.cover, 4))
            .raw("}}"),
    };
}

/// The candidates of the library text `libreria` (`msnlib/1`) for the graph text `ignoto` (`msngrafo/1`), as the JSON `msncerca/1`.
pub fn cerca(libreria: &str, ignoto: &str, o: &Opzioni) -> Result<String> {
    let lib = parse(libreria.as_bytes()).map_err(|_| bad())?;
    let g = parse(ignoto.as_bytes()).map_err(|_| bad())?;
    let u = read_graph(&g)?;
    let found = hits(&lib, &u, o)?;
    let mut out = Out::default();
    out.raw("{\"formato\":\"msncerca/1\",\"opzioni\":{\"ppm_prec\":")
        .num(o.ppm_prec);
    out.raw(",\"tol_mz\":").num(o.tol_mz);
    out.raw(",\"min_prove\":").int(o.min_prove as i64);
    out.raw(",\"top\":").int(o.top as i64);
    out.raw(",\"delta_max\":")
        .num(o.delta_max)
        .raw("},\"candidati\":[");
    for (i, h) in found.iter().enumerate() {
        out.raw(if i > 0 { "," } else { "" });
        write(&mut out, h);
    }
    out.raw("]}\n");
    Ok(out.0)
}
