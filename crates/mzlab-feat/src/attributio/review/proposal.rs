// Ported from Attributio src/attributio/review/build.py (the record of a feature: tier, reasons, candidate structures and the evidence behind them).
//! The proposal of the review for one feature: `class` (the tier, `3-keep` … `2a->3`), the level it proposes, the reasons, the structure it rests on, the
//! other candidate structures and the evidence (explained fragments, precursor in the MS2, raw proofs, references, purity). It is a sorting aid for the
//! chemist, never a verdict. `Proposal::to_json` writes it for the page (no serde: the engine stays light).

use super::purity::{CandidateScores, Impossible};
use super::raw::Ms1Metrics;

/// The structure the identity of the feature rests on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Best {
    /// `CSI:FingerID`, `library` or `formula` (no structure, only a formula).
    pub structure_from: String,
    pub formula: Option<String>,
    pub smiles: Option<String>,
    pub ik1: Option<String>,
    pub adduct: Option<String>,
    pub ion_formula: Option<String>,
    pub ion_ppm: Option<f64>,
}

/// Another candidate structure of the feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Alternative {
    pub label: String,
    pub formula: Option<String>,
    pub adduct: Option<String>,
    pub ppm: Option<f64>,
    pub n_expl: usize,
    pub frac: f64,
    pub fits: bool,
    pub ik1: Option<String>,
}

/// A hit of one of the pipeline tools on a reference.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolRef {
    pub tool: String,
    pub kind: String,
    pub score: Option<f64>,
    pub shared: Option<f64>,
    pub ref_id: Option<String>,
}

/// A reference spectrum (library or tool hit) with its scores against the experimental one.
#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    /// `library`, `library-analog` or `tool`.
    pub origin: String,
    pub library: Option<String>,
    pub entry: Option<String>,
    pub name: String,
    pub ik1: Option<String>,
    pub same: bool,
    pub smiles: Option<String>,
    pub prec: Option<f64>,
    pub adduct: Option<String>,
    /// `exact` or `analog`.
    pub kind: String,
    pub dm: Option<f64>,
    pub dm_ppm: Option<f64>,
    pub dm_formulas: Vec<String>,
    pub tools: Vec<ToolRef>,
    pub cos: Option<f64>,
    pub cos_vis: Option<f64>,
    pub matched: i64,
    pub matched_vis: i64,
    pub n_shifted: i64,
    /// `false` when the spectrum of the reference is not in the libraries (its USI only).
    pub available: bool,
    pub scores: Option<CandidateScores>,
}

/// The raw proofs of the best MS2 scan.
#[derive(Clone, Debug, PartialEq)]
pub struct RawEvidence {
    pub file: String,
    pub scan: i64,
    pub low: f64,
    pub high: f64,
    pub target: f64,
    pub lo: f64,
    pub hi: f64,
    pub ms1: Option<Ms1Metrics>,
}

/// Isolation-window purity, co-elution, unexplained signal and the three-state badge.
#[derive(Clone, Debug, PartialEq)]
pub struct PurityEvidence {
    pub purity: Option<f64>,
    pub prec_int: Option<f64>,
    /// `(m/z, relative intensity, kind, apex offset, r)` of every co-isolated ion.
    #[allow(clippy::type_complexity)]
    pub co: Vec<(f64, f64, String, Option<f64>, Option<f64>)>,
    pub unexplained: Option<f64>,
    pub imposs: Vec<Impossible>,
    pub badge: String,
    pub badge_reasons: Vec<String>,
    pub low_information: Option<bool>,
}

/// A fragment of the experimental spectrum explained by a sub-formula of the ion.
#[derive(Clone, Debug, PartialEq)]
pub struct FragView {
    pub formula: String,
    pub mz_th: f64,
    pub ppm: f64,
    pub loss: String,
}

/// A peak of the experimental spectrum, intensity in % of the base peak.
#[derive(Clone, Debug, PartialEq)]
pub struct PeakView {
    pub mz: f64,
    pub rel: f64,
    pub frag: Option<FragView>,
}

/// The reference spectrum drawn against the experimental one: its peaks and the matched pairs `(experimental index, reference index, shifted)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Mirror {
    /// Position in `Evidence::references`.
    pub reference: usize,
    pub peaks: Vec<(f64, f64)>,
    pub pairs: Vec<(usize, usize, bool)>,
}

/// What the charts of the Banco 2b need: peaks with their fragments, the mirror and the isotope pattern `(m/z, % of the monoisotopic)` of the ion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub peaks: Vec<PeakView>,
    pub mirror: Option<Mirror>,
    pub isotopes: Vec<(f64, f64)>,
}

/// What the proposal rests on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Evidence {
    pub n_peaks: usize,
    pub n_raw_peaks: usize,
    pub n_explained: usize,
    pub frac_explained: f64,
    pub decoy_frac: f64,
    pub precursor_present: bool,
    pub precursor_rel: Option<f64>,
    pub raw: Option<RawEvidence>,
    pub references: Vec<Reference>,
    pub purity: Option<PurityEvidence>,
    /// The peaks, the fragments and the mirror spectrum that the Banco 2b draws.
    pub view: Option<View>,
}

/// The proposal of the review for one feature.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub pol: String,
    pub id: i64,
    /// The tier: `1`, `2a-robust`, `2a-unverified`, `2a->2b`, `2a->3`, `2b-strong`, `2b-check` or `3-keep`.
    pub class: String,
    /// The level the tier proposes (`1`, `2a`, `2b` or `3`).
    pub proposed: String,
    pub reasons: Vec<String>,
    pub best: Best,
    pub alternatives: Vec<Alternative>,
    pub evidence: Evidence,
}

// ------------------------------------------------------------------------------ JSON

/// A JSON value (written by hand: `serde_json` is a dev-dependency only).
#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    /// A JSON text already written, put in as it is.
    Raw(String),
    Arr(Vec<J>),
    Obj(Vec<(&'static str, J)>),
}

impl J {
    pub fn s(v: &str) -> J {
        J::Str(v.to_string())
    }
    pub fn strs(v: &[String]) -> J {
        J::Arr(v.iter().map(|s| J::s(s)).collect())
    }
    pub fn os(v: &Option<String>) -> J {
        v.as_ref().map_or(J::Null, |s| J::s(s))
    }
    pub fn on(v: Option<f64>) -> J {
        v.filter(|x| x.is_finite()).map_or(J::Null, J::Num)
    }
    /// The JSON text.
    pub fn write(&self, out: &mut String) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            J::Num(x) if !x.is_finite() => out.push_str("null"),
            J::Num(x) if x.fract() == 0.0 && x.abs() < 1e15 => {
                out.push_str(&format!("{}", *x as i64))
            }
            J::Num(x) => out.push_str(&format!("{x}")),
            J::Str(s) => Self::write_str(s, out),
            J::Raw(s) => out.push_str(s),
            J::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    x.write(out);
                }
                out.push(']');
            }
            J::Obj(v) => {
                out.push('{');
                for (i, (k, x)) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    Self::write_str(k, out);
                    out.push(':');
                    x.write(out);
                }
                out.push('}');
            }
        }
    }
    fn write_str(s: &str, out: &mut String) {
        out.push('"');
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
    }
}

impl Best {
    fn json(&self) -> J {
        J::Obj(vec![
            ("structure_from", J::s(&self.structure_from)),
            ("formula", J::os(&self.formula)),
            ("smiles", J::os(&self.smiles)),
            ("ik1", J::os(&self.ik1)),
            ("adduct", J::os(&self.adduct)),
            ("ion_formula", J::os(&self.ion_formula)),
            ("ion_ppm", J::on(self.ion_ppm)),
        ])
    }
}

impl Alternative {
    fn json(&self) -> J {
        J::Obj(vec![
            ("label", J::s(&self.label)),
            ("formula", J::os(&self.formula)),
            ("adduct", J::os(&self.adduct)),
            ("ppm", J::on(self.ppm)),
            ("n_expl", J::Num(self.n_expl as f64)),
            ("frac", J::Num(self.frac)),
            ("fits", J::Bool(self.fits)),
            ("ik1", J::os(&self.ik1)),
        ])
    }
}

impl Reference {
    fn json(&self) -> J {
        let tools = self
            .tools
            .iter()
            .map(|t| {
                J::Obj(vec![
                    ("tool", J::s(&t.tool)),
                    ("kind", J::s(&t.kind)),
                    ("score", J::on(t.score)),
                    ("shared", J::on(t.shared)),
                    ("ref_id", J::os(&t.ref_id)),
                ])
            })
            .collect();
        let q = self.scores.as_ref().map_or(J::Null, |q| {
            J::Obj(vec![
                ("fwd", J::Num(q.fwd)),
                ("rev", J::Num(q.rev)),
                ("presence", J::Num(q.presence)),
                ("n_main", J::Num(q.n_main as f64)),
                ("cos_expl", J::Num(q.cos_expl)),
                ("matched_expl", J::Num(q.matched_expl as f64)),
                ("informative", J::Num(q.informative as f64)),
                ("p_chance", J::Num(q.p_chance)),
                ("matched", J::Num(q.matched as f64)),
            ])
        });
        J::Obj(vec![
            ("origin", J::s(&self.origin)),
            ("library", J::os(&self.library)),
            ("entry", J::os(&self.entry)),
            ("name", J::s(&self.name)),
            ("ik1", J::os(&self.ik1)),
            ("same", J::Bool(self.same)),
            ("smiles", J::os(&self.smiles)),
            ("prec", J::on(self.prec)),
            ("adduct", J::os(&self.adduct)),
            ("kind", J::s(&self.kind)),
            ("dm", J::on(self.dm)),
            ("dm_ppm", J::on(self.dm_ppm)),
            ("dm_formulas", J::strs(&self.dm_formulas)),
            ("tools", J::Arr(tools)),
            ("cos", J::on(self.cos)),
            ("cos_vis", J::on(self.cos_vis)),
            ("matched", J::Num(self.matched as f64)),
            ("matched_vis", J::Num(self.matched_vis as f64)),
            ("n_shifted", J::Num(self.n_shifted as f64)),
            ("available", J::Bool(self.available)),
            ("scores", q),
        ])
    }
}

impl RawEvidence {
    fn json(&self) -> J {
        let m = self.ms1.as_ref().map_or(J::Null, |m| {
            J::Obj(vec![
                ("iso_ok", m.iso_ok.map_or(J::Null, J::Bool)),
                ("obs1", J::Num(m.obs1)),
                ("th1", J::Num(m.th1)),
                ("obs2", J::Num(m.obs2)),
                ("th2", J::Num(m.th2)),
                ("chim_n", J::Num(m.chim_n as f64)),
                ("chim_max", J::Num(m.chim_max)),
                (
                    "chim",
                    J::Arr(
                        m.chim
                            .iter()
                            .map(|c| J::Arr(vec![J::Num(c.0), J::Num(c.1)]))
                            .collect(),
                    ),
                ),
            ])
        });
        J::Obj(vec![
            ("file", J::s(&self.file)),
            ("scan", J::Num(self.scan as f64)),
            ("low", J::Num(self.low)),
            ("high", J::Num(self.high)),
            ("target", J::Num(self.target)),
            ("lo", J::Num(self.lo)),
            ("hi", J::Num(self.hi)),
            ("ms1", m),
        ])
    }
}

impl PurityEvidence {
    fn json(&self) -> J {
        let co = self
            .co
            .iter()
            .map(|c| {
                J::Obj(vec![
                    ("mz", J::Num(c.0)),
                    ("rel", J::Num(c.1)),
                    ("kind", J::s(&c.2)),
                    ("offset", J::on(c.3)),
                    ("r", J::on(c.4)),
                ])
            })
            .collect();
        let imp = self
            .imposs
            .iter()
            .map(|i| {
                J::Obj(vec![
                    ("mz", J::Num(i.mz)),
                    ("rel", J::Num(i.rel)),
                    ("loss", J::Num(i.loss)),
                    ("why", J::s(i.why)),
                ])
            })
            .collect();
        J::Obj(vec![
            ("purity", J::on(self.purity)),
            ("prec_int", J::on(self.prec_int)),
            ("co", J::Arr(co)),
            ("unexplained", J::on(self.unexplained)),
            ("imposs", J::Arr(imp)),
            ("badge", J::s(&self.badge)),
            ("badge_reasons", J::strs(&self.badge_reasons)),
            (
                "low_information",
                self.low_information.map_or(J::Null, J::Bool),
            ),
        ])
    }
}

impl Evidence {
    fn json(&self) -> J {
        J::Obj(vec![
            ("n_peaks", J::Num(self.n_peaks as f64)),
            ("n_raw_peaks", J::Num(self.n_raw_peaks as f64)),
            ("n_explained", J::Num(self.n_explained as f64)),
            ("frac_explained", J::Num(self.frac_explained)),
            ("decoy_frac", J::Num(self.decoy_frac)),
            ("precursor_present", J::Bool(self.precursor_present)),
            ("precursor_rel", J::on(self.precursor_rel)),
            ("raw", self.raw.as_ref().map_or(J::Null, RawEvidence::json)),
            (
                "references",
                J::Arr(self.references.iter().map(Reference::json).collect()),
            ),
            (
                "purity",
                self.purity.as_ref().map_or(J::Null, PurityEvidence::json),
            ),
            ("view", self.view.as_ref().map_or(J::Null, View::json)),
        ])
    }
}

impl Proposal {
    /// The proposal as a JSON value.
    pub fn json(&self) -> J {
        J::Obj(vec![
            ("pol", J::s(&self.pol)),
            ("id", J::Num(self.id as f64)),
            ("class", J::s(&self.class)),
            ("proposed", J::s(&self.proposed)),
            ("reasons", J::strs(&self.reasons)),
            ("best", self.best.json()),
            (
                "alternatives",
                J::Arr(self.alternatives.iter().map(Alternative::json).collect()),
            ),
            ("evidence", self.evidence.json()),
        ])
    }

    /// The proposal as JSON text.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        self.json().write(&mut s);
        s
    }
}
