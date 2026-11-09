//! The cleavages of a structure (the SMILES of the user) that explain the m/z of ONE node of the graph: the types of the answer, the input taken from
//! the graph and the `strutture` key of `msngrafo/1`. The cleaving itself is `mzlab_feat::chem::strutture` (the fragmenter of mzLab, which this crate
//! cannot depend on). Candidates, never identifications: a broken bond is a hypothesis.

use super::{GNode, Graph};
use crate::json::{round, Out};

/// A peak of the spectrum of the node that the same piece of a cleavage also explains (`rel` in % of the base peak).
#[derive(Clone, Debug, PartialEq)]
pub struct ExplainedPeak {
    pub mz: f64,
    pub rel: f64,
    pub ppm: f64,
    pub formula: String,
    pub loss: String,
}

/// One cleaved bond and the piece that carries the charge.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    /// Index of the bond in the structure graph and of its two atoms (0-based, as the atoms of the SMILES).
    pub legame: usize,
    pub atoms: (usize, usize),
    pub label: String,
    /// 1 or 2: the side of the bond that carries the charge.
    pub lato: u8,
    pub piece: Vec<usize>,
    pub rest: Vec<usize>,
    /// Neutral formula of the piece (with the hydrogen of the cut) and the form that matched (`ion`, `radical`, `-H2O`, ...).
    pub formula: String,
    pub variant: String,
    pub perdita: String,
    pub theoretical_mz: f64,
    pub ppm: f64,
    pub peaks: Vec<ExplainedPeak>,
}

/// The cleavages that give the same m/z; `formula` is the candidate formula of the node with that m/z, if there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureCandidate {
    pub formula: Option<String>,
    pub theoretical_mz: f64,
    pub ppm: f64,
    pub cuts: Vec<Cut>,
}

/// The answer for one node.
#[derive(Clone, Debug, PartialEq)]
pub struct Structures {
    pub node: usize,
    pub smiles: String,
    pub mz: f64,
    pub ppm_tol: f64,
    /// Heavy atoms of the structure and bonds that can be cleaved (acyclic single bonds).
    pub atoms: usize,
    pub bonds: usize,
    pub candidates: Vec<StructureCandidate>,
    /// Keys `warn.msngrafo.*`.
    pub warnings: Vec<&'static str>,
}

/// What the cleaving needs from the graph: the m/z of the node, its polarity, the peaks of its own spectrum (m/z, % of the base peak) and the
/// candidate formulas it already has (formula, theoretical m/z).
#[derive(Clone, Debug, PartialEq)]
pub struct StructuresInput {
    pub nodo: usize,
    pub mz: f64,
    pub polarita: i8,
    pub picchi: Vec<(f64, f64)>,
    pub formule: Vec<(String, f64)>,
    pub root: bool,
}

impl Graph {
    /// The input of the cleavages of node `nodo`; `None` when there is no such node. The peaks are those of the spectra of which the node is the precursor
    /// (the highest relative intensity when several energies give the same m/z), ordered by m/z.
    pub fn structures_input(&self, node: usize) -> Option<StructuresInput> {
        let n: &GNode = self.nodi.get(node)?;
        let mut peaks: Vec<(f64, f64)> = Vec::new();
        for s in self.spettri.iter().filter(|s| s.precursore == Some(node)) {
            for &(mz, rel, _) in &s.picchi {
                match peaks.iter_mut().find(|p| p.0 == mz) {
                    Some(p) => p.1 = p.1.max(rel),
                    None => peaks.push((mz, rel)),
                }
            }
        }
        crate::sortc::slice_by(&mut peaks, |a, b| a.0.total_cmp(&b.0));
        let formule = n.formula.as_ref().map_or(vec![], |f| {
            f.candidates
                .iter()
                .map(|c| (c.formula.clone(), c.theoretical_mz))
                .collect()
        });
        Some(StructuresInput {
            nodo: node,
            mz: n.mz,
            polarita: self.polarita,
            picchi: peaks,
            formule,
            root: self.radici.contains(&node),
        })
    }
}

fn atoms(o: &mut Out, a: &[usize]) {
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    o.raw("]");
}

fn cut_at(o: &mut Out, t: &Cut) {
    o.raw("{\"legame\":{\"idx\":").int(t.legame as i64);
    o.raw(",\"atomi\":[")
        .int(t.atoms.0 as i64)
        .raw(",")
        .int(t.atoms.1 as i64)
        .raw("],\"etichetta\":")
        .str(&t.label)
        .raw("},\"lato\":")
        .int(t.lato as i64)
        .raw(",\"pezzo\":");
    atoms(o, &t.piece);
    o.raw(",\"resto\":");
    atoms(o, &t.rest);
    o.raw(",\"formula\":")
        .str(&t.formula)
        .raw(",\"variante\":")
        .str(&t.variant)
        .raw(",\"perdita\":")
        .str(&t.perdita)
        .raw(",\"mz_teorico\":")
        .num(round(t.theoretical_mz, 5))
        .raw(",\"ppm\":")
        .num(round(t.ppm, 2))
        .raw(",\"picchi\":[");
    for (i, p) in t.peaks.iter().enumerate() {
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
            .str(&p.loss)
            .raw("}");
    }
    o.raw("]}");
}

/// The value of the key `strutture` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn structures_json(s: &Structures) -> String {
    let mut o = Out::default();
    o.raw("{\"nodo\":")
        .str(&format!("n{}", s.node))
        .raw(",\"smiles\":")
        .str(&s.smiles)
        .raw(",\"mz\":")
        .num(round(s.mz, 5))
        .raw(",\"ppm_tol\":")
        .num(s.ppm_tol)
        .raw(",\"atomi\":")
        .int(s.atoms as i64)
        .raw(",\"legami\":")
        .int(s.bonds as i64)
        .raw(",\"avvisi\":[");
    for (i, a) in s.warnings.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"candidati\":[");
    for (i, c) in s.candidates.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"formula\":");
        match &c.formula {
            Some(f) => o.str(f),
            None => o.raw("null"),
        };
        o.raw(",\"mz_teorico\":")
            .num(round(c.theoretical_mz, 5))
            .raw(",\"ppm\":")
            .num(round(c.ppm, 2))
            .raw(",\"tagli\":[");
        for (k, t) in c.cuts.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" });
            cut_at(&mut o, t);
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}

/// `grafo` with the key `strutture_tutti`: the cleavages of every node of level 2 or more (`strutture=tutti`), one `strutture` value each, by node.
pub fn with_all_structures(graph: &str, s: &[Structures]) -> String {
    let v: Vec<String> = s.iter().map(structures_json).collect();
    super::vista::with_key(graph, "strutture_tutti", &format!("[{}]", v.join(",")))
}

/// `grafo` (the text of an `msngrafo/1`) with the key `strutture` added at the end; the text of a graph without it does not change.
pub fn with_structures(graph: &str, s: &Structures) -> String {
    super::vista::with_key(graph, "strutture", &structures_json(s))
}
