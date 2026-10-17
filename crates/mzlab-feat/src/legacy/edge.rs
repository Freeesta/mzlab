//! The evidence on the link between two co-eluting ions as mzFinder writes it (`edge_evidence`, with its
//! texts, in f64) and the tables of mzFinder it reads (`relations`, `dimer_of`). `family::evidence` is the same reading for the pipeline (f32,
//! without texts); this one gives the JSON of `api.identity_graph`.

use mzlab_core::elements::{mass, mass_of, parse_formula, ELECTRON};
use mzlab_core::json::Out;

mod report;
mod stats;
pub use report::origin_evidence;
pub use stats::*;

pub const ABS_DA: f64 = 0.002;
pub const PPM: f64 = 5.0;
const COELUTION_MIN: f64 = 0.05;
const CV_STABLE: f64 = 0.30;
const MIN_SAMPLES: usize = 3;
const SKELETON_MIN: f64 = 0.5;
const MAX_PEAKS: usize = 40;
const NO_FRAG_ADDUCTS: [&str; 3] = ["[M+Na]+", "[M+K]+", "[M+Na-2H]-"];

/// `iimn.ISOTOPES`.
pub const ISOTOPES: [(&str, f64); 3] =
    [("13C", 1.003355), ("34S/13C2", 1.99580), ("13C2", 2.00671)];
/// `iimn.LOSSES`: `ionfamily.NEUTRAL_LOSSES`, then the `EXTRA_LOSSES` not already there.
const LOSS_NAMES: [&str; 23] = [
    "H2O", "NH3", "CO", "CO2", "CH2O", "CH4O", "C2H2O", "HCOOH", "C2H4", "C3H6", "HCN", "HF",
    "HCl", "HBr", "SO2", "SO3", "H2S", "CH3", "NO2", "C6H6", "C4H8", "C4H9N", "C4H8O",
];

fn m(e: &str) -> f64 {
    mass_of(e).expect("fixed element")
}

fn neutral(f: &str) -> f64 {
    mass(&parse_formula(f).expect("fixed formula"))
}

/// `E.PROTON`.
pub fn proton() -> f64 {
    m("H") - ELECTRON
}

/// The tables of `iimn`, built once.
pub struct Iimn {
    pub proton: f64,
    /// `ADDUCT_FROM_H` (index 0) and `ADDUCT_FROM_DEPROT` (index 1).
    pub adducts: [Vec<(&'static str, f64)>; 2],
    pub losses: Vec<(&'static str, f64)>,
    pub nh3: f64,
}

impl Iimn {
    pub fn new() -> Iimn {
        let p = proton();
        Iimn {
            proton: p,
            adducts: [
                vec![
                    ("[M+Na]+", m("Na") - ELECTRON - p),
                    ("[M+K]+", m("K") - ELECTRON - p),
                    ("[M+NH4]+", m("N") + 4.0 * m("H") - ELECTRON - p),
                    ("[M+H+ACN]+", neutral("C2H3N")),
                ],
                vec![
                    ("[M+FA-H]-", neutral("CH2O2")),
                    ("[M+Cl]-", m("Cl") + ELECTRON + p),
                    ("[M+Na-2H]-", m("Na") - ELECTRON - p),
                ],
            ],
            losses: LOSS_NAMES.iter().map(|&n| (n, neutral(n))).collect(),
            nh3: neutral("NH3"),
        }
    }

    pub fn adducts_of(&self, pol: i32) -> &[(&'static str, f64)] {
        &self.adducts[if pol > 0 { 0 } else { 1 }]
    }
}

impl Default for Iimn {
    fn default() -> Self {
        Iimn::new()
    }
}

/// One hypothesis of `iimn.relations`: role, name, `of` (true = "x") and the signed step.
#[derive(Clone, Debug)]
pub struct Rel {
    pub role: &'static str,
    pub name: String,
    pub of_x: bool,
    pub delta: f64,
}

/// `iimn.relations(delta, mz_ref, ppm, abs_da, adducts)`.
pub fn relations(t: &Iimn, delta: f64, mz_ref: f64, ppm: f64, abs_da: f64, pol: i32) -> Vec<Rel> {
    let tol = (mz_ref * ppm * 1e-6).max(abs_da);
    let mut out = Vec::new();
    let mut push = |role, name: &str, of_x, d| {
        out.push(Rel {
            role,
            name: name.to_string(),
            of_x,
            delta: d,
        })
    };
    for &(name, d) in t.adducts_of(pol) {
        if (delta - d).abs() <= tol {
            push("adduct", name, true, d);
        }
        if (delta + d).abs() <= tol {
            push("adduct", name, false, -d);
        }
    }
    for &(name, d) in ISOTOPES.iter() {
        if (delta - d).abs() <= tol {
            push("isotope", name, true, d);
        }
        if (delta + d).abs() <= tol {
            push("isotope", name, false, -d);
        }
    }
    for &(name, d) in &t.losses {
        if (delta + d).abs() <= tol {
            push("loss", name, true, -d);
        }
        if (delta - d).abs() <= tol {
            push("loss", name, false, d);
        }
    }
    out
}

/// `iimn.dimer_of(mz_x, mz_y, ppm, abs_da, sign)`.
pub fn dimer_of(t: &Iimn, mz_x: f64, mz_y: f64, ppm: f64, abs_da: f64, sign: i32) -> bool {
    let s = sign as f64;
    let tol = (mz_y * ppm * 1e-6).max(abs_da);
    (mz_y - (2.0 * (mz_x - s * t.proton) + s * t.proton)).abs() <= tol
}

/// One reading of `relations_of`.
struct Reading {
    role: &'static str,
    name: String,
    ppm: f64,
    step: f64,
}

fn relations_of(t: &Iimn, mz_lo: f64, mz_up: f64, pol: i32) -> Vec<Reading> {
    let delta = mz_up - mz_lo;
    let mut out = Vec::new();
    for r in relations(t, delta, mz_lo, PPM, ABS_DA, pol) {
        if (r.of_x && (r.role == "adduct" || r.role == "isotope")) || (!r.of_x && r.role == "loss")
        {
            out.push(Reading {
                role: r.role,
                name: r.name,
                ppm: (delta - r.delta) / mz_lo * 1e6,
                step: r.delta,
            });
        }
    }
    if dimer_of(t, mz_lo, mz_up, PPM, ABS_DA, pol) {
        let name = if pol > 0 { "[2M+H]+" } else { "[2M-H]-" };
        out.push(Reading {
            role: "dimer",
            name: name.into(),
            ppm: 0.0,
            step: delta,
        });
    }
    out
}

/// One piece of evidence: key, text, strength and the extra fields already written as `,"k":v` JSON.
struct Ev {
    key: &'static str,
    text: String,
    strength: &'static str,
    extra: String,
}

fn ev(key: &'static str, text: String, strength: &'static str, extra: String) -> Ev {
    Ev {
        key,
        text,
        strength,
        extra,
    }
}

fn kv(k: &str, v: f64) -> String {
    let mut o = Out::default();
    o.raw(",\"").raw(k).raw("\":").num(v);
    o.0
}

/// The inputs of one edge.
pub struct EdgeIn<'a> {
    pub mz_lo: f64,
    pub mz_up: f64,
    pub pol: i32,
    pub ms2_lo: Option<&'a Spec>,
    pub ms2_up: Option<&'a Spec>,
    pub areas: Option<(&'a [f64], &'a [f64])>,
    pub drt_min: Option<f64>,
}

/// What the evidence found, for the label.
#[derive(Default)]
struct Found {
    pos: bool,
    neg: bool,
    tl_hit: bool,
    frag_in: bool,
    stable: bool,
}

fn step_evidence(e: &EdgeIn, rels: &[Reading], out: &mut Vec<Ev>) {
    for r in rels.iter().filter(|r| r.role != "isotope") {
        let mut x = Out::default();
        x.raw(",\"name\":")
            .str(&r.name)
            .raw(",\"role\":")
            .str(r.role);
        let text = format!(
            "Δm/z {:.4} = {} {} ({:+.1} ppm)",
            e.mz_up - e.mz_lo,
            r.role,
            r.name,
            r.ppm
        );
        out.push(ev("dm", text, "info", x.0 + &kv("ppm", r.ppm)));
    }
    if let Some(d) = e.drt_min {
        let within = d <= COELUTION_MIN;
        let text = format!(
            "ΔRT {:.1} s ({} {COELUTION_MIN} min)",
            d * 60.0,
            if within { "within" } else { "beyond" }
        );
        out.push(ev(
            "coelution",
            text,
            if within { "evidence" } else { "against" },
            kv("drt_min", d),
        ));
    }
}

fn isf_evidence(e: &EdgeIn, s_up: Option<&Spec>, loss: bool, out: &mut Vec<Ev>, f: &mut Found) {
    if loss {
        match s_up {
            None => out.push(ev(
                "frag_ms2",
                "no MS2 of the heavier ion: its fragments are unknown".into(),
                "info",
                String::new(),
            )),
            Some(s) => match near(s, e.mz_lo, Some(e.mz_up)) {
                Some((a, rel)) => {
                    f.frag_in = true;
                    let text = format!(
                        "m/z {:.4} is a fragment in the MS2 of {:.4} ({:+.1} ppm, {:.0} % of the base peak)",
                        e.mz_lo,
                        e.mz_up,
                        (a - e.mz_lo) / e.mz_lo * 1e6,
                        rel
                    );
                    out.push(ev("frag_ms2", text, "evidence", kv("rel", rel)));
                }
                None => out.push(ev(
                    "frag_ms2",
                    format!(
                        "m/z {:.4} is not among the fragments of the MS2 of {:.4}",
                        e.mz_lo, e.mz_up
                    ),
                    "against",
                    String::new(),
                )),
            },
        }
    }
    if let Some((lo, up)) = e.areas {
        let rs = ratio_stats(lo, up);
        let head = format!("ratio {:.4}/{:.4}", e.mz_up, e.mz_lo);
        match rs.cv {
            None => out.push(ev(
                "ratio",
                format!("{head}: fewer than {MIN_SAMPLES} common samples"),
                "info",
                format!(",\"n\":{}", rs.n),
            )),
            Some(cv) => {
                f.stable = cv <= CV_STABLE;
                let text = format!(
                    "{head} {}: CV {:.0} % over {} samples",
                    if f.stable { "stable" } else { "not stable" },
                    100.0 * cv,
                    rs.n
                );
                let st = if f.stable { "evidence" } else { "against" };
                out.push(ev(
                    "ratio",
                    text,
                    st,
                    kv("cv", cv) + &format!(",\"n\":{}", rs.n),
                ));
            }
        }
    }
}

fn adduct_evidence(
    t: &Iimn,
    e: &EdgeIn,
    name: &str,
    s: (Option<&Spec>, Option<&Spec>),
    out: &mut Vec<Ev>,
    f: &mut Found,
) {
    let (s_lo, s_up) = s;
    if let (Some(lo), Some(up)) = (s_lo, s_up) {
        let (n, sh, frac) = skeleton(lo, up, e.mz_lo, e.mz_up);
        match frac {
            None => out.push(ev(
                "skeleton",
                "the lighter ion has no fragment besides its own precursor".into(),
                "info",
                String::new(),
            )),
            Some(fr) if fr >= SKELETON_MIN => {
                f.pos = true;
                let text = format!(
                    "common skeleton: {sh} of {n} fragments of {:.4} also in {:.4} (same m/z or shifted by Δm/z)",
                    e.mz_lo, e.mz_up
                );
                out.push(ev("skeleton", text, "evidence", kv("frac", fr)));
            }
            Some(fr) => {
                f.neg = true;
                let text = format!(
                    "no common skeleton: {sh} of {n} fragments of {:.4} also in {:.4}",
                    e.mz_lo, e.mz_up
                );
                out.push(ev("skeleton", text, "against", kv("frac", fr)));
            }
        }
    }
    if let (true, Some(up)) = (name == "[M+NH4]+", s_up) {
        let lm = t.nh3;
        if near(up, e.mz_up - lm, Some(e.mz_up)).is_some() {
            (f.pos, f.neg, f.tl_hit) = (true, false, true);
            let text = format!(
                "typical loss of {name}: -{lm:.3} (NH3) seen in the MS2 of {:.4}",
                e.mz_up
            );
            out.push(ev("typical_loss", text, "evidence", String::new()));
        } else {
            let text = format!(
                "typical loss of {name} (-{lm:.3}, NH3) not seen in the MS2 of {:.4}",
                e.mz_up
            );
            out.push(ev("typical_loss", text, "against", String::new()));
            f.neg = f.neg || s_lo.is_none();
        }
    }
    if let (true, Some(up)) = (NO_FRAG_ADDUCTS.contains(&name), s_up) {
        if peaks_besides(up, e.mz_up) == 0 {
            let text = format!("{name} does not fragment in its MS2 (a hint, not a proof)");
            out.push(ev("no_fragmentation", text, "hint", String::new()));
        }
    }
}

/// `edge_evidence(...)` as JSON `{kind, label, text, relations, evidence, summary}`.
pub fn edge_evidence(t: &Iimn, e: &EdgeIn, o: &mut Out) {
    let rels = relations_of(t, e.mz_lo, e.mz_up, e.pol);
    let adduct = rels
        .iter()
        .find(|r| r.role == "adduct" || r.role == "dimer");
    let losses: Vec<&Reading> = rels.iter().filter(|r| r.role == "loss").collect();
    let mut evs = Vec::new();
    let mut f = Found::default();
    let kind = if adduct.is_none() && losses.is_empty() {
        "other"
    } else {
        let (s_lo, s_up) = (e.ms2_lo, e.ms2_up);
        step_evidence(e, &rels, &mut evs);
        isf_evidence(e, s_up, !losses.is_empty(), &mut evs, &mut f);
        let mut isf_ok = !losses.is_empty() && f.frag_in && f.stable;
        if let Some(a) = adduct {
            adduct_evidence(t, e, &a.name, (s_lo, s_up), &mut evs, &mut f);
        }
        if f.tl_hit && losses.iter().all(|r| r.name == "NH3") {
            isf_ok = false;
        }
        match (adduct.is_some(), !losses.is_empty()) {
            (true, false) if f.neg && !f.pos => "undetermined",
            (true, false) => "adduct",
            (false, true) if isf_ok => "isf",
            (false, true) => "undetermined",
            _ if f.pos && !isf_ok => "adduct",
            _ if isf_ok && !f.pos => "isf",
            _ => "undetermined",
        }
    };
    write_edge(kind, &rels, &evs, o);
}

fn write_edge(kind: &str, rels: &[Reading], evs: &[Ev], o: &mut Out) {
    let label = match kind {
        "adduct" => "adduct",
        "isf" => "in-source fragment?",
        "undetermined" => "not distinguishable",
        _ => "",
    };
    let shown: Vec<&str> = evs
        .iter()
        .filter(|e| e.key != "coelution" || e.strength == "against")
        .map(|e| e.text.as_str())
        .collect();
    let summary = shown.join("; ");
    let text = match kind {
        "other" => String::new(),
        "undetermined" if shown.is_empty() => label.to_string(),
        "undetermined" => format!("{label} ({summary})"),
        _ => format!("{label} (evidence: {summary})"),
    };
    o.raw("{\"kind\":").str(kind).raw(",\"label\":").str(label);
    o.raw(",\"text\":").str(&text).raw(",\"relations\":[");
    for (i, r) in rels.iter().enumerate() {
        o.raw(if i > 0 { ",{\"role\":" } else { "{\"role\":" })
            .str(r.role);
        o.raw(",\"name\":").str(&r.name).raw(",\"ppm\":").num(r.ppm);
        o.raw(",\"step\":").num(r.step).raw("}");
    }
    o.raw("],\"evidence\":[");
    for (i, e) in evs.iter().enumerate() {
        o.raw(if i > 0 { ",{\"key\":" } else { "{\"key\":" })
            .str(e.key);
        o.raw(",\"text\":")
            .str(&e.text)
            .raw(",\"strength\":")
            .str(e.strength);
        o.raw(&e.extra).raw("}");
    }
    o.raw("],\"summary\":").str(&summary).raw("}");
}
