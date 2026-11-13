//! The label rule of the static figure: the text pieces of an ion and of a loss, the same as `mzlab/web/msngrafo-vista.js`, and the boxes they make.

use super::scene::*;
use crate::sortc::SortC;

// ---- labels (the rule of msngrafo-vista.js) ------------------------------------------------------------------------------

/// A piece of a text: plain, subscript or superscript.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Piece {
    N(String),
    Sub(String),
    Sup(String),
}

/// The formula split into its plain text and its charge: «C13H25N4O3S+» → («C13H25N4O3S», «+»).
pub(crate) fn split_charge(f: &str, dflt: &str) -> (String, String) {
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
pub(crate) fn piece_formula(f: &str, charge: &str) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut digits = String::new();
    let mut text = String::new();
    for ch in f.chars() {
        if ch.is_ascii_digit() {
            if !text.is_empty() {
                out.push(Piece::N(std::mem::take(&mut text)));
            }
            digits.push(ch);
        } else {
            if !digits.is_empty() {
                out.push(Piece::Sub(std::mem::take(&mut digits)));
            }
            text.push(ch);
        }
    }
    if !text.is_empty() {
        out.push(Piece::N(text));
    }
    if !digits.is_empty() {
        out.push(Piece::Sub(digits));
    }
    if !charge.is_empty() {
        out.push(Piece::Sup(if charge == "-" { "−" } else { charge }.into()));
    }
    out
}

/// The plain form of the pieces (for the accessible names and the tooltips).
pub(crate) fn plane(p: &[Piece]) -> String {
    p.iter()
        .map(|x| match x {
            Piece::N(t) | Piece::Sub(t) | Piece::Sup(t) => t.as_str(),
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

pub(crate) fn sign(v: f64) -> String {
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

pub(crate) fn level_text(p: &Pos) -> String {
    p.livelli
        .iter()
        .map(|l| format!("MS{l}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn trial_text(p: &str) -> String {
    match p {
        "perdita" => "known loss",
        "isotopi" => "isotopes",
        "msn" => "MSn",
        x => x,
    }
    .to_string()
}

pub(crate) struct Row {
    pub(crate) parti: Vec<Piece>,
    pub(crate) ppm: String,
    pub(crate) on: bool,
    pub(crate) evidence: Vec<String>,
}

pub(crate) struct Scatola {
    pub(crate) righe: Vec<Row>,
    pub(crate) more: String,
    pub(crate) rel: Option<f64>,
    pub(crate) aria: Vec<String>,
    pub(crate) amb: bool,
}

pub(crate) fn choice_of(i: &Ion) -> Option<String> {
    i.choice.clone().or_else(|| {
        if i.cands.len() == 1 {
            Some(i.cands[0].formula.clone())
        } else {
            None
        }
    })
}

pub(crate) fn scatola(sc: &Scena, i: &Ion, p: &Pos, compact: bool) -> Scatola {
    let chosen = choice_of(i);
    let mut cs: Vec<&Cand> = i.cands.iter().collect();
    cs.sortc_by_key(|c| u8::from(Some(&c.formula) != chosen.as_ref()));
    let keep = if compact { 1 } else { MAXC };
    let rows = cs
        .iter()
        .take(keep)
        .map(|c| {
            let (b, ch) = split_charge(&c.formula, &sc.charge);
            Row {
                parti: piece_formula(&b, &ch),
                ppm: format!("{} ppm", sign(c.ppm)),
                on: Some(&c.formula) == chosen.as_ref(),
                evidence: c.evidence.iter().map(|x| trial_text(x)).collect(),
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
        format!("levels {}", level_text(p)),
    ];
    if let Some(r) = rel {
        aria.push(format!("{r:.1} % of the base peak"));
    }
    if let Some(c) = &chosen {
        let (b, ch) = split_charge(c, &sc.charge);
        aria.push(format!("formula {b}{ch}"));
    } else if !cs.is_empty() {
        aria.push(format!("{} candidate formulas", cs.len()));
    }
    if p.common {
        aria.push(format!("reached from {} precursors", p.entranti));
    }
    Scatola {
        righe: rows,
        more: piu,
        rel,
        aria,
        amb: i.has_formula && chosen.is_none(),
    }
}

pub(crate) fn node_tip(sc: &Scena, i: &Ion, p: &Pos) -> String {
    let mut out = vec![format!("m/z {:.5} · {}", i.mz, level_text(p))];
    for a in sc.edges.iter().filter(|a| a.a == i.id) {
        for m in &a.measures {
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
pub(crate) fn loss_text(a: &Edge) -> (Vec<Piece>, Option<Vec<Piece>>) {
    if let Some(f) = &a.formula {
        let mut v = vec![Piece::N("−".into())];
        v.extend(piece_formula(f, ""));
        return (v, None);
    }
    let comp: Vec<&String> = a.compat.iter().take(2).collect();
    let mut second = None;
    if !comp.is_empty() {
        let mut v = vec![Piece::N("(compatible with ".into())];
        for (k, c) in comp.iter().enumerate() {
            if k > 0 {
                v.push(Piece::N(" / ".into()));
            }
            v.extend(piece_formula(c, ""));
        }
        v.push(Piece::N(")".into()));
        second = Some(v);
    }
    (vec![Piece::N(format!("Δ {:.3} Da", a.from_mass))], second)
}

pub(crate) fn loss_tip(a: &Edge) -> String {
    if let Some(f) = &a.formula {
        return format!("−{}", bella(f));
    }
    let comp: Vec<String> = a.compat.iter().take(2).map(|c| bella(c)).collect();
    let extra = if comp.is_empty() {
        String::new()
    } else {
        format!(" (compatible with {})", comp.join(" / "))
    };
    format!("Δ {:.3} Da{extra}", a.from_mass)
}
