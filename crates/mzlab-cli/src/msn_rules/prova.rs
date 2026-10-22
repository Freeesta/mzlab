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
pub const SPOSTA_PERDITE: f64 = 0.0137;

/// A record ready to be judged: the ion as the graph reads it, the activation and the peaks.
pub struct Registro {
    pub id: String,
    pub strato: &'static str,
    pub pol: i8,
    pub prec: f64,
    pub ce: f64,
    /// `CID` (trap type) or `HCD` (beam type).
    pub att: &'static str,
    pub ione: String,
    pub smiles: String,
    pub picchi: Vec<(f64, f64)>,
}

/// Peaks explained by (a) sub-formulas, (b) + rules of the formula, (c) + rules of the structure: counts and % of the base peak, and the rules that fired
/// (peaks per rule id).
#[derive(Default)]
pub struct Misura {
    pub n: usize,
    pub totale: f64,
    pub a: (usize, f64),
    pub b: (usize, f64),
    pub c: (usize, f64),
    pub regole: BTreeMap<String, usize>,
}

impl Misura {
    pub fn json(&self, o: &mut Out, con_regole: bool) {
        let n = |x: f64| mzlab_core::json::round(x, 2) + 0.0;
        o.raw("{\"n\":").int(self.n as i64);
        o.raw(",\"it\":").num(n(self.totale));
        o.raw(",\"a\":").int(self.a.0 as i64);
        o.raw(",\"ia\":").num(n(self.a.1));
        if con_regole {
            o.raw(",\"b\":").int(self.b.0 as i64);
            o.raw(",\"ib\":").num(n(self.b.1));
            o.raw(",\"c\":").int(self.c.0 as i64);
            o.raw(",\"ic\":").num(n(self.c.1));
            o.raw(",\"r\":{");
            for (i, (k, v)) in self.regole.iter().enumerate() {
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

fn opzioni(r: &Registro, regole: bool, attese: &[Attesa]) -> Opzioni {
    Opzioni {
        formula: Some(r.ione.clone()),
        regole: regole.then(ParamRegole::default),
        attese: attese.to_vec(),
        ..Opzioni::default()
    }
}

fn grafo(r: &Registro, picchi: &[(f64, f64)], o: &Opzioni) -> Grafo {
    grafo_da_picchi((r.prec, r.pol), (r.att, r.ce), picchi, o)
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
fn con_regola(g: &Grafo, v: &Valutazione, regole: &mut BTreeMap<String, usize>) -> (usize, f64) {
    let Some(prec) = g.spettri.first().and_then(|s| s.precursore) else {
        return (0, 0.0);
    };
    let (mut n, mut it) = (0, 0.0);
    for (rel, nodo) in frammenti(g) {
        let Some(nodo) = nodo else { continue };
        let ids = ids_del_picco(g, v, prec, nodo);
        if !ids.is_empty() {
            n += 1;
            it += rel;
        }
        for id in ids {
            *regole.entry(id).or_default() += 1;
        }
    }
    (n, it)
}

fn ids_del_picco(g: &Grafo, v: &Valutazione, prec: usize, nodo: usize) -> Vec<String> {
    let mut ids: Vec<String> = vec![];
    if let Some(i) = g.archi.iter().position(|x| x.da == prec && x.a == nodo) {
        ids.extend(v.archi[i].regole.iter().map(|r| r.id.to_string()));
        ids.extend(v.archi[i].composta.iter().map(|(x, y)| format!("{x}+{y}")));
    }
    ids.extend(
        v.ioni
            .iter()
            .filter(|x| x.nodo == nodo)
            .map(|x| x.regola.to_string()),
    );
    ids.sort();
    ids.dedup();
    ids
}

/// The three counts of the record on `picchi`, with the expectations of a structure (`attese`; the rules are evaluated only with `regole`).
pub fn misura(r: &Registro, picchi: &[(f64, f64)], attese: &[Attesa], regole: bool) -> Misura {
    let ga = grafo(r, picchi, &opzioni(r, false, &[]));
    let fr = frammenti(&ga);
    let mut m = Misura {
        n: fr.len(),
        totale: fr.iter().map(|p| p.0).sum(),
        a: (
            fr.iter().filter(|p| p.1.is_some()).count(),
            fr.iter().filter(|p| p.1.is_some()).map(|p| p.0).sum(),
        ),
        ..Misura::default()
    };
    if regole {
        let (ob, oc) = (opzioni(r, true, &[]), opzioni(r, true, attese));
        let (gb, gc) = (grafo(r, picchi, &ob), grafo(r, picchi, &oc));
        let p = ParamRegole::default();
        m.b = con_regola(&gb, &valuta_regole(&gb, &ob, &p), &mut BTreeMap::new());
        m.c = con_regola(&gc, &valuta_regole(&gc, &oc, &p), &mut m.regole);
    }
    m
}

/// The expectations of the structure of the record (`None` when the SMILES cannot be read).
pub fn attese_del_registro(r: &Registro) -> Option<Vec<Attesa>> {
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
pub fn picchi_spostati(r: &Registro) -> Vec<(f64, f64)> {
    r.picchi
        .iter()
        .map(|&(m, i)| {
            (
                if m < r.prec - 0.5 {
                    m - SPOSTA_PERDITE
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
    pub struttura: Option<Misura>,
    pub spettro: Option<Misura>,
    pub perdite: Misura,
}

pub fn decoy(r: &Registro, seme: u64, altro: Option<&[(f64, f64)]>, attese: &[Attesa]) -> Decoy {
    let struttura = Mol::parse(&r.smiles).ok().map(|m| {
        let d = attese_mol(&m.rimescola(seme_di(seme, &r.id)), tabella_regole());
        misura(r, &r.picchi, &d, true)
    });
    Decoy {
        struttura,
        spettro: altro.map(|p| misura(r, p, attese, true)),
        perdite: misura(r, &picchi_spostati(r), attese, true),
    }
}
