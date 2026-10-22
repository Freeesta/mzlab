//! The rules of `fragrules/1` evaluated on the graph `msngrafo/1` (by the formula alone, or with the products the structure expects): which rule
//! explains the loss of an arc, the even-electron check, the trap cut-off, composite and stepped losses, `after` chains. Evidence with its reason and source, never a verdict.
//! Used by `grafo_json` (so by the `.wasm` module and the command line) when the option `regole` is on.

use super::candidati::{Elementi, ATOMI};
use super::formule::addotto;
use super::regole::{tabella, Attivazione, Globali, Regola, Tabella, Tipo};
use super::{Grafo, Opzioni};
use crate::elements::{fmt, mass, parse_formula, ELECTRON};

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
    pub formula: String,
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

/// What a rule expects of the structure of the user: the product it gives (a neutral loss, or an ion when `ione`) with its formula. The structure
/// side computes them (`mzlab_feat::chem::regole::attese`); the core only matches them against the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Attesa {
    pub regola: String,
    pub formula: String,
    pub ione: bool,
}

/// A rule with the formula of its product (the table's own, or the one the structure gave) and the mass of that formula.
struct Cand {
    r: &'static Regola,
    formula: String,
    massa: f64,
}

/// The parameters and inputs every evaluation of an arc needs.
struct Ctx<'a> {
    t: &'static Tabella,
    g: &'a Grafo,
    add: Option<&'a str>,
    p: &'a ParamRegole,
    attese: &'a [Attesa],
}

fn cand(r: &'static Regola, formula: &str) -> Option<Cand> {
    let f = parse_formula(formula).ok()?;
    Some(Cand {
        r,
        formula: fmt(&f),
        massa: mass(&f),
    })
}

/// The rules of the polarity and of the adduct (`radice`: the ion is the one of the root) whose product is a loss or an ion (`tipo`): those the
/// formula alone decides (with the formula of the table) and those the structure expects (`attese`, with the formula it gave).
fn applicabili(c: &Ctx, pol: i8, radice: bool, tipo: Tipo) -> Vec<Cand> {
    let ok = |r: &Regola| {
        (r.polarita == 0 || r.polarita == pol)
            && (r.ioni.is_empty()
                || (radice && c.add.is_some_and(|a| r.ioni.iter().any(|i| i == a))))
    };
    let mut v: Vec<Cand> =
        c.t.regole
            .iter()
            .filter(|r| r.tipo == tipo && !r.legge_struttura() && ok(r))
            .filter_map(|r| cand(r, &r.formula))
            .collect();
    for a in c.attese.iter().filter(|a| a.ione == (tipo == Tipo::Ion)) {
        if let Some(r) =
            c.t.regole
                .iter()
                .find(|r| r.id == a.regola)
                .filter(|r| ok(r))
        {
            v.extend(cand(r, &a.formula));
        }
    }
    v
}

/// The loss vectors (in the elements of `e`) of the loss rules that hold from an ion of the root or of a fragment: the evidence `regola` of
/// a candidate formula.
pub(super) fn vettori_perdita(
    e: &Elementi,
    pol: i8,
    addotto: Option<&str>,
    radice: bool,
) -> Vec<Vec<u32>> {
    tabella()
        .regole
        .iter()
        .filter(|r| {
            r.tipo == Tipo::Loss
                && r.solo_formula()
                && (r.polarita == 0 || r.polarita == pol)
                && (r.ioni.is_empty()
                    || (radice && addotto.is_some_and(|a| r.ioni.iter().any(|i| i == a))))
        })
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

fn applicata(c: &Cand, delta: f64) -> Applicata {
    let r = c.r;
    Applicata {
        id: &r.id,
        formula: c.formula.clone(),
        massa: c.massa,
        errore_mda: (delta - c.massa) * 1000.0,
        radicale: r.radicale || radicale(&c.formula),
        priorita: r.priorita,
        energia: &r.energia,
        perche: &r.perche,
        refs: r.refs.iter().map(|x| x.key.as_str()).collect(),
    }
}

/// The loss of the arc `i` matches the rule: the same formula when the arc has one, else the same mass within the tolerance.
fn coincide(g: &Grafo, i: usize, c: &Cand, toll: f64) -> bool {
    let a = &g.archi[i];
    match &a.formula {
        Some((f, _)) => parse_formula(f).is_ok_and(|p| fmt(&p) == c.formula),
        None => (a.perdita - c.massa).abs() <= toll / 1000.0,
    }
}

/// The rule `r` follows another: it holds only when an arc into the precursor of arc `i` is explained by a rule of its `after`.
fn dopo(c: &Ctx, prev: Option<&[ValArco]>, i: usize, r: &Regola) -> bool {
    r.after.is_empty()
        || prev.is_some_and(|pv| {
            c.g.archi.iter().enumerate().any(|(j, x)| {
                x.a == c.g.archi[i].da
                    && pv[j]
                        .regole
                        .iter()
                        .any(|y| r.after.iter().any(|id| id == y.id))
            })
        })
}

fn valuta_arco(c: &Ctx, i: usize, prev: Option<&[ValArco]>) -> ValArco {
    let (g, glob) = (c.g, &c.t.globali);
    let a = &g.archi[i];
    let att = attivazioni(g, i);
    let radice = g.radici.contains(&a.da);
    let regole: Vec<Applicata> = applicabili(c, g.polarita, radice, Tipo::Loss)
        .iter()
        .filter(|x| {
            compatibile(x.r, &att) && dopo(c, prev, i, x.r) && coincide(g, i, x, c.p.toll_mda)
        })
        .map(|x| applicata(x, a.perdita))
        .collect();
    let beam = att.contains(&Attivazione::Beam);
    let composta = if regole.is_empty() && glob.beam_composite && beam {
        composte(c, i, radice)
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
fn composte(c: &Ctx, i: usize, radice: bool) -> Vec<(&'static str, &'static str)> {
    let att = [Attivazione::Beam];
    let semplici = |radice: bool| -> Vec<Cand> {
        applicabili(c, c.g.polarita, radice, Tipo::Loss)
            .into_iter()
            .filter(|x| compatibile(x.r, &att) && x.r.after.is_empty())
            .collect()
    };
    let (primi, secondi) = (semplici(radice), semplici(false));
    let mut v = vec![];
    for r1 in &primi {
        for r2 in &secondi {
            if (c.g.archi[i].perdita - r1.massa - r2.massa).abs() <= c.p.toll_mda / 1000.0 {
                v.push((&r1.r.id[..], &r2.r.id[..]));
            }
        }
    }
    v
}

/// The product of every loss rule from each precursor of a trap CID spectrum below `q / 0.908` of its m/z: not observable there.
fn sotto_soglia(c: &Ctx) -> Vec<NonOsservabile> {
    let g = c.g;
    let mut v: Vec<NonOsservabile> = vec![];
    for s in g
        .spettri
        .iter()
        .filter(|s| tipo_attivazione(&s.att) == Attivazione::CidTrap)
    {
        let Some(n) = s.precursore else { continue };
        let mz = g.nodi[n].mz;
        let soglia = c.p.cutoff_q / LIMITE_Q * mz;
        for r in applicabili(c, g.polarita, g.radici.contains(&n), Tipo::Loss)
            .iter()
            .filter(|x| x.r.attivazione != Attivazione::Beam && x.r.after.is_empty())
        {
            let prodotto = mz - r.massa;
            if prodotto < soglia && !v.iter().any(|x| x.nodo == n && x.regola == r.r.id) {
                v.push(NonOsservabile {
                    nodo: n,
                    regola: &r.r.id,
                    mz: prodotto,
                    soglia,
                });
            }
        }
    }
    v
}

/// The nodes that are the ion of an `ion` rule (a sodium adduct that leaves as Na+, a tropylium or an acylium of the structure).
fn ioni_regola(c: &Ctx) -> Vec<IoneRegola> {
    let g = c.g;
    let mut v = vec![];
    if g.radici.is_empty() {
        return v;
    }
    for r in applicabili(c, g.polarita, true, Tipo::Ion) {
        let mz = r.massa - g.polarita as f64 * ELECTRON;
        for (n, x) in g.nodi.iter().enumerate() {
            let giu = (x.mz - mz) * 1000.0;
            let gia = v
                .iter()
                .any(|y: &IoneRegola| y.nodo == n && y.regola == r.r.id);
            if giu.abs() <= c.p.toll_mda && !gia {
                v.push(IoneRegola {
                    nodo: n,
                    regola: &r.r.id,
                    errore_mda: giu,
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
    let c = Ctx {
        t,
        g,
        add,
        p,
        attese: &o.attese,
    };
    // a second pass lets a rule that follows another (`after`) see the arcs the first one explained
    let mut archi: Vec<ValArco> = (0..g.archi.len())
        .map(|i| valuta_arco(&c, i, None))
        .collect();
    archi = (0..g.archi.len())
        .map(|i| valuta_arco(&c, i, Some(&archi)))
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
        ioni: ioni_regola(&c),
        non_osservabili: sotto_soglia(&c),
        archi,
    }
}
