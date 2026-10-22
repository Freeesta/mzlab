//! The model of the static figure: the scene read from `msngrafo/1` (ions, arcs, positions of the key `vista`), the text pieces of the LABEL RULE of
//! `mzlab/web/msngrafo-vista.js` and the boxes of the ions. Used only by `msn_figura`.

use std::collections::{HashMap, HashSet};

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

pub(crate) const W: f64 = 176.0;
pub(crate) const WR: f64 = 150.0;
pub(crate) const LH: f64 = 14.0;
pub(crate) const GAP_X: f64 = 130.0;
pub(crate) const GAP_Y: f64 = 20.0;
pub(crate) const MAXC: usize = 4;
pub(crate) const DW: f64 = 150.0;
pub(crate) const DH: f64 = 80.0;

pub(crate) const INK: &str = "#1a1a1a";
pub(crate) const MUTED: &str = "#4d4d4d";
pub(crate) const LINE: &str = "#595959";
pub(crate) const BEST: &str = "#0072B2";
pub(crate) const WARN: &str = "#E69F00";

// ---- model -----------------------------------------------------------------------------------------------------------

pub(crate) struct Cand {
    pub(crate) formula: String,
    pub(crate) ppm: f64,
    pub(crate) prove: Vec<String>,
}

pub(crate) struct Ion {
    pub(crate) id: String,
    pub(crate) livello: i64,
    pub(crate) mz: f64,
    pub(crate) isolato: bool,
    pub(crate) cands: Vec<Cand>,
    pub(crate) scelta: Option<String>,
    pub(crate) has_formula: bool,
}

pub(crate) struct Pos {
    pub(crate) livelli: Vec<i64>,
    pub(crate) rel: Option<f64>,
    pub(crate) da: Option<String>,
    pub(crate) entranti: i64,
    pub(crate) comune: bool,
    pub(crate) col: i64,
    pub(crate) riga: i64,
    pub(crate) anello: i64,
    pub(crate) angolo: f64,
}

pub(crate) struct Misura {
    pub(crate) livello: i64,
    pub(crate) att: Option<String>,
    pub(crate) ce: Option<f64>,
    pub(crate) rel: f64,
}

pub(crate) struct Arco {
    pub(crate) da: String,
    pub(crate) a: String,
    pub(crate) misure: Vec<Misura>,
    pub(crate) formula: Option<String>,
    pub(crate) da_massa: f64,
    pub(crate) compat: Vec<String>,
    pub(crate) regola: String,
}

/// Everything the drawing needs, read once from the graph.
pub(crate) struct Scena {
    pub(crate) ioni: Vec<Ion>,
    pub(crate) pos: HashMap<String, Pos>,
    pub(crate) archi: Vec<Arco>,
    pub(crate) best: HashSet<String>,
    pub(crate) carica: String,
    pub(crate) molfile: Option<String>,
    pub(crate) smiles: bool,
}

pub(crate) fn bad() -> Error {
    Error::new("err.msnfigura.bad")
}

pub(crate) fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.str()).map(str::to_string)
}

pub(crate) fn n(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.f64())
}

/// The rule ids shown under a loss: at most two (the weak ones tagged), `+n` for the rest; empty when no rule explains it.
pub(crate) fn regola_di(j: &Value, da: &str, a: &str) -> String {
    let Some(r) = j.get("regole").and_then(|r| r.get("archi")).and_then(|l| {
        l.arr()
            .iter()
            .find(|x| s(x, "da").as_deref() == Some(da) && s(x, "a").as_deref() == Some(a))
    }) else {
        return String::new();
    };
    let reg = r.get("regole").map_or(&[][..], |x| x.arr());
    let comp = r.get("composta").map_or(&[][..], |x| x.arr());
    let ids: Vec<String> = if reg.is_empty() {
        comp.iter()
            .map(|c| {
                c.arr()
                    .iter()
                    .filter_map(|x| x.str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .collect()
    } else {
        reg.iter()
            .map(|x| {
                let weak = if n(x, "priorita") == Some(3.0) {
                    " (weak)"
                } else {
                    ""
                };
                format!("{}{weak}", s(x, "id").unwrap_or_default())
            })
            .collect()
    };
    if ids.is_empty() {
        return String::new();
    }
    let more = if ids.len() > 2 {
        format!(" +{}", ids.len() - 2)
    } else {
        String::new()
    };
    format!(
        "{}{more}",
        ids.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
    )
}

fn leggi_ioni(j: &Value) -> Result<Vec<Ion>> {
    let mut ioni = Vec::new();
    for x in j.get("nodi").map_or(&[][..], |a| a.arr()) {
        let f = x.get("formula").filter(|f| !f.is_null());
        let cands = f
            .and_then(|f| f.get("candidati"))
            .map_or(&[][..], |c| c.arr())
            .iter()
            .map(|c| Cand {
                formula: s(c, "formula").unwrap_or_default(),
                ppm: n(c, "ppm").unwrap_or(0.0),
                prove: c
                    .get("prove")
                    .map_or(&[][..], |p| p.arr())
                    .iter()
                    .filter_map(|p| p.str().map(str::to_string))
                    .collect(),
            })
            .collect();
        ioni.push(Ion {
            id: s(x, "id").ok_or_else(bad)?,
            livello: n(x, "livello").unwrap_or(0.0) as i64,
            mz: n(x, "mz").ok_or_else(bad)?,
            isolato: x.get("isolato").and_then(|b| b.bool()).unwrap_or(false),
            cands,
            scelta: f.and_then(|f| s(f, "scelta")),
            has_formula: f.is_some(),
        });
    }
    Ok(ioni)
}

fn leggi_pos(vista: &Value) -> Result<HashMap<String, Pos>> {
    let mut pos = HashMap::new();
    for x in vista.get("nodi").map_or(&[][..], |a| a.arr()) {
        let best = x.get("best");
        let albero = x.get("albero");
        let radiale = x.get("radiale").filter(|r| !r.is_null());
        pos.insert(
            s(x, "id").ok_or_else(bad)?,
            Pos {
                livelli: x
                    .get("livelli")
                    .map_or(&[][..], |l| l.arr())
                    .iter()
                    .filter_map(|l| l.f64())
                    .map(|l| l as i64)
                    .collect(),
                rel: n(x, "rel"),
                da: best.and_then(|b| s(b, "da")),
                entranti: n(x, "entranti").unwrap_or(0.0) as i64,
                comune: x.get("comune").and_then(|b| b.bool()).unwrap_or(false),
                col: albero.and_then(|a| n(a, "col")).unwrap_or(0.0) as i64,
                riga: albero.and_then(|a| n(a, "riga")).unwrap_or(0.0) as i64,
                anello: radiale.and_then(|a| n(a, "anello")).unwrap_or(0.0) as i64,
                angolo: radiale.and_then(|a| n(a, "angolo")).unwrap_or(0.0),
            },
        );
    }
    Ok(pos)
}

fn leggi_archi(j: &Value) -> Result<Vec<Arco>> {
    let mut archi = Vec::new();
    for a in j.get("archi").map_or(&[][..], |a| a.arr()) {
        let (da, to) = (s(a, "da").ok_or_else(bad)?, s(a, "a").ok_or_else(bad)?);
        let p = a.get("perdita");
        archi.push(Arco {
            misure: a
                .get("misure")
                .map_or(&[][..], |m| m.arr())
                .iter()
                .map(|m| Misura {
                    livello: n(m, "livello").unwrap_or(0.0) as i64,
                    att: s(m, "att"),
                    ce: n(m, "ce"),
                    rel: n(m, "rel").unwrap_or(0.0),
                })
                .collect(),
            formula: p.and_then(|p| s(p, "formula")),
            da_massa: p.and_then(|p| n(p, "da_massa")).unwrap_or(0.0),
            compat: p
                .and_then(|p| p.get("compatibile"))
                .map_or(&[][..], |c| c.arr())
                .iter()
                .filter_map(|c| c.str().map(str::to_string))
                .collect(),
            regola: regola_di(&j, &da, &to),
            da,
            a: to,
        });
    }
    Ok(archi)
}

pub(crate) fn scena(graph: &str) -> Result<Scena> {
    let done = mzlab_core::msngrafo::completa(graph).map_err(|_| bad())?;
    let j = parse(done.as_bytes()).map_err(|_| bad())?;
    let vista = j.get("vista").ok_or_else(bad)?;
    let ioni = leggi_ioni(&j)?;
    let pos = leggi_pos(vista)?;
    let archi = leggi_archi(&j)?;
    let best = vista
        .get("archi")
        .map_or(&[][..], |a| a.arr())
        .iter()
        .filter(|a| a.get("best").and_then(|b| b.bool()) == Some(true))
        .map(|a| {
            format!(
                "{}>{}",
                s(a, "da").unwrap_or_default(),
                s(a, "a").unwrap_or_default()
            )
        })
        .collect();
    let o = j.get("opzioni");
    Ok(Scena {
        ioni,
        pos,
        archi,
        best,
        carica: if n(&j, "polarita") == Some(-1.0) {
            "-"
        } else {
            "+"
        }
        .into(),
        molfile: o
            .and_then(|o| s(o, "molfile"))
            .filter(|m| !m.trim().is_empty()),
        smiles: o
            .and_then(|o| s(o, "smiles"))
            .is_some_and(|m| !m.is_empty()),
    })
}

// ---- labels (the rule of msngrafo-vista.js) ------------------------------------------------------------------------------

/// A piece of a text: plain, subscript or superscript.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Pezzo {
    N(String),
    Sub(String),
    Sup(String),
}

/// The formula split into its plain text and its charge: «C13H25N4O3S+» → («C13H25N4O3S», «+»).
pub(crate) fn dividi_carica(f: &str, dflt: &str) -> (String, String) {
    let mut c = f.chars();
    match c.next_back() {
        Some(l @ ('+' | '-' | '−')) => (
            c.as_str().to_string(),
            if l == '+' { "+" } else { "-" }.into(),
        ),
        _ => (f.to_string(), dflt.to_string()),
    }
}

/// The formula as pieces to draw: digits are subscripts, the charge a superscript.
pub(crate) fn pezzi_formula(f: &str, carica: &str) -> Vec<Pezzo> {
    let mut out: Vec<Pezzo> = Vec::new();
    let mut cifre = String::new();
    let mut testo = String::new();
    for ch in f.chars() {
        if ch.is_ascii_digit() {
            if !testo.is_empty() {
                out.push(Pezzo::N(std::mem::take(&mut testo)));
            }
            cifre.push(ch);
        } else {
            if !cifre.is_empty() {
                out.push(Pezzo::Sub(std::mem::take(&mut cifre)));
            }
            testo.push(ch);
        }
    }
    if !testo.is_empty() {
        out.push(Pezzo::N(testo));
    }
    if !cifre.is_empty() {
        out.push(Pezzo::Sub(cifre));
    }
    if !carica.is_empty() {
        out.push(Pezzo::Sup(if carica == "-" { "−" } else { carica }.into()));
    }
    out
}

/// The plain form of the pieces (for the accessible names and the tooltips).
pub(crate) fn piano(p: &[Pezzo]) -> String {
    p.iter()
        .map(|x| match x {
            Pezzo::N(t) | Pezzo::Sub(t) | Pezzo::Sup(t) => t.as_str(),
        })
        .collect()
}

/// The digits of a formula as Unicode subscripts and the charge as a superscript, for the texts that take no markup.
pub(crate) fn bella(f: &str) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    f.chars()
        .map(|c| c.to_digit(10).map_or(c, |d| SUB[d as usize]))
        .collect()
}

pub(crate) fn segno(v: f64) -> String {
    let sg = if v > 0.0 {
        "+"
    } else if v < 0.0 {
        "−"
    } else {
        ""
    };
    format!("{sg}{:.2}", v.abs())
}

/// A number as JavaScript prints it: whole numbers without decimals.
pub(crate) fn js_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

pub(crate) fn livelli_testo(p: &Pos) -> String {
    p.livelli
        .iter()
        .map(|l| format!("MS{l}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn prova_testo(p: &str) -> String {
    match p {
        "perdita" => "known loss",
        "isotopi" => "isotopes",
        "msn" => "MSn",
        x => x,
    }
    .to_string()
}

pub(crate) struct Riga {
    pub(crate) parti: Vec<Pezzo>,
    pub(crate) ppm: String,
    pub(crate) on: bool,
    pub(crate) prove: Vec<String>,
}

pub(crate) struct Scatola {
    pub(crate) righe: Vec<Riga>,
    pub(crate) piu: String,
    pub(crate) rel: Option<f64>,
    pub(crate) aria: Vec<String>,
    pub(crate) amb: bool,
}

pub(crate) fn scelta_di(i: &Ion) -> Option<String> {
    i.scelta.clone().or_else(|| {
        if i.cands.len() == 1 {
            Some(i.cands[0].formula.clone())
        } else {
            None
        }
    })
}

pub(crate) fn scatola(sc: &Scena, i: &Ion, p: &Pos, compatta: bool) -> Scatola {
    let chosen = scelta_di(i);
    let mut cs: Vec<&Cand> = i.cands.iter().collect();
    cs.sort_by_key(|c| u8::from(Some(&c.formula) != chosen.as_ref()));
    let keep = if compatta { 1 } else { MAXC };
    let righe = cs
        .iter()
        .take(keep)
        .map(|c| {
            let (b, ch) = dividi_carica(&c.formula, &sc.carica);
            Riga {
                parti: pezzi_formula(&b, &ch),
                ppm: format!("{} ppm", segno(c.ppm)),
                on: Some(&c.formula) == chosen.as_ref(),
                prove: c.prove.iter().map(|x| prova_testo(x)).collect(),
            }
        })
        .collect();
    let piu = if cs.len() > keep {
        format!("+{} more", cs.len() - keep)
    } else {
        String::new()
    };
    let rel = if p.da.is_some() { p.rel } else { None };
    let mut aria = vec![
        format!("m/z {:.4}", i.mz),
        format!("levels {}", livelli_testo(p)),
    ];
    if let Some(r) = rel {
        aria.push(format!("{r:.1} % of the base peak"));
    }
    if let Some(c) = &chosen {
        let (b, ch) = dividi_carica(c, &sc.carica);
        aria.push(format!("formula {b}{ch}"));
    } else if !cs.is_empty() {
        aria.push(format!("{} candidate formulas", cs.len()));
    }
    if p.comune {
        aria.push(format!("reached from {} precursors", p.entranti));
    }
    Scatola {
        righe,
        piu,
        rel,
        aria,
        amb: i.has_formula && chosen.is_none(),
    }
}

pub(crate) fn tip_nodo(sc: &Scena, i: &Ion, p: &Pos) -> String {
    let mut out = vec![format!("m/z {:.5} · {}", i.mz, livelli_testo(p))];
    for a in sc.archi.iter().filter(|a| a.a == i.id) {
        for m in &a.misure {
            let att = m
                .att
                .as_ref()
                .map_or(String::new(), |x| format!(" {}", x.to_uppercase()));
            let ce = m.ce.map_or(String::new(), |c| format!(" {}", js_num(c)));
            out.push(format!("MS{}{att}{ce}: {:.1} %", m.livello, m.rel));
        }
    }
    out.join("\n")
}

/// The label of a loss: the pieces of the first line and, if any, of the second («(compatible with H2O)»).
pub(crate) fn perdita_testo(a: &Arco) -> (Vec<Pezzo>, Option<Vec<Pezzo>>) {
    if let Some(f) = &a.formula {
        let mut v = vec![Pezzo::N("−".into())];
        v.extend(pezzi_formula(f, ""));
        return (v, None);
    }
    let comp: Vec<&String> = a.compat.iter().take(2).collect();
    let mut second = None;
    if !comp.is_empty() {
        let mut v = vec![Pezzo::N("(compatible with ".into())];
        for (k, c) in comp.iter().enumerate() {
            if k > 0 {
                v.push(Pezzo::N(" / ".into()));
            }
            v.extend(pezzi_formula(c, ""));
        }
        v.push(Pezzo::N(")".into()));
        second = Some(v);
    }
    (vec![Pezzo::N(format!("Δ {:.3} Da", a.da_massa))], second)
}

pub(crate) fn tip_perdita(a: &Arco) -> String {
    if let Some(f) = &a.formula {
        return format!("−{}", bella(f));
    }
    let comp: Vec<String> = a.compat.iter().take(2).map(|c| bella(c)).collect();
    let extra = if comp.is_empty() {
        String::new()
    } else {
        format!(" (compatible with {})", comp.join(" / "))
    };
    format!("Δ {:.3} Da{extra}", a.da_massa)
}
