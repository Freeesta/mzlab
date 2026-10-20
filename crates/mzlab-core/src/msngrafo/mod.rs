//! The fragmentation graph of an MSn infusion (`msngrafo/1`, `docs/agenti/msn-grafo.md`): the nodes of the precursor tree (`msnalbero`) and
//! their consensus spectra become ONE graph of ions, a node per ion seen at any level, energy or path, an arc per precursor → fragment
//! observed in the spectrum of that precursor, with the neutral loss. A DAG ordered by m/z, never a verdict: with the formula of the ion
//! of the root, each node gets a proposed sub-formula (`formule`), with its alternatives and the reason of the choice.

mod candidati;
mod formule;
mod json;
mod posa;
mod scelta;
mod unisci;

pub use json::json;

use crate::msnalbero::Albero;

/// The options of the graph; every one is written in the JSON so that a graph can be rebuilt.
#[derive(Debug, Clone, PartialEq)]
pub struct Opzioni {
    /// Tolerance of the merging of the peaks, in ppm of the m/z.
    pub ppm: f64,
    /// The smallest tolerance in Da (low m/z, where a few ppm are less than the scatter of the peaks).
    pub da_min: f64,
    /// Peaks below this % of the base peak of their spectrum stay out of the graph.
    pub min_rel: f64,
    /// At most this many peaks per spectrum (the most intense).
    pub max_picchi: usize,
    /// `false`: the M+1 peaks of the fragments are left out (counted); `true`: nodes with `isotopo_di`.
    pub isotopi: bool,
    /// Only the spectra whose last stage has this activation (`cid`, `hcd`, ...; case ignored); `None` = all.
    pub attivazione: Option<String>,
    /// The ion of the root as typed (`C8H11N4O2+`, `C8H10N4O2+H`, `C8H10N4O2 [M+H]+`); `None` = masses only.
    pub formula: Option<String>,
    /// One shift in ppm, the median error of the nodes with a single candidate, applied to every m/z before the choice.
    pub calibra: bool,
    /// Formulas chosen by the user: (m/z of the node, formula of the ion).
    pub fissate: Vec<(f64, String)>,
}

impl Default for Opzioni {
    fn default() -> Self {
        Opzioni {
            ppm: 5.0,
            da_min: 0.002,
            min_rel: 2.0,
            max_picchi: 120,
            isotopi: false,
            attivazione: None,
            formula: None,
            calibra: true,
            fissate: vec![],
        }
    }
}

impl Opzioni {
    /// The tolerance in Da at this m/z.
    pub fn tol(&self, mz: f64) -> f64 {
        (self.ppm * mz * 1e-6).max(self.da_min)
    }
}

/// A consensus spectrum (any intensity scale: the graph uses % of its base peak).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spettro {
    pub mz: Vec<f64>,
    pub rel: Vec<f64>,
}

/// What the graph is made of: the tree of the precursors, the consensus of its MS1 (the root ions) and of each of its nodes, in the order
/// of `albero.nodi`.
pub struct Ingresso<'a> {
    pub albero: &'a Albero,
    pub ms1: Option<&'a Spettro>,
    pub spettri: &'a [Spettro],
    /// 1 or -1 (written as it is).
    pub polarita: i8,
}

/// One spectrum in which an arc is seen: the node of the tree that gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct Misura {
    /// `Nodo::id` of the tree (the filter `msn:<id>` of the routes).
    pub albero: String,
    /// MS level of the spectrum.
    pub livello: u8,
    pub att: String,
    /// `NaN` when the energies are merged in the tree.
    pub ce: f64,
    pub n_scansioni: usize,
    /// % of the base peak of that spectrum (0 for an isolated precursor not found there).
    pub rel: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arco {
    pub da: usize,
    pub a: usize,
    /// m/z of `da` minus m/z of `a`.
    pub perdita: f64,
    /// Known neutral losses within the tolerance (formulas, «compatible with», never assigned).
    pub compatibili: Vec<&'static str>,
    /// The loss between the formulas of the two nodes and its error in ppm of the m/z of `da`, when both have one.
    pub formula: Option<(String, f64)>,
    /// The arc is a step of a path that was isolated (`a` is the precursor of a child of the spectrum).
    pub isolamento: bool,
    pub misure: Vec<Misura>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodoG {
    /// Intensity-weighted mean of the peaks (the precursors when it is only isolated, never seen above `min_rel`).
    pub mz: f64,
    /// Largest distance of an observation from `mz`, in ppm.
    pub dev_ppm: f64,
    /// MS level of the first spectrum in which it is seen (1 = a root ion of the MS1).
    pub livello: u8,
    /// The nodes of the tree whose precursor it is (sorted); empty = only observed.
    pub albero: Vec<String>,
    /// Number of spectra with the peak.
    pub osservazioni: usize,
    pub rel_max: f64,
    pub isotopo_di: Option<usize>,
    /// Column and row of the drawing.
    pub pos: (usize, usize),
    pub formula: Option<FormulaNodo>,
}

/// Another candidate of a node, with its score.
#[derive(Debug, Clone, PartialEq)]
pub struct Alternativa {
    pub formula: String,
    pub ppm: f64,
    pub dbe: f64,
    pub punteggio: f64,
}

/// The formula proposed for a node: a sub-formula of the ion of the root, never an identification.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaNodo {
    pub proposta: String,
    /// Error of the (calibrated) m/z of the node from the m/z of the formula.
    pub ppm: f64,
    pub dbe: f64,
    pub pari: bool,
    /// Sub-formulas of the root within the tolerance, before the constraints of the graph.
    pub n_candidati: usize,
    /// At most 4, best first.
    pub alternative: Vec<Alternativa>,
    /// `why.msngrafo.*` and its numeric parameters.
    pub motivo: (&'static str, Vec<(&'static str, f64)>),
    /// `auto`, `utente` or `radice`.
    pub fonte: &'static str,
    /// The second candidate scores within `AMBIGUA` of the first.
    pub ambigua: bool,
}

/// The ion of the root: its formula, and how far the node of the graph is from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Ione {
    pub formula: String,
    pub mz_teorico: f64,
    pub mz_osservato: f64,
    pub ppm: f64,
}

/// A note on one node of the tree (`warn.msngrafo.*`).
#[derive(Debug, Clone, PartialEq)]
pub struct Avviso {
    pub chiave: &'static str,
    /// Node of the tree (empty when the note is about the graph).
    pub albero: String,
    /// Node of the graph the note is about.
    pub nodo: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Conteggi {
    pub pesanti: usize,
    pub isotopi: usize,
    pub deboli: usize,
}

/// The graph: nodes in the order (level, m/z), arcs in the order (from, to).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grafo {
    pub nodi: Vec<NodoG>,
    pub archi: Vec<Arco>,
    pub radici: Vec<usize>,
    pub polarita: i8,
    pub scartati: Conteggi,
    pub avvisi: Vec<Avviso>,
    pub ione: Option<Ione>,
    /// (shift in ppm, nodes it is the median of).
    pub calibrazione: Option<(f64, usize)>,
}

/// Losses a chemist expects in ESI CID/HCD, even-electron first; matched by mass only.
pub(crate) const PERDITE: [(&str, f64); 16] = [
    ("H2", 2.01565),
    ("CH4", 16.0313),
    ("NH3", 17.02655),
    ("H2O", 18.010565),
    ("HCN", 27.010899),
    ("CO", 27.994915),
    ("C2H4", 28.0313),
    ("CH2O", 30.010565),
    ("CH5N", 31.042199),
    ("CH4O", 32.026215),
    ("H2S", 33.987721),
    ("C2H2O", 42.010565),
    ("C3H6", 42.04695),
    ("CO2", 43.989829),
    ("CH2O2", 46.005479),
    ("SO2", 63.961901),
];

/// The graph of the tree and its spectra (`spettri[i]` = consensus of `albero.nodi[i]`).
pub fn costruisci(ing: &Ingresso, o: &Opzioni) -> Grafo {
    let mut g = unisci::unisci(ing, o);
    formule::assegna(&mut g, o);
    posa::posa(&mut g);
    g
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_formule;
