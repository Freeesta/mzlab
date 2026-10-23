//! The bond breaks of the structure of the user that explain the ions of an `msngrafo/1` (key `rotture`): per node the sets of atoms (one or
//! two bonds cut, a few hydrogens moved) whose m/z is the node's, per arc whether the child's set lies inside the parent's, per spectrum the
//! share of the intensity so explained. Hypotheses with their evidence, never an assignment. The calculation is `mzlab_feat::chem::rotture`
//! (it needs the structure); here the types and the JSON, used by `mzlab-cli msn-graph --rotture` and the `.wasm` `msngrafo`.

use crate::json::{round, Out};

/// Version of the `rotture` key.
pub const VERSIONE: u32 = 1;

/// One set of atoms of the structure that gives the m/z of a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Ipotesi {
    /// Indices of the bonds cut (empty = the whole molecule).
    pub legami: Vec<usize>,
    /// Heavy atoms of the piece (0-based, as in the SMILES), ascending.
    pub pezzo: Vec<usize>,
    /// Neutral formula of the piece with the hydrogens of the cuts and of the transfer.
    pub formula: String,
    /// Hydrogens moved beyond the plain cleavage (−2..2); 0 = the plain one.
    pub h: i8,
    pub mz_teorico: f64,
    pub ppm: f64,
}

/// The hypotheses of one node (at most `max_ipotesi` kept, `n` counts all).
#[derive(Debug, Clone, PartialEq)]
pub struct RotturaNodo {
    pub nodo: usize,
    pub n: usize,
    pub ipotesi: Vec<Ipotesi>,
}

/// An arc and whether a hypothesis of its child lies inside a hypothesis of its parent.
#[derive(Debug, Clone, PartialEq)]
pub struct RotturaArco {
    pub da: usize,
    pub a: usize,
    pub spiegato: bool,
}

/// The intensity of one spectrum (% of its base peak, summed) and the part of it on explained arcs.
#[derive(Debug, Clone, PartialEq)]
pub struct RotturaSpettro {
    pub albero: String,
    pub spiegata: f64,
    pub totale: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rotture {
    pub smiles: String,
    pub ppm: f64,
    pub h_max: u8,
    pub max_legami: u8,
    pub atomi: usize,
    pub legami: usize,
    pub avvisi: Vec<&'static str>,
    pub nodi: Vec<RotturaNodo>,
    pub archi: Vec<RotturaArco>,
    pub spettri: Vec<RotturaSpettro>,
}

impl Rotture {
    /// Arcs explained, arcs, and the explained share of the total intensity (%).
    pub fn conteggi(&self) -> (usize, usize, f64) {
        let sp = self.archi.iter().filter(|a| a.spiegato).count();
        let (s, t) = self
            .spettri
            .iter()
            .fold((0.0, 0.0), |(s, t), x| (s + x.spiegata, t + x.totale));
        (
            sp,
            self.archi.len(),
            if t > 0.0 { 100.0 * s / t } else { 0.0 },
        )
    }
}

fn lista(o: &mut Out, v: &[usize]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    o.raw("]");
}

/// The value of the key `rotture` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn rotture_json(r: &Rotture) -> String {
    let (sp, tot, int) = r.conteggi();
    let mut o = Out::default();
    o.raw("{\"versione\":")
        .int(VERSIONE as i64)
        .raw(",\"smiles\":")
        .str(&r.smiles)
        .raw(",\"ppm\":")
        .num(r.ppm)
        .raw(",\"h_max\":")
        .int(r.h_max as i64)
        .raw(",\"max_legami\":")
        .int(r.max_legami as i64)
        .raw(",\"atomi\":")
        .int(r.atomi as i64)
        .raw(",\"legami\":")
        .int(r.legami as i64)
        .raw(",\"avvisi\":[");
    for (i, a) in r.avvisi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(a);
    }
    o.raw("],\"conteggi\":{\"archi_spiegati\":")
        .int(sp as i64)
        .raw(",\"archi\":")
        .int(tot as i64)
        .raw(",\"intensita_spiegata\":")
        .num(round(int, 1))
        .raw("},\"nodi\":[");
    for (i, n) in r.nodi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"nodo\":")
            .str(&format!("n{}", n.nodo))
            .raw(",\"n\":")
            .int(n.n as i64)
            .raw(",\"ipotesi\":[");
        for (k, h) in n.ipotesi.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).raw("{\"legami\":");
            lista(&mut o, &h.legami);
            o.raw(",\"pezzo\":");
            lista(&mut o, &h.pezzo);
            o.raw(",\"formula\":")
                .str(&h.formula)
                .raw(",\"h\":")
                .int(h.h as i64)
                .raw(",\"mz_teorico\":")
                .num(round(h.mz_teorico, 5))
                .raw(",\"ppm\":")
                .num(round(h.ppm, 2))
                .raw("}");
        }
        o.raw("]}");
    }
    o.raw("],\"archi\":[");
    for (i, a) in r.archi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"da\":")
            .str(&format!("n{}", a.da))
            .raw(",\"a\":")
            .str(&format!("n{}", a.a))
            .raw(",\"spiegato\":")
            .raw(if a.spiegato { "true" } else { "false" })
            .raw("}");
    }
    o.raw("],\"spettri\":[");
    for (i, s) in r.spettri.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"albero\":")
            .str(&s.albero)
            .raw(",\"spiegata\":")
            .num(round(s.spiegata, 1))
            .raw(",\"totale\":")
            .num(round(s.totale, 1))
            .raw("}");
    }
    o.raw("]}");
    o.0
}

/// `grafo` (the text of an `msngrafo/1`) with the key `rotture` added at the end; the text of a graph without it does not change.
pub fn con_rotture(grafo: &str, r: &Rotture) -> String {
    super::vista::con_chiave(grafo, "rotture", &rotture_json(r))
}
