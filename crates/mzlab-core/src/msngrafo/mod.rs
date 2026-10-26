//! The fragmentation graph of an MSn infusion (`msngrafo/1`, `docs/agenti/msn-grafo.md`): the nodes of the precursor tree (`msnalbero`) and
//! their consensus spectra become ONE graph of ions, a node per ion seen at any level, energy or path, an arc per precursor → fragment
//! observed in the spectrum of that precursor, with the neutral loss. A DAG ordered by m/z, never a verdict: with the formula of the ion
//! of the root given by the user, each fragment gets the sub-formulas within the window of ppm (`formule`), and the others leave the graph.

mod annota;
mod candidati;
mod cerca;
mod cerca_ms2;
mod da_picchi;
mod esporta;
mod formule;
mod interpreta;
mod json;
mod leggi;
mod lib_export;
mod libreria;
mod posa;
mod posa_radiale;
mod radice;
mod regole;
mod regole_json;
mod regole_valuta;
mod rotta;
mod rotture;
mod strutture;
mod tabelle;
mod unisci;
mod vista;

pub use cerca::{cerca, opzioni_da_richiesta as opzioni_cerca, Opzioni as OpzioniCerca};
pub use cerca_ms2::{cerca_ms2, Ms2Hit};
pub use da_picchi::grafo_da_picchi;
pub use esporta::{da_richiesta_export, esporta, Formato};
pub use interpreta::{
    con_interpretazione, interpreta, interpretazione_json, perdite_note, InterpArco, PerditaArco,
    PerditaNota, PPM as PPM_PERDITA,
};
pub use json::json;
pub use leggi::{leggi, Letto};
pub use lib_export::{export_text, Formato as LibFormato};
pub use libreria::aggiungi;
pub use radice::{candidati_radice, CandidatoRadice};
pub use regole::{leggi_tabella, tabella as tabella_regole, Regola, Tabella, Tipo};
pub use regole_json::{con_regole, regole_json};
pub use regole_valuta::{valuta as valuta_regole, Attesa, ParamRegole, Valutazione};
pub use rotta::{da_richiesta, grafo_json};
pub use rotture::{
    con_rotture, rotture_json, Ipotesi, RotturaArco, RotturaNodo, RotturaSpettro, Rotture,
};
pub use strutture::{
    con_strutture, con_strutture_tutti, strutture_json, CandidatoStruttura, IngressoStrutture,
    PiccoSpiegato, Strutture, Taglio,
};
pub use tabelle::{library_text, table_text};
pub use vista::completa;

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
    /// The structure of the molecule, only written back in the JSON (the page draws the fragments on it).
    pub smiles: Option<String>,
    /// The 2D structure of the molecule as a molfile, only written back in the JSON (the drawings of the root and of the fragments start from it).
    pub molfile: Option<String>,
    /// A sub-formula is a candidate of a fragment within this many ppm of its m/z.
    pub ppm_formula: f64,
    /// Below this m/z the window may widen to `ppm_basso`, for a candidate with at least `prove_min` evidences.
    pub mz_basso: f64,
    pub ppm_basso: f64,
    pub prove_min: usize,
    /// Formulas chosen by the user among the candidates: (m/z of the node, formula of the ion).
    pub scelte: Vec<(f64, String)>,
    /// The rules of `fragrules/1` (key `regole` and the evidence `regola` of the candidates); `None` = off.
    pub regole: Option<ParamRegole>,
    /// What the structure of the user expects of the rules (`mzlab_feat::chem::regole::attese`); empty = no structure given.
    pub attese: Vec<Attesa>,
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
            smiles: None,
            molfile: None,
            ppm_formula: 10.0,
            mz_basso: 150.0,
            ppm_basso: 20.0,
            prove_min: 1,
            scelte: vec![],
            regole: None,
            attese: vec![],
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
    /// Width of the isolation window (Da) of each node of the tree, `None` when the file does not say; an empty slice = none known.
    pub larghezze: &'a [Option<f64>],
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
    /// The loss between the chosen formulas of the two nodes and its error in ppm of the m/z of `da`, when both have one.
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
    /// The M+1 of its most intense observation, % of it.
    pub m1: Option<f64>,
    /// Its own isolation window (Da), when it is isolated and the file says.
    pub larghezza: Option<f64>,
    /// The narrowest window on the path to the spectrum of its most intense observation: below `STRETTA` its M+1 is not expected there.
    pub a_monte: Option<f64>,
    /// With a formula: the peaks of its spectra that no sub-formula explains (their nodes left the graph).
    pub non_spiegati: Option<NonSpiegati>,
    /// Column and row of the drawing.
    pub pos: (usize, usize),
    pub formula: Option<FormulaNodo>,
}

/// The unexplained peaks of the spectra of a node: how many, and the 3 most intense (m/z, % of the base peak).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NonSpiegati {
    pub n: usize,
    pub picchi: Vec<(f64, f64)>,
}

/// One spectrum of the tree as the graph uses it: its peaks (after `min_rel`, `max_picchi`, heavy and isotope peaks) and the node of each,
/// `None` when it left the graph (no sub-formula of the root): the clean spectrum is the peaks with a node.
#[derive(Debug, Clone, PartialEq)]
pub struct SpettroG {
    /// `Nodo::id` of the tree.
    pub albero: String,
    pub livello: u8,
    pub att: String,
    pub ce: f64,
    /// The node of its precursor.
    pub precursore: Option<usize>,
    pub larghezza: Option<f64>,
    /// (m/z in this spectrum, % of its base peak, node).
    pub picchi: Vec<(f64, f64, Option<usize>)>,
}

/// A window narrower than this does not take the M+1 of a singly charged ion (±1.00336 Da around its centre).
pub const STRETTA: f64 = 2.0 * 1.003355;

/// A sub-formula of the ion of the root that explains a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidato {
    pub formula: String,
    pub mz_teorico: f64,
    /// Error of the m/z of the node from `mz_teorico`.
    pub ppm: f64,
    pub dbe: f64,
    pub pari: bool,
    /// The window it is in: `ppm_formula`, or `ppm_basso` with its evidences.
    pub finestra: f64,
    /// `perdita`, `isotopi`, `msn`: why a candidate of the wide window counts.
    pub prove: Vec<&'static str>,
}

/// The formulas of a node: the candidates, never a proposal; a choice only when there is one candidate or the user made it.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaNodo {
    /// By |ppm|, then Hill order.
    pub candidati: Vec<Candidato>,
    pub scelta: Option<String>,
    /// `radice`, `unica` or `utente` (with a choice).
    pub fonte: Option<&'static str>,
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
    /// Nodes with no sub-formula of the root in their window (only with a formula).
    pub formula: usize,
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
    /// Formulas for the root when the options gave none.
    pub candidati_radice: Vec<CandidatoRadice>,
    pub spettri: Vec<SpettroG>,
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
    g.candidati_radice = candidati_radice(&g, o);
    posa::posa(&mut g);
    g
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_cerca;
#[cfg(test)]
mod tests_esche;
#[cfg(test)]
mod tests_formule;
#[cfg(test)]
mod tests_interpreta;
#[cfg(test)]
mod tests_regole;
#[cfg(test)]
mod tests_regole_struttura;
#[cfg(test)]
mod tests_selezione;
#[cfg(test)]
mod tests_vista;
