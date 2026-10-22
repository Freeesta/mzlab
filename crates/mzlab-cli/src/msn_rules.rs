//! `mzlab-cli msn-rules RECORD...`: the rules of `fragrules/1` judged on public library spectra (MassBank records, MSP) with a known SMILES. One line of
//! numbers per record (`msn_rules_prova`); `msn-rules somma` adds the lines up (`msn_rules_somma`). The protocol is `docs/studi/regole-frammentazione-piano.md` §3.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzlab_core::json::Out;
use mzlab_lib::parse::{detect, parse_all};

use self::prova::{attese_del_registro, decoy, misura, Registro};
use crate::Outcome;

mod prova;
mod somma;
#[cfg(test)]
mod tests;

pub const HELP: &str = "mzlab-cli msn-rules: the fragmentation rules judged on library spectra with a known structure.

usage: mzlab-cli msn-rules RECORD... [--lista FILE] [--smiles S] [--attivazione A] [--regole] [--decoy SEME] [--out PATH]
       mzlab-cli msn-rules somma LINES... [--out PATH]

  RECORD...         MassBank records (one spectrum each) or MSP files; --lista: a file with one path per line
  --smiles S        the structure, when the record has none (one record)
  --attivazione A   cid (trap type) or hcd (beam type), for a file that does not say (MSP)
  --regole          also the peaks explained with the rules of the formula and of the structure, and the peaks per rule id
  --decoy SEME      the same numbers on three decoys: the structure rewired (same formula), the spectrum of another record with the same nominal
                    precursor, the losses shifted by 0.0137 Da; SEME fixes the choice, the same seed gives the same lines
  somma             the lines of the records added up per stratum (trap CID, HCD, Q-TOF, other beam) and polarity: coverage of (c) against the decoys,
                    peaks per rule, decoy rate per rule and the thresholds of the protocol (a rule with fewer than 20 applications is not measured)

Peaks above 2 % of the base peak, 10 ppm (20 below m/z 150) for a fragment, 3 mDa for a loss. Output: a JSON line per record, numbers only.";

/// Adducts the graph reads for the ion of the precursor.
const ADDOTTI: [&str; 8] = [
    "[M+H]+",
    "[M-H]-",
    "[M+Na]+",
    "[M+NH4]+",
    "[M+K]+",
    "[M+Cl]-",
    "[M+HCOO]-",
    "[M+CH3COO]-",
];

struct Args {
    file: Vec<PathBuf>,
    lista: Option<PathBuf>,
    smiles: Option<String>,
    att: Option<String>,
    regole: bool,
    decoy: Option<u64>,
    out: Option<PathBuf>,
}

fn parse_args(mut a: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut r = Args {
        file: vec![],
        lista: None,
        smiles: None,
        att: None,
        regole: false,
        decoy: None,
        out: None,
    };
    while let Some(x) = a.next() {
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--lista" => r.lista = a.next().map(PathBuf::from),
            "--smiles" => r.smiles = a.next(),
            "--attivazione" => r.att = a.next(),
            "--regole" => r.regole = true,
            "--decoy" => {
                let s = a.next().and_then(|v| v.parse().ok());
                r.decoy = Some(s.ok_or("--decoy needs a number")?);
            }
            "--out" => r.out = a.next().map(PathBuf::from),
            f if !f.starts_with("--") => r.file.push(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    Ok(Some(r))
}

/// Why a record is left out (written in its line, counted by `somma`).
type Salto = &'static str;

/// The first number of a text («35 (nominal)», «20 eV»); 0 when there is none.
fn primo_numero(t: &str) -> f64 {
    let s: String = t
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    s.parse().unwrap_or(0.0)
}

/// The value after `KEY` in a line of the text (`AC$MASS_SPECTROMETRY: FRAGMENTATION_MODE HCD` → `HCD`).
fn campo(testo: &str, chiave: &str) -> Option<String> {
    testo.lines().find_map(|l| {
        let (_, v) = l.split_once(chiave)?;
        Some(v.trim().to_string())
    })
}

/// The stratum and the activation (`CID` trap type, `HCD` beam type) from the instrument and the fragmentation mode.
fn strato(strumento: &str, modo: &str) -> Option<(&'static str, &'static str)> {
    let s = strumento.to_ascii_uppercase();
    let tipo = s.rsplit('-').next().unwrap_or("");
    match modo.to_ascii_uppercase().as_str() {
        "HCD" => Some(("hcd", "HCD")),
        "CID" if tipo.starts_with("IT") || tipo.starts_with("QIT") => Some(("cid_trappola", "CID")),
        "CID" if tipo.contains("TOF") => Some(("qtof", "HCD")),
        "CID" => Some(("fascio", "HCD")),
        _ => None,
    }
}

fn leggi(p: &PathBuf, a: &Args) -> Result<Registro, Salto> {
    let dati = std::fs::read(p).map_err(|_| "file")?;
    let nome = p
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into());
    let fmt = detect(&nome, &dati).map_err(|_| "formato")?;
    let testo = String::from_utf8_lossy(&dati);
    let s = parse_all(fmt, &dati)
        .0
        .into_iter()
        .next()
        .ok_or("spettro")?;
    if campo(&testo, "MS_TYPE").is_some_and(|t| t != "MS2") {
        return Err("ms_tipo");
    }
    let modo = campo(&testo, "FRAGMENTATION_MODE")
        .or(a.att.clone())
        .ok_or("attivazione")?;
    let (strato, att) = strato(&s.meta.instrument, &modo).ok_or("attivazione")?;
    let smiles = a.smiles.clone().unwrap_or(s.meta.smiles.clone());
    if smiles.is_empty() || smiles == "N/A" {
        return Err("smiles");
    }
    if s.pol == 0
        || s.prec <= 0.0
        || s.meta.formula.is_empty()
        || !ADDOTTI.contains(&&s.meta.adduct[..])
    {
        return Err("ione");
    }
    Ok(Registro {
        id: if s.meta.accession.is_empty() {
            nome
        } else {
            s.meta.accession.clone()
        },
        strato,
        pol: s.pol,
        prec: s.prec,
        ce: primo_numero(&s.meta.ce),
        att,
        ione: format!("{} {}", s.meta.formula, s.meta.adduct),
        smiles,
        picchi: s.mz.iter().copied().zip(s.it.iter().copied()).collect(),
    })
}

/// The neutral formula of a record (the first word of its ion): the same compound at another energy, or an isomer, is not a decoy.
fn neutra(r: &Registro) -> &str {
    r.ione.split(' ').next().unwrap_or("")
}

/// The spectrum of another record of the batch with the same nominal precursor and polarity (the next one, in the order of the ids, with another
/// neutral formula and structure); `None` when there is none.
fn altro_spettro<'a>(r: &Registro, tutti: &'a [Registro]) -> Option<&'a [(f64, f64)]> {
    let gruppo: Vec<&Registro> = tutti
        .iter()
        .filter(|x| {
            x.pol == r.pol
                && x.prec.round() == r.prec.round()
                && (x.id == r.id || neutra(x) != neutra(r))
        })
        .collect();
    let i = gruppo.iter().position(|x| x.id == r.id)?;
    (1..gruppo.len())
        .map(|k| gruppo[(i + k) % gruppo.len()])
        .find(|x| x.smiles != r.smiles)
        .map(|x| &x.picchi[..])
}

fn riga(r: &Registro, tutti: &[Registro], a: &Args) -> String {
    let Some(attese) = attese_del_registro(r) else {
        return format!("{{\"id\":{:?},\"saltato\":\"smiles\"}}", r.id);
    };
    let mut o = Out::default();
    o.raw("{\"id\":")
        .str(&r.id)
        .raw(",\"strato\":")
        .str(r.strato);
    o.raw(",\"pol\":").int(r.pol as i64).raw(",\"v\":");
    misura(r, &r.picchi, &attese, a.regole).json(&mut o, a.regole);
    if let Some(seme) = a.decoy {
        let d = decoy(r, seme, altro_spettro(r, tutti), &attese);
        for (k, m) in [
            ("ds", &d.struttura),
            ("dp", &d.spettro),
            ("dl", &Some(d.perdite)),
        ] {
            if let Some(m) = m {
                o.raw(",").str(k).raw(":");
                m.json(&mut o, true);
            }
        }
    }
    o.raw("}");
    o.0
}

fn righe(a: &Args) -> Result<String, String> {
    let mut files = a.file.clone();
    if let Some(l) = &a.lista {
        let t = std::fs::read_to_string(l).map_err(|e| format!("{}: {e}", l.display()))?;
        files.extend(
            t.lines()
                .filter(|x| !x.trim().is_empty())
                .map(PathBuf::from),
        );
    }
    if files.is_empty() {
        return Err("a record is needed".into());
    }
    let (mut letti, mut saltati): (Vec<Registro>, BTreeMap<String, Salto>) =
        (vec![], BTreeMap::new());
    for f in &files {
        match leggi(f, a) {
            Ok(r) => letti.push(r),
            Err(s) => {
                saltati.insert(f.display().to_string(), s);
            }
        }
    }
    letti.sort_by(|x, y| x.id.cmp(&y.id));
    let mut out: Vec<String> = letti.iter().map(|r| riga(r, &letti, a)).collect();
    out.extend(
        saltati
            .iter()
            .map(|(f, s)| format!("{{\"id\":{f:?},\"saltato\":\"{s}\"}}")),
    );
    Ok(out.join("\n") + "\n")
}

pub fn run(args: Vec<String>) -> Outcome {
    let fallito = |m: String| Outcome {
        code: 2,
        stdout: vec![],
        stderr: format!("error: {m}\n(mzlab-cli msn-rules --help)\n"),
    };
    if args.first().map(String::as_str) == Some("somma") {
        return somma::run(args[1..].to_vec());
    }
    let a = match parse_args(args.into_iter()) {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Outcome {
                code: 0,
                stdout: format!("{HELP}\n").into_bytes(),
                stderr: String::new(),
            }
        }
        Err(e) => return fallito(e),
    };
    match righe(&a) {
        Err(e) => fallito(e),
        Ok(t) => somma::emetti(t, &a.out),
    }
}
