//! `mzlab-cli msn-rules RECORD...`: the rules of `fragrules/1` judged on public library spectra (MassBank records, MSP) with a known SMILES. One line of
//! numbers per record (`msn_rules_prova`); `msn-rules sum` adds the lines up (`msn_rules_somma`). The protocol is in `docs/agenti/msn-regole-prove.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzlab_core::json::Out;
use mzlab_lib::parse::{detect, parse_all};

use self::trial::{decoy, measure, registry_expectations, Registry};
use crate::msn_outputs::canonical;
use crate::Outcome;

mod sum;
#[cfg(test)]
mod tests;
mod trial;

/// Old flag and subcommand names of `msn-rules` (old, new); resolved before the match.
const ALIASES: &[(&str, &str)] = &[
    ("--lista", "--list"),
    ("--attivazione", "--activation"),
    ("--regole", "--rules"),
    ("somma", "sum"),
];

pub const HELP: &str = "mzlab-cli msn-rules: the fragmentation rules judged on library spectra with a known structure.

usage: mzlab-cli msn-rules RECORD... [--list FILE] [--smiles S] [--activation A] [--rules] [--decoy SEME] [--out PATH]
       mzlab-cli msn-rules sum LINES... [--out PATH]

  RECORD...         MassBank records (one spectrum each) or MSP files; --list: a file with one path per line
  --smiles S        the structure, when the record has none (one record)
  --activation A    cid (trap type) or hcd (beam type), for a file that does not say (MSP)
  --rules           also the peaks explained with the rules of the formula and of the structure, and the peaks per rule id
  --decoy SEME      the same numbers on three decoys: the structure rewired (same formula), the spectrum of another record with the same nominal
                    precursor, the losses shifted by 0.0137 Da; SEME fixes the choice, the same seed gives the same lines
  sum               the lines of the records added up per stratum (trap CID, HCD, Q-TOF, other beam) and polarity: coverage of (c) against the decoys,
                    peaks per rule, decoy rate per rule and the thresholds of the protocol (a rule with fewer than 20 applications is not measured)

Peaks above 2 % of the base peak, 10 ppm (20 below m/z 150) for a fragment, 3 mDa for a loss. Output: a JSON line per record, numbers only.

old names still accepted: --lista --attivazione --regole somma";

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
    list: Option<PathBuf>,
    smiles: Option<String>,
    att: Option<String>,
    rules: bool,
    decoy: Option<u64>,
    out: Option<PathBuf>,
}

fn parse_args(mut a: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut r = Args {
        file: vec![],
        list: None,
        smiles: None,
        att: None,
        rules: false,
        decoy: None,
        out: None,
    };
    while let Some(x) = a.next() {
        let x = canonical(ALIASES, x);
        match x.as_str() {
            "--help" | "-h" => return Ok(None),
            "--list" => r.list = a.next().map(PathBuf::from),
            "--smiles" => r.smiles = a.next(),
            "--activation" => r.att = a.next(),
            "--rules" => r.rules = true,
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
type Skip = &'static str;

/// The first number of a text («35 (nominal)», «20 eV»); 0 when there is none.
fn first_number(t: &str) -> f64 {
    let s: String = t
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    s.parse().unwrap_or(0.0)
}

/// The value after `KEY` in a line of the text (`AC$MASS_SPECTROMETRY: FRAGMENTATION_MODE HCD` → `HCD`).
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (_, v) = l.split_once(key)?;
        Some(v.trim().to_string())
    })
}

/// The stratum and the activation (`CID` trap type, `HCD` beam type) from the instrument and the fragmentation mode.
fn layer(instrument: &str, mode: &str) -> Option<(&'static str, &'static str)> {
    let s = instrument.to_ascii_uppercase();
    let kind = s.rsplit('-').next().unwrap_or("");
    match mode.to_ascii_uppercase().as_str() {
        "HCD" => Some(("hcd", "HCD")),
        "CID" if kind.starts_with("IT") || kind.starts_with("QIT") => Some(("cid_trappola", "CID")),
        "CID" if kind.contains("TOF") => Some(("qtof", "HCD")),
        "CID" => Some(("fascio", "HCD")),
        _ => None,
    }
}

fn read_registry(p: &PathBuf, a: &Args) -> Result<Registry, Skip> {
    let data = std::fs::read(p).map_err(|_| "file")?;
    let name = p
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into());
    let fmt = detect(&name, &data).map_err(|_| "formato")?;
    let text = String::from_utf8_lossy(&data);
    let s = parse_all(fmt, &data)
        .0
        .into_iter()
        .next()
        .ok_or("spettro")?;
    if field(&text, "MS_TYPE").is_some_and(|t| t != "MS2") {
        return Err("ms_tipo");
    }
    let mode = field(&text, "FRAGMENTATION_MODE")
        .or(a.att.clone())
        .ok_or("attivazione")?;
    let (layer_name, att) = layer(&s.meta.instrument, &mode).ok_or("attivazione")?;
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
    Ok(Registry {
        id: if s.meta.accession.is_empty() {
            name
        } else {
            s.meta.accession.clone()
        },
        layer_name,
        pol: s.pol,
        prec: s.prec,
        ce: first_number(&s.meta.ce),
        att,
        ione: format!("{} {}", s.meta.formula, s.meta.adduct),
        smiles,
        peaks: s.mz.iter().copied().zip(s.it.iter().copied()).collect(),
    })
}

/// The neutral formula of a record (the first word of its ion): the same compound at another energy, or an isomer, is not a decoy.
fn neutra(r: &Registry) -> &str {
    r.ione.split(' ').next().unwrap_or("")
}

/// The spectrum of another record of the batch with the same nominal precursor and polarity (the next one, in the order of the ids, with another
/// neutral formula and structure); `None` when there is none.
fn other_spectrum<'a>(r: &Registry, all: &'a [Registry]) -> Option<&'a [(f64, f64)]> {
    let group: Vec<&Registry> = all
        .iter()
        .filter(|x| {
            x.pol == r.pol
                && x.prec.round() == r.prec.round()
                && (x.id == r.id || neutra(x) != neutra(r))
        })
        .collect();
    let i = group.iter().position(|x| x.id == r.id)?;
    (1..group.len())
        .map(|k| group[(i + k) % group.len()])
        .find(|x| x.smiles != r.smiles)
        .map(|x| &x.peaks[..])
}

fn row(r: &Registry, all: &[Registry], a: &Args) -> String {
    let Some(expected) = registry_expectations(r) else {
        return format!("{{\"id\":{:?},\"saltato\":\"smiles\"}}", r.id);
    };
    let mut o = Out::default();
    o.raw("{\"id\":")
        .str(&r.id)
        .raw(",\"strato\":")
        .str(r.layer_name);
    o.raw(",\"pol\":").int(r.pol as i64).raw(",\"v\":");
    measure(r, &r.peaks, &expected, a.rules).json(&mut o, a.rules);
    if let Some(seed) = a.decoy {
        let d = decoy(r, seed, other_spectrum(r, all), &expected);
        for (k, m) in [
            ("ds", &d.structure),
            ("dp", &d.spectrum),
            ("dl", &Some(d.losses)),
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

fn rows(a: &Args) -> Result<String, String> {
    let mut files = a.file.clone();
    if let Some(l) = &a.list {
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
    let (mut read_items, mut skipped): (Vec<Registry>, BTreeMap<String, Skip>) =
        (vec![], BTreeMap::new());
    for f in &files {
        match read_registry(f, a) {
            Ok(r) => read_items.push(r),
            Err(s) => {
                skipped.insert(f.display().to_string(), s);
            }
        }
    }
    read_items.sort_by(|x, y| x.id.cmp(&y.id));
    let mut out: Vec<String> = read_items.iter().map(|r| row(r, &read_items, a)).collect();
    out.extend(
        skipped
            .iter()
            .map(|(f, s)| format!("{{\"id\":{f:?},\"saltato\":\"{s}\"}}")),
    );
    Ok(out.join("\n") + "\n")
}

pub fn run(args: Vec<String>) -> Outcome {
    let failed = |m: String| Outcome {
        code: 2,
        stdout: vec![],
        stderr: format!("error: {m}\n(mzlab-cli msn-rules --help)\n"),
    };
    if args
        .first()
        .map(|x| canonical(ALIASES, x.clone()))
        .as_deref()
        == Some("sum")
    {
        return sum::run(args[1..].to_vec());
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
        Err(e) => return failed(e),
    };
    match rows(&a) {
        Err(e) => failed(e),
        Ok(t) => sum::emit(t, &a.out),
    }
}
