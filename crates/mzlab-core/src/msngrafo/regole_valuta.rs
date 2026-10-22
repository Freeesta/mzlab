//! The rules of `fragrules/1` that need only the formula, evaluated on the graph `msngrafo/1`: the key `regole` (which rule explains the loss of
//! an arc, the even-electron check, the trap cut-off, composite and stepped losses). Evidence with its reason and source, never a verdict.
//! Used by `grafo_json` (so by the `.wasm` module and the command line) when the option `regole` is on.

use super::candidati::{Elementi, ATOMI};
use super::formule::addotto;
use super::regole::{tabella, Attivazione, Globali, Regola, Tabella, Tipo};
use super::{Grafo, Opzioni};
use crate::elements::{fmt, parse_formula, ELECTRON};
use crate::json::{round, Out};

/// Version of the key `regole`.
pub const VERSIONE: u32 = 1;
/// The fragments of a trap CID are stable above q / this factor of the precursor m/z (the Mathieu limit of the stable ions).
const LIMITE_Q: f64 = 0.908;

/// The parameters of the rules: the q of the trap cut-off and the tolerance of a loss against a rule, in mDa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamRegole {
    pub cutoff_q: f64,
    pub toll_mda: f64,
}

impl Default for ParamRegole {
    fn default() -> Self {
        let g: &Globali = &tabella().globali;
        ParamRegole {
            cutoff_q: g.trap_cutoff_q,
            toll_mda: g.loss_tolerance_mda,
        }
    }
}

/// A rule that explains the loss of an arc.
#[derive(Debug, Clone, PartialEq)]
pub struct Applicata {
    pub id: &'static str,
    pub formula: &'static str,
    pub massa: f64,
    /// Observed Δ m/z minus the exact mass of the rule, in mDa.
    pub errore_mda: f64,
    pub radicale: bool,
    pub priorita: u8,
    pub energia: &'static str,
    pub perche: &'static str,
    pub refs: Vec<&'static str>,
}

/// The evaluation of one arc, in the order of `g.archi`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValArco {
    pub regole: Vec<Applicata>,
    /// Two rules whose losses add up to this one (beam-type activation, the intermediate step is not observed): their ids.
    pub composta: Vec<(&'static str, &'static str)>,
    /// A spectrum of the arc is a sum of energies: no breakdown curve is read from it.
    pub stepped: bool,
    /// `warn.msngrafo.*` keys of the arc.
    pub avvisi: Vec<&'static str>,
}

/// A rule whose product would fall below the trap cut-off of the spectrum of node `nodo`: it cannot be observed there.
#[derive(Debug, Clone, PartialEq)]
pub struct NonOsservabile {
    pub nodo: usize,
    pub regola: &'static str,
    pub mz: f64,
    pub soglia: f64,
}

/// A rule of kind `ion` that a node of the graph matches by m/z.
#[derive(Debug, Clone, PartialEq)]
pub struct IoneRegola {
    pub nodo: usize,
    pub regola: &'static str,
    pub errore_mda: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Valutazione {
    pub addotto: Option<&'static str>,
    pub archi: Vec<ValArco>,
    pub ioni: Vec<IoneRegola>,
    pub non_osservabili: Vec<NonOsservabile>,
    pub spiegati: usize,
    pub senza_regola: usize,
    /// Share of the intensity of the arcs (their most intense observation) that a rule explains.
    pub intensita_spiegata: f64,
}

/// The adduct of the ion of the root as typed; `None` for the ion itself or when the options give no formula.
pub(super) fn addotto_radice(o: &Opzioni, pol: i8) -> Option<&'static str> {
    addotto(o.formula.as_deref()?, pol)
}

/// The rules the formula alone decides, of the polarity and of the adduct (`radice`: the ion is the one of the root).
fn applicabili<'a>(
    t: &'static Tabella,
    pol: i8,
    addotto: Option<&'a str>,
    radice: bool,
    tipo: Tipo,
) -> impl Iterator<Item = &'static Regola> + 'a {
    t.regole.iter().filter(move |r| {
        r.tipo == tipo
            && r.solo_formula()
            && (r.polarita == 0 || r.polarita == pol)
            && (r.ioni.is_empty()
                || (radice && addotto.is_some_and(|a| r.ioni.iter().any(|i| i == a))))
    })
}

/// The loss vectors (in the elements of `e`) of the loss rules that hold from an ion of the root or of a fragment: the evidence `regola` of
/// a candidate formula.
pub(super) fn vettori_perdita(
    e: &Elementi,
    pol: i8,
    addotto: Option<&str>,
    radice: bool,
) -> Vec<Vec<u32>> {
    applicabili(tabella(), pol, addotto, radice, Tipo::Loss)
        .filter_map(|r| e.conta(&parse_formula(&r.formula).ok()?))
        .collect()
}

fn attivazioni(g: &Grafo, i: usize) -> Vec<Attivazione> {
    let mut v: Vec<Attivazione> = g.archi[i]
        .misure
        .iter()
        .map(|m| tipo_attivazione(&m.att))
        .collect();
    v.dedup();
    v
}

/// `CID` is the trap type (cut-off), anything else (`HCD`, ...) the beam type.
fn tipo_attivazione(att: &str) -> Attivazione {
    if att.eq_ignore_ascii_case("CID") {
        Attivazione::CidTrap
    } else {
        Attivazione::Beam
    }
}

fn compatibile(r: &Regola, att: &[Attivazione]) -> bool {
    r.attivazione == Attivazione::Both || att.is_empty() || att.contains(&r.attivazione)
}

/// A neutral loss with an odd number of electrons.
pub(super) fn radicale(formula: &str) -> bool {
    let Ok(f) = parse_formula(formula) else {
        return false;
    };
    let z: u64 = f
        .iter()
        .map(|(e, n)| ATOMI.iter().find(|a| a.0 == e).map_or(0, |a| a.1 as u64) * n)
        .sum();
    z % 2 == 1
}

fn applicata(r: &'static Regola, delta: f64) -> Applicata {
    Applicata {
        id: &r.id,
        formula: &r.formula,
        massa: r.massa,
        errore_mda: (delta - r.massa) * 1000.0,
        radicale: r.radicale,
        priorita: r.priorita,
        energia: &r.energia,
        perche: &r.perche,
        refs: r.refs.iter().map(|x| x.key.as_str()).collect(),
    }
}

/// The loss of the arc `i` matches the rule: the same formula when the arc has one, else the same mass within the tolerance.
fn coincide(g: &Grafo, i: usize, r: &Regola, toll: f64) -> bool {
    let a = &g.archi[i];
    match &a.formula {
        Some((f, _)) => parse_formula(f).is_ok_and(|p| fmt(&p) == r.formula),
        None => (a.perdita - r.massa).abs() <= toll / 1000.0,
    }
}

fn valuta_arco(
    t: &'static Tabella,
    g: &Grafo,
    i: usize,
    add: Option<&str>,
    p: &ParamRegole,
    glob: &Globali,
) -> ValArco {
    let a = &g.archi[i];
    let att = attivazioni(g, i);
    let radice = g.radici.contains(&a.da);
    let regole: Vec<Applicata> = applicabili(t, g.polarita, add, radice, Tipo::Loss)
        .filter(|r| compatibile(r, &att) && coincide(g, i, r, p.toll_mda))
        .map(|r| applicata(r, a.perdita))
        .collect();
    let beam = att.contains(&Attivazione::Beam);
    let composta = if regole.is_empty() && glob.beam_composite && beam {
        composte(t, g, i, add, radice, p.toll_mda)
    } else {
        vec![]
    };
    let mut avvisi = vec![];
    let da_pari = g.nodi[a.da]
        .formula
        .as_ref()
        .and_then(|f| {
            f.candidati
                .iter()
                .find(|c| Some(&c.formula) == f.scelta.as_ref())
        })
        .is_some_and(|c| c.pari);
    let formula = a.formula.as_ref().map(|(f, _)| f.as_str());
    if glob.even_electron
        && da_pari
        && formula.is_some_and(radicale)
        && !regole.iter().any(|r| r.radicale)
    {
        avvisi.push("warn.msngrafo.radicalLoss");
    }
    ValArco {
        regole,
        composta,
        stepped: glob.stepped_sum && a.misure.iter().any(|m| m.ce.is_nan()),
        avvisi,
    }
}

/// The pairs of loss rules (the second one on the product) whose masses add up to the loss of the arc.
fn composte(
    t: &'static Tabella,
    g: &Grafo,
    i: usize,
    add: Option<&str>,
    radice: bool,
    toll: f64,
) -> Vec<(&'static str, &'static str)> {
    let att = [Attivazione::Beam];
    let primi: Vec<&Regola> = applicabili(t, g.polarita, add, radice, Tipo::Loss)
        .filter(|r| compatibile(r, &att))
        .collect();
    let secondi: Vec<&Regola> = applicabili(t, g.polarita, add, false, Tipo::Loss)
        .filter(|r| compatibile(r, &att))
        .collect();
    let mut v = vec![];
    for r1 in &primi {
        for r2 in &secondi {
            if (g.archi[i].perdita - r1.massa - r2.massa).abs() <= toll / 1000.0 {
                v.push((&r1.id[..], &r2.id[..]));
            }
        }
    }
    v
}

/// The product of every loss rule from each precursor of a trap CID spectrum below `q / 0.908` of its m/z: not observable there.
fn sotto_soglia(
    t: &'static Tabella,
    g: &Grafo,
    add: Option<&str>,
    p: &ParamRegole,
) -> Vec<NonOsservabile> {
    let mut v: Vec<NonOsservabile> = vec![];
    for s in g
        .spettri
        .iter()
        .filter(|s| tipo_attivazione(&s.att) == Attivazione::CidTrap)
    {
        let Some(n) = s.precursore else { continue };
        let mz = g.nodi[n].mz;
        let soglia = p.cutoff_q / LIMITE_Q * mz;
        for r in applicabili(t, g.polarita, add, g.radici.contains(&n), Tipo::Loss)
            .filter(|r| r.attivazione != Attivazione::Beam)
        {
            let prodotto = mz - r.massa;
            if prodotto < soglia && !v.iter().any(|x| x.nodo == n && x.regola == r.id) {
                v.push(NonOsservabile {
                    nodo: n,
                    regola: &r.id,
                    mz: prodotto,
                    soglia,
                });
            }
        }
    }
    v
}

/// The nodes that are the ion of an `ion` rule (a sodium adduct that leaves as Na+).
fn ioni_regola(
    t: &'static Tabella,
    g: &Grafo,
    add: Option<&str>,
    p: &ParamRegole,
) -> Vec<IoneRegola> {
    let mut v = vec![];
    if g.radici.is_empty() {
        return v;
    }
    for r in applicabili(t, g.polarita, add, true, Tipo::Ion) {
        let mz = r.massa - g.polarita as f64 * ELECTRON;
        for (n, x) in g.nodi.iter().enumerate() {
            if (x.mz - mz).abs() <= p.toll_mda / 1000.0 {
                v.push(IoneRegola {
                    nodo: n,
                    regola: &r.id,
                    errore_mda: (x.mz - mz) * 1000.0,
                });
            }
        }
    }
    v
}

/// The rules of the table on the graph; the arcs are in the order of `g.archi`.
pub fn valuta(g: &Grafo, o: &Opzioni, p: &ParamRegole) -> Valutazione {
    valuta_con(tabella(), g, o, p)
}

/// `valuta` with another table (the tests read one of their own).
pub(super) fn valuta_con(
    t: &'static Tabella,
    g: &Grafo,
    o: &Opzioni,
    p: &ParamRegole,
) -> Valutazione {
    let add = addotto_radice(o, g.polarita);
    let glob = &t.globali;
    let archi: Vec<ValArco> = (0..g.archi.len())
        .map(|i| valuta_arco(t, g, i, add, p, glob))
        .collect();
    let forza = |i: usize| g.archi[i].misure.iter().map(|m| m.rel).fold(0.0, f64::max);
    let (mut tutta, mut spiegata) = (0.0, 0.0);
    let mut spiegati = 0;
    for (i, a) in archi.iter().enumerate() {
        tutta += forza(i);
        if !a.regole.is_empty() || !a.composta.is_empty() {
            spiegati += 1;
            spiegata += forza(i);
        }
    }
    let mut archi = archi;
    for (i, a) in archi.iter_mut().enumerate() {
        let soglia_nodo = |n: usize| {
            g.spettri.iter().any(|s| {
                s.precursore == Some(g.archi[i].da)
                    && tipo_attivazione(&s.att) == Attivazione::CidTrap
                    && g.nodi[n].mz < p.cutoff_q / LIMITE_Q * g.nodi[g.archi[i].da].mz
            })
        };
        if soglia_nodo(g.archi[i].a) {
            a.avvisi.push("warn.msngrafo.belowCutoff");
        }
    }
    Valutazione {
        addotto: add,
        senza_regola: archi.len() - spiegati,
        spiegati,
        intensita_spiegata: if tutta > 0.0 { spiegata / tutta } else { 0.0 },
        ioni: ioni_regola(t, g, add, p),
        non_osservabili: sotto_soglia(t, g, add, p),
        archi,
    }
}

fn lista(o: &mut Out, v: &[&str]) {
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(x);
    }
    o.raw("]");
}

fn applicata_json(o: &mut Out, r: &Applicata) {
    o.raw("{\"id\":")
        .str(r.id)
        .raw(",\"formula\":")
        .str(r.formula)
        .raw(",\"massa\":")
        .num(round(r.massa, 5))
        .raw(",\"errore_mda\":")
        .num(round(r.errore_mda, 2))
        .raw(",\"radicale\":")
        .raw(if r.radicale { "true" } else { "false" })
        .raw(",\"priorita\":")
        .int(r.priorita as i64)
        .raw(",\"energia\":")
        .str(r.energia)
        .raw(",\"refs\":");
    lista(o, &r.refs);
    o.raw("}");
}

fn arco_json(o: &mut Out, g: &Grafo, i: usize, a: &ValArco) {
    o.raw("{\"da\":")
        .str(&format!("n{}", g.archi[i].da))
        .raw(",\"a\":")
        .str(&format!("n{}", g.archi[i].a))
        .raw(",\"regole\":[");
    for (j, r) in a.regole.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        applicata_json(o, r);
    }
    o.raw("],\"composta\":[");
    for (j, (x, y)) in a.composta.iter().enumerate() {
        o.raw(if j > 0 { "," } else { "" });
        lista(o, &[x, y]);
    }
    o.raw("],\"stepped\":")
        .raw(if a.stepped { "true" } else { "false" })
        .raw(",\"avvisi\":");
    lista(o, &a.avvisi);
    o.raw("}");
}

/// The value of the key `regole` (see `docs/schemi/msngrafo-1.json`): keys in a fixed order, rounded numbers.
pub fn regole_json(g: &Grafo, v: &Valutazione, p: &ParamRegole) -> String {
    let mut o = Out::default();
    o.raw("{\"versione\":")
        .int(VERSIONE as i64)
        .raw(",\"tabella\":\"fragrules/1\",\"cutoff_q\":")
        .num(p.cutoff_q)
        .raw(",\"toll_perdita_mda\":")
        .num(p.toll_mda)
        .raw(",\"addotto\":");
    match v.addotto {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"archi\":[");
    for (i, a) in v.archi.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        arco_json(&mut o, g, i, a);
    }
    o.raw("],\"ioni\":[");
    for (i, x) in v.ioni.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"nodo\":")
            .str(&format!("n{}", x.nodo))
            .raw(",\"regola\":")
            .str(x.regola)
            .raw(",\"errore_mda\":")
            .num(round(x.errore_mda, 2))
            .raw("}");
    }
    o.raw("],\"non_osservabili\":[");
    for (i, x) in v.non_osservabili.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"nodo\":")
            .str(&format!("n{}", x.nodo))
            .raw(",\"regola\":")
            .str(x.regola)
            .raw(",\"mz\":")
            .num(round(x.mz, 4))
            .raw(",\"soglia\":")
            .num(round(x.soglia, 4))
            .raw("}");
    }
    o.raw("],\"conteggi\":{\"archi_spiegati\":")
        .int(v.spiegati as i64)
        .raw(",\"archi_senza_regola\":")
        .int(v.senza_regola as i64)
        .raw(",\"intensita_spiegata\":")
        .num(round(v.intensita_spiegata, 3))
        .raw("}}");
    o.0
}

/// `grafo` (the text of an `msngrafo/1`) with the key `regole` added at the end; the text of a graph without it does not change.
pub fn con_regole(grafo: &str, g: &Grafo, o: &Opzioni, p: &ParamRegole) -> String {
    let testa = grafo.strip_suffix('}').unwrap_or(grafo);
    format!(
        "{testa},\"regole\":{}}}",
        regole_json(g, &valuta(g, o, p), p)
    )
}
