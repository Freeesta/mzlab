//! Hypotheses of the targeted pass: the ions that the chemistry says could be there, each with the label of the source that proposed it.
//! Sources: (a) the parent times the photocatalytic rules, one or two steps (`rules`, data in `TP_Mine/data/regole_tp.json`) and cleavages given as formulas;
//! (b) the offline suspect index; (c) the expected members of the families already found; (d) lists imported or typed by the user.
//! One pass, no loop to convergence: the order is fixed (mass, then label), so two runs give the same `Registry` byte for byte.
//! A hypothesis is a place to look, never a verdict: what it finds is graded by `target`.

mod rules;
mod sources;

use std::collections::BTreeMap;

use mzlab_core::elements::{self, Formula};
use mzlab_core::error::Result;
use mzlab_core::json::{round, Out};

pub use rules::{
    parse_delta, Counts, Dir, Rule, Rules, FORMAT as RULES_FORMAT, VERSION as RULES_VERSION,
};
pub use sources::{FamilyBase, Imported};

/// Bits of `Hypothesis::sources`.
pub const SRC_RULE: u8 = 1;
pub const SRC_SUSPECT: u8 = 2;
pub const SRC_FAMILY: u8 = 4;
pub const SRC_IMPORTED: u8 = 8;
pub const SRC_CLEAVAGE: u8 = 16;

pub const REGISTRY_FORMAT: &str = "ipotesi/1";
const SRC_NAMES: [(u8, &str); 5] = [
    (SRC_RULE, "regola"),
    (SRC_SUSPECT, "sospetti"),
    (SRC_FAMILY, "famiglia"),
    (SRC_IMPORTED, "importata"),
    (SRC_CLEAVAGE, "rottura"),
];

#[derive(Debug, Clone)]
pub struct HypoParams {
    /// Polarity of the file to search: 1 or -1.
    pub polarity: i8,
    /// Most reactions chained from the parent.
    pub max_steps: u8,
    pub min_mz: f64,
    /// Most hypotheses taken from the suspect index (the best supported first); the number cut is in the registry.
    pub max_suspects: usize,
    /// Expected members of a family in the other polarity too ([M-H]- of a positive base and the reverse).
    pub other_polarity: bool,
}

impl Default for HypoParams {
    fn default() -> Self {
        HypoParams {
            polarity: 1,
            max_steps: 2,
            min_mz: 50.0,
            max_suspects: 500,
            other_polarity: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hypothesis {
    pub id: u32,
    pub sources: u8,
    /// Rule ids joined by `+`, `m+1`, the adduct, the key of a suspect, the name of an imported compound.
    pub label: String,
    pub formula: String,
    pub neutral: f64,
    pub mz: f64,
    pub adduct: &'static str,
    pub polarity: i8,
    pub steps: u8,
    /// Prediction level of the weakest step (2 literature, 3 class, 4 generic); 0 without a rule.
    pub level: u8,
    pub dir: Dir,
    /// RT of the family base: the hypothesis is looked for there, not in the window of the rule.
    pub rt_ref: Option<f32>,
    /// Other ways to the same formula, with the same or more steps.
    pub alternatives: Vec<String>,
}

/// What the search of one hypothesis gave (filled by `target`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    NotSearched,
    NotFound,
    Found(u32),
}

/// Everything the pass proves: hypotheses in a stable order, with their outcome.
#[derive(Debug, Clone)]
pub struct Registry {
    pub hyps: Vec<Hypothesis>,
    pub outcomes: Vec<Outcome>,
    pub rules_version: u32,
    pub polarity: i8,
    /// Suspect hits left out by `max_suspects`.
    pub suspects_cut: u32,
}

pub struct Inputs<'a> {
    pub parent: &'a str,
    pub suspects: Option<&'a crate::suspects::Suspects>,
    pub families: &'a [FamilyBase],
    pub imported: &'a [Imported],
    /// Formulas of cleavage products of the parent (the caller knows the structure; formulas alone give no fragments).
    pub fragments: &'a [String],
}

pub(crate) fn to_formula(c: &Counts) -> Option<Formula> {
    let mut out = Vec::new();
    for (e, n) in c {
        if *n < 0 {
            return None;
        }
        if *n > 0 {
            out.push((e.clone(), *n as u64));
        }
    }
    Some(out)
}

pub(crate) fn counts_of(f: &Formula) -> Counts {
    f.iter().map(|(e, n)| (e.clone(), *n as i64)).collect()
}

/// The adduct of the polarity and the m/z of a neutral mass.
pub(crate) fn ion(neutral: f64, polarity: i8) -> (&'static str, f64) {
    let a = if polarity >= 0 { "[M+H]+" } else { "[M-H]-" };
    (a, elements::ion_mz(neutral, a).unwrap_or(neutral))
}

fn same_key(a: &Hypothesis, b: &Hypothesis) -> bool {
    if a.formula.is_empty() || b.formula.is_empty() {
        a.mz.to_bits() == b.mz.to_bits() && a.adduct == b.adduct && a.label == b.label
    } else {
        a.formula == b.formula && a.adduct == b.adduct
    }
}

/// Merges the hypotheses that name the same ion (same formula and adduct): sources are united, the other labels become alternatives.
fn merge(mut raw: Vec<Hypothesis>) -> Vec<Hypothesis> {
    raw.sort_by(|a, b| {
        a.mz.total_cmp(&b.mz)
            .then_with(|| a.adduct.cmp(b.adduct))
            .then_with(|| a.formula.cmp(&b.formula))
            .then_with(|| a.label.cmp(&b.label))
    });
    let mut out: Vec<Hypothesis> = Vec::with_capacity(raw.len());
    for h in raw {
        match out.last_mut() {
            Some(l) if same_key(l, &h) => {
                l.sources |= h.sources;
                if l.label != h.label && !l.alternatives.contains(&h.label) {
                    l.alternatives.push(h.label);
                }
                l.level = if l.level == 0 {
                    h.level
                } else if h.level == 0 {
                    l.level
                } else {
                    l.level.min(h.level)
                };
                l.rt_ref = l.rt_ref.or(h.rt_ref);
            }
            _ => out.push(h),
        }
    }
    for (i, h) in out.iter_mut().enumerate() {
        h.id = i as u32;
        h.alternatives.sort();
    }
    out
}

/// All the hypotheses of a parent, in order of m/z; ids follow that order.
pub fn generate(rules: &Rules, inp: &Inputs, p: &HypoParams) -> Result<Registry> {
    let parent = elements::parse_formula(inp.parent)?;
    let pc = counts_of(&parent);
    let mut raw = sources::from_rules(rules, &pc, p);
    raw.extend(sources::from_fragments(inp.fragments, &pc, p)?);
    let mut cut = 0;
    if let Some(s) = inp.suspects {
        let (h, c) = sources::from_suspects(s, &pc, p);
        raw.extend(h);
        cut = c;
    }
    raw.extend(sources::from_families(inp.families, p));
    raw.extend(sources::from_imported(inp.imported, p)?);
    let hyps = merge(raw);
    let outcomes = vec![Outcome::NotSearched; hyps.len()];
    Ok(Registry {
        hyps,
        outcomes,
        rules_version: RULES_VERSION,
        polarity: p.polarity,
        suspects_cut: cut,
    })
}

fn names(sources: u8) -> impl Iterator<Item = &'static str> {
    SRC_NAMES
        .into_iter()
        .filter(move |(b, _)| sources & b != 0)
        .map(|(_, n)| n)
}

impl Registry {
    pub fn len(&self) -> usize {
        self.hyps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hyps.is_empty()
    }

    /// Number of hypotheses of each source bit (a hypothesis with two sources counts in both).
    pub fn by_source(&self) -> BTreeMap<&'static str, u32> {
        let mut m = BTreeMap::new();
        for h in &self.hyps {
            for n in names(h.sources) {
                *m.entry(n).or_insert(0) += 1;
            }
        }
        m
    }

    pub fn found(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o, Outcome::Found(_)))
            .count()
    }

    /// The registry as JSON (`ipotesi/1`): the base of the SHA-256 that compares two runs.
    pub fn to_json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"format\":")
            .str(REGISTRY_FORMAT)
            .raw(",\"rules_version\":")
            .int(self.rules_version as i64);
        o.raw(",\"polarity\":")
            .int(self.polarity as i64)
            .raw(",\"tried\":")
            .int(self.len() as i64);
        o.raw(",\"found\":")
            .int(self.found() as i64)
            .raw(",\"suspects_cut\":")
            .int(self.suspects_cut as i64);
        o.raw(",\"hypotheses\":[");
        for (i, (h, out)) in self.hyps.iter().zip(&self.outcomes).enumerate() {
            if i > 0 {
                o.raw(",");
            }
            hypothesis_json(&mut o, h, *out);
        }
        o.raw("]}");
        o.0
    }
}

fn hypothesis_json(o: &mut Out, h: &Hypothesis, out: Outcome) {
    o.raw("{\"id\":").int(h.id as i64).raw(",\"sources\":[");
    for (i, n) in names(h.sources).enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(n);
    }
    o.raw("],\"label\":")
        .str(&h.label)
        .raw(",\"formula\":")
        .str(&h.formula);
    o.raw(",\"mass\":")
        .num(round(h.neutral, 6))
        .raw(",\"mz\":")
        .num(round(h.mz, 6));
    o.raw(",\"adduct\":")
        .str(h.adduct)
        .raw(",\"steps\":")
        .int(h.steps as i64)
        .raw(",\"level\":")
        .int(h.level as i64);
    o.raw(",\"rt_ref\":")
        .opt(h.rt_ref.map(f64::from))
        .raw(",\"alternatives\":[");
    for (i, a) in h.alternatives.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(a);
    }
    o.raw("],\"outcome\":");
    match out {
        Outcome::NotSearched => o.str("non_cercata"),
        Outcome::NotFound => o.str("non_trovata"),
        Outcome::Found(n) => o.raw("{\"rows\":").int(n as i64).raw("}"),
    };
    o.raw("}");
}
