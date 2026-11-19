//! `mzlab-cli msn rules sum DOC...`: the documents (`msn-rules/1`) of `msn rules` added up per stratum (instrument and activation) and polarity: the coverage of the
//! rules of the structure against each decoy, the decoy rate of every rule and the thresholds of the protocol. All the arithmetic of the validation is here.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mzlab_core::json::{parse, round, Out, Value};
use mzlab_core::msngraph::{tabella_regole, Tipo};

use crate::Outcome;

/// A rule with fewer applications than this in a stratum is «not measured».
const MIN_COUNT: usize = 20;
/// A rule whose decoy rate is above this goes to priority 3.
const MAX_RATE: f64 = 0.30;
/// The coverage (c) of the true records must lose at most this share of itself against every decoy that applies, on average.
const MIN_DIFFERENCE: f64 = 0.5;
/// A record with a smaller share of its intensity explained by sub-formulas (a) is nominal mass: counted apart, outside the 10 ppm test.
const NOMINAL: f64 = 0.5;
const DECOY: [&str; 3] = ["ds", "dp", "dl"];

#[derive(Default)]
struct Layer {
    registries: usize,
    /// Records of nominal mass (a below `NOMINALE`), left out of everything else.
    nominal: usize,
    /// Mean-to-be fractions of the intensity: (a), (b), (c) of the true record.
    fractions: Vec<[f64; 3]>,
    /// Per decoy kind: (c of the true record, c of the decoy) of the records that have it.
    pairs: [Vec<(f64, f64)>; 3],
    /// Peaks per rule id: true, then the three decoys.
    rules: BTreeMap<String, [usize; 4]>,
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::f64).unwrap_or(0.0)
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (n, s) = v.fold((0, 0.0), |(n, s), x| (n + 1, s + x));
    if n == 0 {
        0.0
    } else {
        s / n as f64
    }
}

fn fraction(m: &Value, k: &str) -> Option<f64> {
    let t = num(m, "it");
    (t > 0.0).then(|| num(m, k) / t)
}

fn add_rules(s: &mut Layer, m: &Value, place: usize) {
    if let Some(Value::Obj(r)) = m.get("r") {
        for (id, n) in r {
            s.rules.entry(id.clone()).or_default()[place] += n.f64().unwrap_or(0.0) as usize;
        }
    }
}

fn read_line(s: &mut Layer, v: &Value) {
    let Some(real) = v.get("v") else { return };
    let a = fraction(real, "ia").unwrap_or(0.0);
    if a < NOMINAL {
        s.nominal += 1;
        return;
    }
    s.registries += 1;
    add_rules(s, real, 0);
    let Some(c) = fraction(real, "ic") else {
        return;
    };
    s.fractions
        .push([a, fraction(real, "ib").unwrap_or(0.0), c]);
    for (k, name) in DECOY.iter().enumerate() {
        if let Some(d) = v.get(name) {
            add_rules(s, d, k + 1);
            s.pairs[k].push((c, fraction(d, "ic").unwrap_or(0.0)));
        }
    }
}

/// The decoy rate of a rule: the largest of decoys / true among the decoys that apply to it (the rewired structure only to a rule with a site or a
/// cleavage: a rule that asks for a group or an element passes it by construction, the rewiring keeps the groups).
fn tasso(id: &str, n: &[usize; 4]) -> f64 {
    let reads = tabella_regole()
        .rules
        .iter()
        .find(|r| r.id == id)
        .is_none_or(|r| r.site.is_some() || r.tipo == Tipo::Cleavage);
    (1..4)
        .filter(|&k| k > 1 || reads)
        .map(|k| n[k] as f64 / n[0].max(1) as f64)
        .fold(0.0, f64::max)
}

fn outcome(n: &[usize; 4], t: f64) -> &'static str {
    if n[0] < MIN_COUNT {
        "not_measured"
    } else if t > MAX_RATE {
        "priority_3"
    } else {
        "ok"
    }
}

fn write_layer(o: &mut Out, (layer_name, pol): &(String, i64), s: &Layer) {
    let r3 = |x: f64| round(x, 3);
    let col = |i: usize| mean(s.fractions.iter().map(|f| f[i]));
    o.raw("{\"layer\":")
        .str(layer_name)
        .raw(",\"pol\":")
        .int(*pol);
    o.raw(",\"records\":").int(s.registries as i64);
    o.raw(",\"nominal\":").int(s.nominal as i64);
    o.raw(",\"coverage\":{\"a\":").num(r3(col(0)));
    o.raw(",\"b\":")
        .num(r3(col(1)))
        .raw(",\"c\":")
        .num(r3(col(2)));
    o.raw("},\"decoy\":{");
    let mut passes = None;
    for (k, name) in DECOY.iter().enumerate() {
        let c = &s.pairs[k];
        let (v, d) = (mean(c.iter().map(|p| p.0)), mean(c.iter().map(|p| p.1)));
        o.raw(if k > 0 { "," } else { "" }).str(name);
        o.raw(":{\"records\":").int(c.len() as i64);
        o.raw(",\"real\":").num(r3(v)).raw(",\"decoy\":").num(r3(d));
        o.raw(",\"difference\":").num(r3(v - d)).raw("}");
        if !c.is_empty() {
            passes = Some(passes.unwrap_or(true) && v - d >= MIN_DIFFERENCE * v);
        }
    }
    o.raw("},\"coverage_threshold\":");
    o.raw(match passes {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    });
    o.raw(",\"rules\":[");
    for (i, (id, n)) in s.rules.iter().enumerate() {
        let t = tasso(id, n);
        o.raw(if i > 0 { "," } else { "" }).raw("{\"id\":").str(id);
        o.raw(",\"real\":")
            .int(n[0] as i64)
            .raw(",\"ds\":")
            .int(n[1] as i64);
        o.raw(",\"dp\":")
            .int(n[2] as i64)
            .raw(",\"dl\":")
            .int(n[3] as i64);
        o.raw(",\"rate\":")
            .num(r3(t))
            .raw(",\"outcome\":")
            .str(outcome(n, t))
            .raw("}");
    }
    o.raw("]}");
}

/// The summary of the `msn-rules/1` documents `docs`: one JSON object, the strata ordered by name and polarity.
pub fn summed(docs: &[String]) -> Result<String, String> {
    let mut layers: BTreeMap<(String, i64), Layer> = BTreeMap::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let mut records = Vec::new();
    for d in docs {
        let doc = parse(d.as_bytes())
            .map_err(|_| "err.cli.line message=not an msn-rules/1 document".to_string())?;
        if doc.get("schema").and_then(Value::str) != Some("msn-rules/1") {
            return Err("err.cli.line message=not an msn-rules/1 document".to_string());
        }
        records.extend(doc.get("records").map_or(&[][..], Value::arr).to_vec());
    }
    for v in records {
        if let Some(s) = v.get("skipped").and_then(Value::str) {
            *skipped.entry(s.to_string()).or_default() += 1;
            continue;
        }
        let key = (
            v.get("layer")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            num(&v, "pol") as i64,
        );
        read_line(layers.entry(key).or_default(), &v);
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"msn-rules-sum/1\",\"thresholds\":{\"min_count\":")
        .int(MIN_COUNT as i64);
    o.raw(",\"max_rate\":").num(MAX_RATE);
    o.raw(",\"min_difference\":").num(MIN_DIFFERENCE);
    o.raw("},\"skipped\":{");
    for (i, (k, n)) in skipped.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(k)
            .raw(":")
            .int(*n as i64);
    }
    o.raw("},\"layers\":[");
    for (i, (k, s)) in layers.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        write_layer(&mut o, k, s);
    }
    Ok(o.raw("]}\n").0.clone())
}

/// The text to the standard output, or to the file of `--out`.
pub fn emit(t: String, out: &Option<PathBuf>) -> Outcome {
    let (stdout, stderr) = match out {
        None => (t.into_bytes(), String::new()),
        Some(p) => match std::fs::write(p, t) {
            Ok(()) => (vec![], String::new()),
            Err(e) => {
                return Outcome::fail_error(
                    &mzlab_core::Error::new("err.cli.out")
                        .with("path", format!("{}: {e}", p.display())),
                )
            }
        },
    };
    Outcome {
        code: 0,
        stdout,
        stderr,
        error: None,
    }
}

fn failed(m: String) -> Outcome {
    Outcome::fail(&m)
}

pub fn run(args: Vec<String>) -> Outcome {
    let (mut docs, mut out, mut a) = (Vec::new(), None, args.into_iter());
    while let Some(x) = a.next() {
        if x == "--out" {
            out = a.next().map(PathBuf::from);
        } else if x.starts_with("--") {
            return Outcome::fail(&format!("unknown option {x}"));
        } else {
            match std::fs::read_to_string(&x) {
                Ok(t) => docs.push(t),
                Err(e) => return failed(format!("{x}: {e}")),
            }
        }
    }
    if docs.is_empty() {
        return failed("a msn-rules/1 document is needed".to_string());
    }
    match summed(&docs) {
        Ok(t) => emit(t, &out),
        Err(e) => failed(e),
    }
}
