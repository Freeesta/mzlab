//! `mzlab-cli msn-rules somma LINES...`: the lines of `msn-rules` added up per stratum (instrument and activation) and polarity: the coverage of the
//! rules of the structure against each decoy, the decoy rate of every rule and the thresholds of the protocol. All the arithmetic of the validation is here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::msngrafo::{tabella_regole, Tipo};

use crate::Outcome;

/// A rule with fewer applications than this in a stratum is «not measured».
const MINIME: usize = 20;
/// A rule whose decoy rate is above this goes to priority 3.
const TASSO_MAX: f64 = 0.30;
/// The coverage (c) of the true records must lose at most this share of itself against every decoy that applies, on average.
const DIFFERENZA_MIN: f64 = 0.5;
/// A record with a smaller share of its intensity explained by sub-formulas (a) is nominal mass: counted apart, outside the 10 ppm test.
const NOMINALE: f64 = 0.5;
const DECOY: [&str; 3] = ["ds", "dp", "dl"];

#[derive(Default)]
struct Strato {
    registri: usize,
    /// Records of nominal mass (a below `NOMINALE`), left out of everything else.
    nominali: usize,
    /// Mean-to-be fractions of the intensity: (a), (b), (c) of the true record.
    frazioni: Vec<[f64; 3]>,
    /// Per decoy kind: (c of the true record, c of the decoy) of the records that have it.
    coppie: [Vec<(f64, f64)>; 3],
    /// Peaks per rule id: true, then the three decoys.
    regole: BTreeMap<String, [usize; 4]>,
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::f64).unwrap_or(0.0)
}

fn media(v: impl Iterator<Item = f64>) -> f64 {
    let (n, s) = v.fold((0, 0.0), |(n, s), x| (n + 1, s + x));
    if n == 0 {
        0.0
    } else {
        s / n as f64
    }
}

fn frazione(m: &Value, k: &str) -> Option<f64> {
    let t = num(m, "it");
    (t > 0.0).then(|| num(m, k) / t)
}

fn aggiungi_regole(s: &mut Strato, m: &Value, posto: usize) {
    if let Some(Value::Obj(r)) = m.get("r") {
        for (id, n) in r {
            s.regole.entry(id.clone()).or_default()[posto] += n.f64().unwrap_or(0.0) as usize;
        }
    }
}

fn leggi_riga(s: &mut Strato, v: &Value) {
    let Some(vero) = v.get("v") else { return };
    let a = frazione(vero, "ia").unwrap_or(0.0);
    if a < NOMINALE {
        s.nominali += 1;
        return;
    }
    s.registri += 1;
    aggiungi_regole(s, vero, 0);
    let Some(c) = frazione(vero, "ic") else {
        return;
    };
    s.frazioni.push([a, frazione(vero, "ib").unwrap_or(0.0), c]);
    for (k, nome) in DECOY.iter().enumerate() {
        if let Some(d) = v.get(nome) {
            aggiungi_regole(s, d, k + 1);
            s.coppie[k].push((c, frazione(d, "ic").unwrap_or(0.0)));
        }
    }
}

/// The decoy rate of a rule: the largest of decoys / true among the decoys that apply to it (the rewired structure only to a rule with a site or a
/// cleavage: a rule that asks for a group or an element passes it by construction, the rewiring keeps the groups).
fn tasso(id: &str, n: &[usize; 4]) -> f64 {
    let legge = tabella_regole()
        .regole
        .iter()
        .find(|r| r.id == id)
        .is_none_or(|r| r.site.is_some() || r.tipo == Tipo::Cleavage);
    (1..4)
        .filter(|&k| k > 1 || legge)
        .map(|k| n[k] as f64 / n[0].max(1) as f64)
        .fold(0.0, f64::max)
}

fn esito(n: &[usize; 4], t: f64) -> &'static str {
    if n[0] < MINIME {
        "non_misurata"
    } else if t > TASSO_MAX {
        "priorita_3"
    } else {
        "ok"
    }
}

fn scrivi_strato(o: &mut Out, (strato, pol): &(String, i64), s: &Strato) {
    let r3 = |x: f64| round(x, 3);
    let col = |i: usize| media(s.frazioni.iter().map(|f| f[i]));
    o.raw("{\"strato\":").str(strato).raw(",\"pol\":").int(*pol);
    o.raw(",\"registri\":").int(s.registri as i64);
    o.raw(",\"nominali\":").int(s.nominali as i64);
    o.raw(",\"copertura\":{\"a\":").num(r3(col(0)));
    o.raw(",\"b\":")
        .num(r3(col(1)))
        .raw(",\"c\":")
        .num(r3(col(2)));
    o.raw("},\"decoy\":{");
    let mut passa = None;
    for (k, nome) in DECOY.iter().enumerate() {
        let c = &s.coppie[k];
        let (v, d) = (media(c.iter().map(|p| p.0)), media(c.iter().map(|p| p.1)));
        o.raw(if k > 0 { "," } else { "" }).str(nome);
        o.raw(":{\"registri\":").int(c.len() as i64);
        o.raw(",\"vero\":").num(r3(v)).raw(",\"decoy\":").num(r3(d));
        o.raw(",\"differenza\":").num(r3(v - d)).raw("}");
        if !c.is_empty() {
            passa = Some(passa.unwrap_or(true) && v - d >= DIFFERENZA_MIN * v);
        }
    }
    o.raw("},\"soglia_copertura\":");
    o.raw(match passa {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    });
    o.raw(",\"regole\":[");
    for (i, (id, n)) in s.regole.iter().enumerate() {
        let t = tasso(id, n);
        o.raw(if i > 0 { "," } else { "" }).raw("{\"id\":").str(id);
        o.raw(",\"vero\":")
            .int(n[0] as i64)
            .raw(",\"ds\":")
            .int(n[1] as i64);
        o.raw(",\"dp\":")
            .int(n[2] as i64)
            .raw(",\"dl\":")
            .int(n[3] as i64);
        o.raw(",\"tasso\":")
            .num(r3(t))
            .raw(",\"esito\":")
            .str(esito(n, t))
            .raw("}");
    }
    o.raw("]}");
}

/// The summary of the lines `testo`: one JSON object, the strata ordered by name and polarity.
pub fn somma(testo: &str) -> Result<String, String> {
    let mut strati: BTreeMap<(String, i64), Strato> = BTreeMap::new();
    let mut saltati: BTreeMap<String, usize> = BTreeMap::new();
    for l in testo.lines().filter(|l| !l.trim().is_empty()) {
        let v = parse(l.as_bytes()).map_err(|_| format!("not a JSON line: {l}"))?;
        if let Some(s) = v.get("saltato").and_then(Value::str) {
            *saltati.entry(s.to_string()).or_default() += 1;
            continue;
        }
        let chiave = (
            v.get("strato")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            num(&v, "pol") as i64,
        );
        leggi_riga(strati.entry(chiave).or_default(), &v);
    }
    let mut o = Out::default();
    o.raw("{\"soglie\":{\"minime\":").int(MINIME as i64);
    o.raw(",\"tasso_max\":").num(TASSO_MAX);
    o.raw(",\"differenza_min\":").num(DIFFERENZA_MIN);
    o.raw("},\"saltati\":{");
    for (i, (k, n)) in saltati.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(k)
            .raw(":")
            .int(*n as i64);
    }
    o.raw("},\"strati\":[");
    for (i, (k, s)) in strati.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        scrivi_strato(&mut o, k, s);
    }
    Ok(o.raw("]}\n").0.clone())
}

/// The text to the standard output, or to the file of `--out`.
pub fn emetti(t: String, out: &Option<PathBuf>) -> Outcome {
    let (stdout, stderr) = match out {
        None => (t.into_bytes(), String::new()),
        Some(p) => match std::fs::write(p, t) {
            Ok(()) => (vec![], String::new()),
            Err(e) => return fallito(format!("{}: {e}", p.display())),
        },
    };
    Outcome {
        code: 0,
        stdout,
        stderr,
    }
}

fn fallito(m: String) -> Outcome {
    Outcome {
        code: 2,
        stdout: vec![],
        stderr: format!("error: {m}\n(mzlab-cli msn-rules --help)\n"),
    }
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut testo, mut out, mut a) = (String::new(), None, args.into_iter());
    while let Some(x) = a.next() {
        if x == "--out" {
            out = a.next().map(PathBuf::from);
        } else if let Err(e) = std::fs::read_to_string(&x).map(|t| testo.push_str(&t)) {
            return fallito(format!("{x}: {e}"));
        }
    }
    match somma(&testo) {
        Ok(t) => emetti(t, &out),
        Err(e) => fallito(e),
    }
}
