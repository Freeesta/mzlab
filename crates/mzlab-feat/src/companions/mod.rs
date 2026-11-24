//! `features.companions@1`: the ions that co-elute with a row of an aligned unit-mass table and differ from it by a known nominal mass (M+1/M+2
//! isotopes, adducts, neutral losses) or only by their time profile, written as edges (`companions/1`) with the numbers behind each: RT offset, Pearson
//! of the heights over the files, intensity ratio and its spread. No row is merged or removed; a mass difference alone never makes an edge.

mod evidence;
pub mod list;
mod text;

use crate::store::read_table;
use crate::table::Aligned;
use mzlab_core::error::Result;
use mzlab_core::json::Value;

pub use list::Rel;
pub use text::to_json;

/// The warning of a table with fewer files than `min_corr_samples`: no pair can be tested, so no edge is written (the candidates are listed as `weak`).
pub const WARN_CORR_FILES: &str = "warn.flow.companions.corrFiles";

/// The parameters of the step (descriptor `steps::companions`).
#[derive(Debug, Clone)]
pub struct Params {
    pub rt_tol: f64,
    pub min_corr: f64,
    pub weak_corr: f64,
    pub min_corr_samples: usize,
    pub da_tol: f64,
    pub adducts: Vec<Rel>,
    pub losses: Vec<Rel>,
    pub max_edges: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Isotope,
    Relation,
    Coelution,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Isotope => "isotope",
            Kind::Relation => "relation",
            Kind::Coelution => "coelution",
        }
    }
}

/// An edge from the lighter row `a` to the heavier row `b` (row indices). `parent` = the row the other one is a companion of (`None` for co-elution);
/// `rule` says why that one.
#[derive(Debug, Clone)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub kind: Kind,
    pub relations: Vec<String>,
    pub parent: Option<usize>,
    pub rule: &'static str,
    pub ev: evidence::Evidence,
}

/// The counts of the summary: every candidate pair ends in one of them.
#[derive(Debug, Clone, Default)]
pub struct Counts {
    pub pairs: usize,
    pub isotope: usize,
    pub relation: usize,
    pub coelution: usize,
    pub weak: usize,
    pub untested: usize,
    pub implied: usize,
    pub dropped: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Found {
    pub edges: Vec<Edge>,
    pub weak: Vec<Edge>,
    pub counts: Counts,
    /// Per row: the parent it is a companion of (row index) and the kind of the edge that says so.
    pub companion_of: Vec<Option<(usize, Kind)>>,
}

/// The `companions/1` text and its warnings.
#[derive(Debug, Clone)]
pub struct Made {
    pub text: String,
    pub warnings: Vec<&'static str>,
}

fn num(params: &[(String, Value)], k: &str, d: f64) -> f64 {
    params
        .iter()
        .find(|(n, _)| n == k)
        .and_then(|(_, v)| v.f64())
        .unwrap_or(d)
}

fn text<'a>(params: &'a [(String, Value)], k: &str, d: &'a str) -> &'a str {
    params
        .iter()
        .find(|(n, _)| n == k)
        .and_then(|(_, v)| v.str())
        .unwrap_or(d)
}

/// The parameters of the step from the canonical parameters of the plan; a malformed list is `err.flow.companions.list`.
pub fn params_of(params: &[(String, Value)]) -> Result<Params> {
    Ok(Params {
        rt_tol: num(params, "rt_tol", 0.1),
        min_corr: num(params, "min_corr", 0.9),
        weak_corr: num(params, "weak_corr", 0.5),
        min_corr_samples: num(params, "min_corr_samples", 3.0).max(2.0) as usize,
        da_tol: num(params, "da_tol", 0.4).max(0.0),
        adducts: list::parse(text(params, "adducts", list::ADDUCTS), list::ADDUCT_NAMES)?,
        losses: list::parse(text(params, "losses", list::LOSSES), list::LOSS_NAMES)?,
        max_edges: num(params, "max_edges", 20000.0).max(0.0) as usize,
    })
}

/// The `companions/1` text of the `.mzfeat` bytes `table` under the parameters of the plan: what `flow run` and the `flowtab` `.wasm` both call.
pub fn companions_text(params: &[(String, Value)], table: &[u8]) -> Result<Made> {
    let (t, _) = read_table(table)?;
    let p = params_of(params)?;
    let f = find(&t, &p);
    let tested = t.n_files >= p.min_corr_samples;
    Ok(Made {
        text: to_json(&t, &p, &f),
        warnings: if tested {
            Vec::new()
        } else {
            vec![WARN_CORR_FILES]
        },
    })
}

/// Every pair of rows within `rt_tol` whose nominal difference fits a relation, or that only co-elute, tested on the heights over the files.
pub fn find(t: &Aligned, p: &Params) -> Found {
    let n = t.len();
    let mut out = Found {
        companion_of: vec![None; n],
        ..Found::default()
    };
    let order = crate::sortc::order(n, &mut |i, j| {
        crate::sortc::asc_nan_last(t.rt[i], t.rt[j]).then(i.cmp(&j))
    });
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if t.rt[j as usize] - t.rt[i as usize] > p.rt_tol {
                break;
            }
            pair(t, p, i as usize, j as usize, &mut out);
        }
    }
    let by_ids = |x: &Edge, y: &Edge| x.a.cmp(&y.a).then(x.b.cmp(&y.b)).then(x.kind.cmp(&y.kind));
    crate::sortc::any_by(&mut out.edges, by_ids);
    // the weak list is cut at `text::WEAK_CAP`: the strongest first (|Pearson|, an untested pair last), so the kept ones are the most informative
    crate::sortc::any_by(&mut out.weak, |x, y| {
        crate::sortc::asc_nan_last(-x.ev.r.abs(), -y.ev.r.abs()).then(by_ids(x, y))
    });
    drop_implied(&mut out, n);
    cap_edges(&mut out, p.max_edges);
    choose_parents(&mut out);
    out
}

/// At most `max` edges stay, the strongest (Pearson, then the rows): the others are counted in `counts.dropped` and in their kind's count no more.
/// The cap comes after `drop_implied`, so an edge that adds nothing never takes the place of one that does.
fn cap_edges(f: &mut Found, max: usize) {
    if f.edges.len() <= max {
        return;
    }
    let mut rank: Vec<usize> = (0..f.edges.len()).collect();
    let e = &f.edges;
    crate::sortc::any_by(&mut rank, |&x, &y| {
        crate::sortc::asc_nan_last(-e[x].ev.r, -e[y].ev.r)
            .then(e[x].a.cmp(&e[y].a))
            .then(e[x].b.cmp(&e[y].b))
            .then(e[x].kind.cmp(&e[y].kind))
    });
    let mut keep = vec![false; e.len()];
    for &i in &rank[..max] {
        keep[i] = true;
    }
    let mut i = 0;
    let mut cut = Counts::default();
    f.edges.retain(|e| {
        i += 1;
        if !keep[i - 1] {
            match e.kind {
                Kind::Isotope => cut.isotope += 1,
                Kind::Relation => cut.relation += 1,
                Kind::Coelution => cut.coelution += 1,
            }
        }
        keep[i - 1]
    });
    f.counts.isotope -= cut.isotope;
    f.counts.relation -= cut.relation;
    f.counts.coelution -= cut.coelution;
    f.counts.dropped = cut.isotope + cut.relation + cut.coelution;
}

/// A co-elution edge between two rows that isotope and relation edges already link (through any chain) says nothing new, and of several edges between
/// one row and the members of one linked group only the strongest tells the news: the others are counted in `counts.implied` and left out.
fn drop_implied(f: &mut Found, n: usize) {
    let mut root: Vec<usize> = (0..n).collect();
    fn find_root(root: &mut [usize], mut x: usize) -> usize {
        while root[x] != x {
            root[x] = root[root[x]];
            x = root[x];
        }
        x
    }
    for e in f.edges.iter().filter(|e| e.kind != Kind::Coelution) {
        let (ra, rb) = (find_root(&mut root, e.a), find_root(&mut root, e.b));
        root[ra] = rb;
    }
    // (group pair, -r, from, to, index) of every co-elution edge, strongest first within its group pair
    let mut co: Vec<(usize, usize, f64, usize, usize, usize)> = Vec::new();
    for (i, e) in f
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == Kind::Coelution)
    {
        let (ra, rb) = (find_root(&mut root, e.a), find_root(&mut root, e.b));
        co.push((ra.min(rb), ra.max(rb), -e.ev.r, e.a, e.b, i));
    }
    crate::sortc::any_by(&mut co, |x, y| {
        (x.0, x.1)
            .cmp(&(y.0, y.1))
            .then(crate::sortc::asc_nan_last(x.2, y.2))
            .then((x.3, x.4).cmp(&(y.3, y.4)))
    });
    let mut keep = vec![true; f.edges.len()];
    for (k, c) in co.iter().enumerate() {
        let repeat = k > 0 && (co[k - 1].0, co[k - 1].1) == (c.0, c.1);
        keep[c.5] = c.0 != c.1 && !repeat;
    }
    let mut i = 0;
    let before = f.edges.len();
    f.edges.retain(|_| {
        i += 1;
        keep[i - 1]
    });
    let gone = before - f.edges.len();
    f.counts.coelution -= gone;
    f.counts.implied = gone;
}

/// The edges a pair of rows gives: one per fitting kind, or a co-elution edge when no mass difference fits.
fn pair(t: &Aligned, p: &Params, i: usize, j: usize, out: &mut Found) {
    let (a, b) = if t.mz[i] <= t.mz[j] { (i, j) } else { (j, i) };
    let d = t.mz[b] - t.mz[a];
    if d < p.da_tol {
        return;
    }
    out.counts.pairs += 1;
    let mut kinds: Vec<(Kind, Vec<String>)> = Vec::new();
    for k in [1.0, 2.0] {
        if (d - k).abs() <= p.da_tol {
            kinds.push((Kind::Isotope, vec![format!("isotope_p{k}")]));
        }
    }
    let rel = list::fitting(p, d);
    if !rel.is_empty() {
        kinds.push((Kind::Relation, rel));
    }
    let explained = !kinds.is_empty();
    if !explained {
        kinds.push((Kind::Coelution, Vec::new()));
    }
    let ev = evidence::of(t, a, b, p.min_corr_samples);
    out.counts.untested += usize::from(!ev.r.is_finite());
    for (kind, relations) in kinds {
        let mut e = Edge {
            a,
            b,
            kind,
            relations,
            parent: None,
            rule: "",
            ev: ev.clone(),
        };
        if !ev.r.is_finite() {
            if explained {
                out.weak.push(e);
            }
        } else if ev.r >= p.min_corr {
            parent_of(t, &mut e);
            out.edges.push(e);
            match kind {
                Kind::Isotope => out.counts.isotope += 1,
                Kind::Relation => out.counts.relation += 1,
                Kind::Coelution => out.counts.coelution += 1,
            }
        } else if explained && ev.r >= p.weak_corr {
            out.counts.weak += 1;
            out.weak.push(e);
        }
    }
}

/// The parent of an edge: the lighter row of an isotope step or an adduct, the heavier row of a loss; when both fit the step, the more intense row.
fn parent_of(t: &Aligned, e: &mut Edge) {
    if e.kind == Kind::Coelution {
        return;
    }
    let (adduct, loss) = if e.kind == Kind::Isotope {
        (true, false)
    } else {
        list::sides(&e.relations)
    };
    (e.parent, e.rule) = match (adduct, loss) {
        (true, false) => (Some(e.a), "lighter"),
        (false, true) => (Some(e.b), "heavier"),
        _ => {
            let (ha, hb) = evidence::total_heights(t, e.a, e.b);
            (Some(if hb > ha { e.b } else { e.a }), "intensity")
        }
    };
}

/// `companion_of` of every row: the parent of the edge with the best Pearson (a tie goes to the smaller RT offset, then to the lower row).
fn choose_parents(f: &mut Found) {
    let mut best: Vec<Option<(f64, f64, usize, Kind)>> = vec![None; f.companion_of.len()];
    for e in &f.edges {
        let Some(par) = e.parent else { continue };
        let child = if par == e.a { e.b } else { e.a };
        let key = (e.ev.r, -e.ev.rt_offset.abs());
        let better = best[child].is_none_or(|(r, o, q, _)| {
            key.0 > r || (key.0 == r && (key.1 > o || (key.1 == o && par < q)))
        });
        if better {
            best[child] = Some((key.0, key.1, par, e.kind));
        }
    }
    for (c, b) in f.companion_of.iter_mut().zip(best) {
        *c = b.map(|(_, _, par, kind)| (par, kind));
    }
}
