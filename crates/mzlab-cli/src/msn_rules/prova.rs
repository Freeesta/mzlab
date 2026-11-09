//! The numbers of `mzlab-cli msn-rules` for ONE library spectrum: how many of its peaks the graph explains by sub-formulas, by the rules of the formula
//! and by the rules of the structure, with the rules that fired; the same on the three decoys (rewired structure, the spectrum of another record,
//! losses shifted). Computed with the engine of the page (`msngrafo`), never a verdict.

use std::collections::BTreeMap;

use mzlab_core::json::Out;
use mzlab_core::msngrafo::{
    grafo_da_picchi, tabella_regole, valuta_regole, Attesa, Grafo, Opzioni, ParamRegole,
    Valutazione,
};
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::regole::attese_mol;

/// Shift of the losses of the third decoy, Da (no loss of the table is near it).
pub const SHIFT_LOSSES: f64 = 0.0137;

/// A record ready to be judged: the ion as the graph reads it, the activation and the peaks.
pub struct Registry {
    pub id: String,
    pub strato: &'static str,
    pub pol: i8,
    pub prec: f64,
    pub ce: f64,
    /// `CID` (trap type) or `HCD` (beam type).
    pub att: &'static str,
    pub ione: String,
    pub smiles: String,
    pub peaks: Vec<(f64, f64)>,
}

/// Peaks explained by (a) sub-formulas, (b) + rules of the formula, (c) + rules of the structure: counts and % of the base peak, and the rules that fired
/// (peaks per rule id).
#[derive(Default)]
pub struct Measure {
    pub n: usize,
    pub total: f64,
    pub a: (usize, f64),
    pub b: (usize, f64),
    pub c: (usize, f64),
    pub rules: BTreeMap<String, usize>,
}

impl Measure {
    pub fn json(&self, o: &mut Out, with_rules: bool) {
        let n = |x: f64| mzlab_core::json::round(x, 2) + 0.0;
        o.raw("{\"n\":").int(self.n as i64);
        o.raw(",\"it\":").num(n(self.total));
        o.raw(",\"a\":").int(self.a.0 as i64);
        o.raw(",\"ia\":").num(n(self.a.1));
        if with_rules {
            o.raw(",\"b\":").int(self.b.0 as i64);
            o.raw(",\"ib\":").num(n(self.b.1));
            o.raw(",\"c\":").int(self.c.0 as i64);
            o.raw(",\"ic\":").num(n(self.c.1));
            o.raw(",\"r\":{");
            for (i, (k, v)) in self.rules.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" })
                    .str(k)
                    .raw(":")
                    .int(*v as i64);
            }
            o.raw("}");
        }
        o.raw("}");
    }
}

fn options(r: &Registry, rules: bool, attese: &[Attesa]) -> Opzioni {
    Opzioni {
        formula: Some(r.ione.clone()),
        regole: rules.then(ParamRegole::default),
        attese: attese.to_vec(),
        ..Opzioni::default()
    }
}

fn graph(r: &Registry, peaks: &[(f64, f64)], o: &Opzioni) -> Grafo {
    grafo_da_picchi((r.prec, r.pol), (r.att, r.ce), peaks, o)
}

/// The fragment peaks of the spectrum of the graph with their node (the peak of the precursor itself is not a fragment): (% of base peak, node).
fn frammenti(g: &Grafo) -> Vec<(f64, Option<usize>)> {
    let Some(s) = g.spettri.first() else {
        return vec![];
    };
    s.picchi
        .iter()
        .filter(|p| p.2.is_none() || p.2 != s.precursore)
        .map(|p| (p.1, p.2))
        .collect()
}

/// Peaks that a rule explains (an arc with a rule, a composite loss or an ion rule on the node): count, intensity, and the rules.
fn with_rule(g: &Grafo, v: &Valutazione, rules: &mut BTreeMap<String, usize>) -> (usize, f64) {
    let Some(prec) = g.spettri.first().and_then(|s| s.precursore) else {
        return (0, 0.0);
    };
    let (mut n, mut it) = (0, 0.0);
    for (rel, node) in frammenti(g) {
        let Some(nodo) = node else { continue };
        let ids = peak_ids(g, v, prec, nodo);
        if !ids.is_empty() {
            n += 1;
            it += rel;
        }
        for id in ids {
            *rules.entry(id).or_default() += 1;
        }
    }
    (n, it)
}

fn peak_ids(g: &Grafo, v: &Valutazione, prec: usize, node: usize) -> Vec<String> {
    let mut ids: Vec<String> = vec![];
    if let Some(i) = g.archi.iter().position(|x| x.da == prec && x.a == node) {
        ids.extend(v.archi[i].regole.iter().map(|r| r.id.to_string()));
        ids.extend(v.archi[i].composta.iter().map(|(x, y)| format!("{x}+{y}")));
    }
    ids.extend(
        v.ioni
            .iter()
            .filter(|x| x.nodo == node)
            .map(|x| x.regola.to_string()),
    );
    ids.sort();
    ids.dedup();
    ids
}

/// The three counts of the record on `picchi`, with the expectations of a structure (`attese`; the rules are evaluated only with `regole`).
pub fn measure(r: &Registry, peaks: &[(f64, f64)], attese: &[Attesa], rules: bool) -> Measure {
    let ga = graph(r, peaks, &options(r, false, &[]));
    let fr = frammenti(&ga);
    let mut m = Measure {
        n: fr.len(),
        total: fr.iter().map(|p| p.0).sum(),
        a: (
            fr.iter().filter(|p| p.1.is_some()).count(),
            fr.iter().filter(|p| p.1.is_some()).map(|p| p.0).sum(),
        ),
        ..Measure::default()
    };
    if rules {
        let (ob, oc) = (options(r, true, &[]), options(r, true, attese));
        let (gb, gc) = (graph(r, peaks, &ob), graph(r, peaks, &oc));
        let p = ParamRegole::default();
        m.b = with_rule(&gb, &valuta_regole(&gb, &ob, &p), &mut BTreeMap::new());
        m.c = with_rule(&gc, &valuta_regole(&gc, &oc, &p), &mut m.rules);
    }
    m
}

/// The expectations of the structure of the record (`None` when the SMILES cannot be read).
pub fn registry_expectations(r: &Registry) -> Option<Vec<Attesa>> {
    let m = Mol::parse(&r.smiles).ok()?;
    Some(attese_mol(&m, tabella_regole()))
}

/// FNV-1a of the id: a seed per record that does not depend on the order of the batch.
pub fn seme_di(seme: u64, id: &str) -> u64 {
    id.bytes().fold(seme ^ 0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The peaks with the losses moved by `SPOSTA_PERDITE` (every peak below the precursor; the precursor stays).
pub fn shifted_peaks(r: &Registry) -> Vec<(f64, f64)> {
    r.peaks
        .iter()
        .map(|&(m, i)| {
            (
                if m < r.prec - 0.5 {
                    m - SHIFT_LOSSES
                } else {
                    m
                },
                i,
            )
        })
        .collect()
}

/// The decoys of a record: (rewired structure, spectrum of another record, losses shifted). The structure decoy is evaluated only for (c).
pub struct Decoy {
    pub struttura: Option<Measure>,
    pub spettro: Option<Measure>,
    pub losses: Measure,
}

pub fn decoy(r: &Registry, seme: u64, altro: Option<&[(f64, f64)]>, attese: &[Attesa]) -> Decoy {
    let struttura = Mol::parse(&r.smiles).ok().map(|m| {
        let d = attese_mol(&m.rimescola(seme_di(seme, &r.id)), tabella_regole());
        measure(r, &r.peaks, &d, true)
    });
    Decoy {
        struttura,
        spettro: altro.map(|p| measure(r, p, attese, true)),
        losses: measure(r, &shifted_peaks(r), attese, true),
    }
}
