//! `features.group@1`: groups the rows of a per-ion table that are one compound (isotopologues, adducts, multimers) without merging them: every row
//! stays, with a group id, its ion species and neutral mass when the evidence gives one, plus the edges (`ionnet/1`) that a molecular network reads.
//! Correlation from the table (`pairs`), then isotopes and ion identity networking (`net`); bytes in, text out for `flow run` and the `flowtab` `.wasm`.

pub mod ions;
mod net;
mod pairs;

use crate::store::{origin_from_meta, read_table};
use crate::table::Aligned;
use mzlab_core::error::Result;
use mzlab_core::json::{Out, Value};

pub use ions::Ion;

/// The parameters of the step (descriptor `steps::group`).
#[derive(Debug, Clone)]
pub struct Params {
    pub rt_tol: f64,
    pub apex_fwhm: f64,
    pub min_corr: f64,
    pub min_corr_samples: usize,
    pub ppm: f64,
    pub mz_abs: f64,
    pub max_charge: i8,
    pub ions: Vec<Ion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    Ion,
    Isotope,
    Corr,
}

impl EdgeKind {
    /// The `EdgeType` of the GNPS supplementary pairs.
    pub fn gnps(self) -> &'static str {
        match self {
            EdgeKind::Ion => "MS1 annotation",
            EdgeKind::Isotope => "MS1 isotope",
            EdgeKind::Corr => "MS1 correlation",
        }
    }
}

/// An edge between rows `a < b` (row indices), with the Pearson of the heights (NaN = not tested) and its note.
#[derive(Debug, Clone)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub kind: EdgeKind,
    pub score: f64,
    pub note: String,
}

/// The result: one value per row, and the edges.
#[derive(Debug, Clone, Default)]
pub struct Grouping {
    pub group: Vec<u32>,
    pub corr_group: Vec<u32>,
    pub ion: Vec<Option<usize>>,
    pub neutral: Vec<f64>,
    pub charge: Vec<i8>,
    pub iso: Vec<u8>,
    pub edges: Vec<Edge>,
}

/// Groups the rows of `t` (any row count; a family-mode table is read by its reference ions).
pub fn group(t: &Aligned, p: &Params) -> Grouping {
    let n = t.len();
    let corr = pairs::all(t, p);
    let (mut iso, mut charge) = (vec![0u8; n], vec![0i8; n]);
    let iso_edges = net::isotopes(t, p, &corr, &mut iso, &mut charge);
    let (mut ion, mut neutral) = (vec![None; n], vec![f64::NAN; n]);
    let ion_edges = net::identities(t, p, &corr, &iso, &charge, &mut ion, &mut neutral);
    let group = net::components(n, iso_edges.iter().chain(&ion_edges).map(|e| (e.a, e.b)));
    let corr_group = net::components(n, corr.iter().map(|c| (c.a, c.b)));
    let mut edges: Vec<Edge> = ion_edges.into_iter().chain(iso_edges).collect();
    for c in &corr {
        if !edges
            .iter()
            .any(|e| (e.a.min(e.b), e.a.max(e.b)) == (c.a, c.b))
        {
            edges.push(Edge {
                a: c.a,
                b: c.b,
                kind: EdgeKind::Corr,
                score: c.r,
                note: String::new(),
            });
        }
    }
    for e in &mut edges {
        (e.a, e.b) = (e.a.min(e.b), e.a.max(e.b));
    }
    crate::sortc::any_by(&mut edges, |x, y| {
        x.a.cmp(&y.a).then(x.b.cmp(&y.b)).then(x.kind.cmp(&y.kind))
    });
    Grouping {
        group,
        corr_group,
        ion,
        neutral,
        charge,
        iso,
        edges,
    }
}

fn num(params: &[(String, Value)], k: &str, d: f64) -> f64 {
    params
        .iter()
        .find(|(n, _)| n == k)
        .and_then(|(_, v)| v.f64())
        .unwrap_or(d)
}

fn text<'a>(params: &'a [(String, Value)], k: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(n, _)| n == k)
        .and_then(|(_, v)| v.str())
}

/// The parameters of the step from the canonical parameters of the plan; `polarity` = the one of the files when the plan says `auto`.
pub fn params_of(params: &[(String, Value)], file_polarity: i8) -> Result<(Params, bool)> {
    let positive = match text(params, "polarity") {
        Some("positive") => true,
        Some("negative") => false,
        _ => file_polarity >= 0,
    };
    let list = if positive {
        text(params, "ions_pos").unwrap_or(ions::DEFAULT_POS)
    } else {
        text(params, "ions_neg").unwrap_or(ions::DEFAULT_NEG)
    };
    let p = Params {
        rt_tol: num(params, "rt_tol", 0.1),
        apex_fwhm: num(params, "apex_fwhm", 0.6),
        min_corr: num(params, "min_corr", 0.75),
        min_corr_samples: num(params, "min_corr_samples", 3.0).max(2.0) as usize,
        ppm: num(params, "ppm", 5.0),
        mz_abs: num(params, "mz_abs", 0.001),
        max_charge: num(params, "max_charge", 2.0).clamp(1.0, 3.0) as i8,
        ions: ions::parse_list(list, positive)?,
    };
    Ok((p, positive))
}

/// The `ionnet/1` text of the `.mzfeat` `table` under the parameters of the step.
pub fn group_text(params: &[(String, Value)], table: &[u8]) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let pol: i64 = origin_from_meta(&meta)
        .facts
        .iter()
        .map(|f| i64::from(f.polarity.signum()))
        .sum();
    let (p, positive) = params_of(params, pol.signum() as i8)?;
    Ok(to_json(&t, &p, positive, &group(&t, &p)))
}

fn opt_num(o: &mut Out, v: f64, nd: usize) {
    if v.is_finite() {
        o.num(mzlab_core::json::round(v, nd));
    } else {
        o.raw("null");
    }
}

/// `ionnet/1`: columns of one value per row (row `id` of the table), then the edges with the column names of the GNPS supplementary pairs.
pub fn to_json(t: &Aligned, p: &Params, positive: bool, g: &Grouping) -> String {
    let n = t.len();
    let groups = g.group.iter().copied().max().unwrap_or(0);
    let mut o = Out::default();
    o.raw("{\"format\":\"ionnet/1\",\"polarity\":")
        .int(if positive { 1 } else { -1 });
    o.raw(",\"rows\":")
        .int(n as i64)
        .raw(",\"groups\":")
        .int(i64::from(groups))
        .raw(",\"ions\":[");
    for (k, i) in p.ions.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(i.name);
    }
    let ints = |o: &mut Out, key: &str, v: &mut dyn Iterator<Item = i64>| {
        o.raw(",\"").raw(key).raw("\":[");
        for (k, x) in v.enumerate() {
            o.raw(if k > 0 { "," } else { "" }).int(x);
        }
        o.raw("]");
    };
    o.raw("]");
    ints(&mut o, "id", &mut (0..n).map(|r| i64::from(t.id(r))));
    ints(&mut o, "group", &mut g.group.iter().map(|&x| i64::from(x)));
    ints(
        &mut o,
        "corr_group",
        &mut g.corr_group.iter().map(|&x| i64::from(x)),
    );
    ints(
        &mut o,
        "charge",
        &mut g.charge.iter().map(|&x| i64::from(x)),
    );
    ints(&mut o, "iso", &mut g.iso.iter().map(|&x| i64::from(x)));
    o.raw(",\"ion\":[");
    for (k, i) in g.ion.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        match i {
            Some(i) => o.str(p.ions[*i].name),
            None => o.raw("null"),
        };
    }
    o.raw("],\"neutral\":[");
    for (k, &m) in g.neutral.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        opt_num(&mut o, m, 6);
    }
    o.raw("],\"edges\":[");
    for (k, e) in g.edges.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"ID1\":")
            .int(i64::from(t.id(e.a)));
        o.raw(",\"ID2\":")
            .int(i64::from(t.id(e.b)))
            .raw(",\"EdgeType\":")
            .str(e.kind.gnps());
        o.raw(",\"Score\":");
        opt_num(&mut o, e.score, 4);
        o.raw(",\"Annotation\":").str(&e.note).raw("}");
    }
    o.raw("]}");
    o.0
}
