//! The cleavages of a structure (the SMILES of the user) that explain the m/z of ONE node of the graph: the types of the answer, the input taken from
//! the graph and the `strutture` key of `msngrafo/1`. The cleaving itself is `mzlab_feat::chem::strutture` (the fragmenter of mzLab, which this crate
//! cannot depend on). Candidates, never identifications: a broken bond is a hypothesis.

use super::{Grafo, NodoG};
use crate::json::{round, Out};

/// A peak of the spectrum of the node that the same piece of a cleavage also explains (`rel` in % of the base peak).
#[derive(Clone, Debug, PartialEq)]
pub struct PiccoSpiegato {
    pub mz: f64,
    pub rel: f64,
    pub ppm: f64,
    pub formula: String,
    pub perdita: String,
}

/// One cleaved bond and the piece that carries the charge.
#[derive(Clone, Debug, PartialEq)]
pub struct Taglio {
    /// Index of the bond in the structure graph and of its two atoms (0-based, as the atoms of the SMILES).
    pub legame: usize,
    pub atomi: (usize, usize),
    pub etichetta: String,
    /// 1 or 2: the side of the bond that carries the charge.
    pub lato: u8,
    pub pezzo: Vec<usize>,
    pub resto: Vec<usize>,
    /// Neutral formula of the piece (with the hydrogen of the cut) and the form that matched (`ion`, `radical`, `-H2O`, ...).
    pub formula: String,
    pub variante: String,
    pub perdita: String,
    pub mz_teorico: f64,
    pub ppm: f64,
    pub picchi: Vec<PiccoSpiegato>,
}

/// The cleavages that give the same m/z; `formula` is the candidate formula of the node with that m/z, if there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidatoStruttura {
    pub formula: Option<String>,
    pub mz_teorico: f64,
    pub ppm: f64,
    pub tagli: Vec<Taglio>,
}

/// The answer for one node.
#[derive(Clone, Debug, PartialEq)]
pub struct Strutture {
    pub nodo: usize,
    pub smiles: String,
    pub mz: f64,
    pub ppm_tol: f64,
    /// Heavy atoms of the structure and bonds that can be cleaved (acyclic single bonds).
    pub atomi: usize,
    pub legami: usize,
    pub candidati: Vec<CandidatoStruttura>,
    /// Keys `warn.msngrafo.*`.
    pub avvisi: Vec<&'static str>,
}

/// What the cleaving needs from the graph: the m/z of the node, its polarity, the peaks of its own spectrum (m/z, % of the base peak) and the
/// candidate formulas it already has (formula, theoretical m/z).
#[derive(Clone, Debug, PartialEq)]
pub struct IngressoStrutture {
    pub nodo: usize,
    pub mz: f64,
    pub polarita: i8,
    pub picchi: Vec<(f64, f64)>,
    pub formule: Vec<(String, f64)>,
    pub radice: bool,
}

impl Grafo {
    /// The input of the cleavages of node `nodo`; `None` when there is no such node. The peaks are those of the spectra of which the node is the precursor
    /// (the highest relative intensity when several energies give the same m/z), ordered by m/z.
    pub fn ingresso_strutture(&self, nodo: usize) -> Option<IngressoStrutture> {
        let n: &NodoG = self.nodi.get(nodo)?;
        let mut picchi: Vec<(f64, f64)> = Vec::new();
        for s in self.spettri.iter().filter(|s| s.precursore == Some(nodo)) {
            for &(mz, rel, _) in &s.picchi {
                match picchi.iter_mut().find(|p| p.0 == mz) {
                    Some(p) => p.1 = p.1.max(rel),
                    None => picchi.push((mz, rel)),
                }
            }
        }
        crate::sortc::slice_by(&mut picchi, |a, b| a.0.total_cmp(&b.0));
        let formule = n.formula.as_ref().map_or(vec![], |f| {
            f.candidati
                .iter()
                .map(|c| (c.formula.clone(), c.mz_teorico))
                .collect()
        });
        Some(IngressoStrutture {
            nodo,
            mz: n.mz,
            polarita: self.polarita,
            picchi,
            formule,
            radice: self.radici.contains(&nodo),
        })
    }
}

fn atomi(o: &mut Out, a: &[usize]) {
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    o.raw("]");
}

fn taglio(o: &mut Out, t: &Taglio) {
    o.raw("{\"legame\":{\"idx\":").int(t.legame as i64);
    o.raw(",\"atomi\":[")
        .int(t.atomi.0 as i64)
        .raw(",")
        .int(t.atomi.1 as i64)
        .raw("],\"etichetta\":")
        .str(&t.etichetta)
        .raw("},\"lato\":")
        .int(t.lato as i64)
        .raw(",\"pezzo\":");
    atomi(o, &t.pezzo);
    o.raw(",\"resto\":");
    atomi(o, &t.resto);
    o.raw(",\"formula\":")
        .str(&t.formula)
        .raw(",\"variante\":")
        .str(&t.variante)
        .raw(",\"perdita\":")
        .str(&t.perdita)
        .raw(",\"mz_teorico\":")
        .num(round(t.mz_teorico, 5))
        .raw(",\"ppm\":")
        .num(round(t.ppm, 2))
        .raw(",\"picchi\":[");
    for (i, p) in t.picchi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(p.mz, 5))
            .raw(",\"rel\":")
            .num(round(p.rel, 1))
            .raw(",\"ppm\":")
            .num(round(p.ppm, 2))
            .raw(",\"formula\":")
            .str(&p.formula)
            .raw(",\"perdita\":")
            .str(&p.perdita)
            .raw("}");
    }
    o.raw("]}");
}

/// The value of the key `strutture` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn strutture_json(s: &Strutture) -> String {
    let mut o = Out::default();
    o.raw("{\"nodo\":")
        .str(&format!("n{}", s.nodo))
        .raw(",\"smiles\":")
        .str(&s.smiles)
        .raw(",\"mz\":")
        .num(round(s.mz, 5))
        .raw(",\"ppm_tol\":")
        .num(s.ppm_tol)
        .raw(",\"atomi\":")
        .int(s.atomi as i64)
        .raw(",\"legami\":")
        .int(s.legami as i64)
        .raw(",\"avvisi\":[");
    for (i, a) in s.avvisi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"candidati\":[");
    for (i, c) in s.candidati.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"formula\":");
        match &c.formula {
            Some(f) => o.str(f),
            None => o.raw("null"),
        };
        o.raw(",\"mz_teorico\":")
            .num(round(c.mz_teorico, 5))
            .raw(",\"ppm\":")
            .num(round(c.ppm, 2))
            .raw(",\"tagli\":[");
        for (k, t) in c.tagli.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" });
            taglio(&mut o, t);
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}

/// `grafo` with the key `strutture_tutti`: the cleavages of every node of level 2 or more (`strutture=tutti`), one `strutture` value each, by node.
pub fn con_strutture_tutti(grafo: &str, s: &[Strutture]) -> String {
    let v: Vec<String> = s.iter().map(strutture_json).collect();
    super::vista::con_chiave(grafo, "strutture_tutti", &format!("[{}]", v.join(",")))
}

/// `grafo` (the text of an `msngrafo/1`) with the key `strutture` added at the end; the text of a graph without it does not change.
pub fn con_strutture(grafo: &str, s: &Strutture) -> String {
    super::vista::con_chiave(grafo, "strutture", &strutture_json(s))
}
