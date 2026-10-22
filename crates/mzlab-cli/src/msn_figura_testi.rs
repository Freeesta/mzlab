//! The label rule of the static figure: the text pieces of an ion and of a loss, the same as `mzlab/web/msngrafo-vista.js`, and the boxes they make.

use super::scena::*;

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
