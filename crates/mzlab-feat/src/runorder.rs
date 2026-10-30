//! The text of `qc.run_order@1`: the facts of the injection sequence of the sample table (`campioni/2` or `/3`) per batch: the count of each role, the largest gap between two QC injections,
//! the blanks and the distinct standard levels, plus `warn.qc.runorder.*` keys. It reports facts and never a verdict. Bytes in, text out: `flow run` (`mzlab-cli`) and the browser module
//! `flowtab` (`mzlab-feat-wasm`) both call it, so they write the same `runorder/1` bytes.

use crate::drift::batches_of;
use crate::samples::Sample;
use mzlab_core::campioni::{self, Campione};
use mzlab_core::disegno::{self, LOTTO_PREDEFINITO};
use mzlab_core::error::Result;
use mzlab_core::json::{parse, Out, Value};

const QC_GAP: &str = "warn.qc.runorder.qc_gap";
const NO_QC: &str = "warn.qc.runorder.no_qc";
const NO_BLANK: &str = "warn.qc.runorder.no_blank_batch";
const STD_LEVELS: &str = "warn.qc.runorder.std_levels";
const ORDER_MISSING: &str = "warn.qc.runorder.order_missing";
const ORDER_DUPLICATE: &str = "warn.qc.runorder.order_duplicate";
/// The keys in the order the warnings of a batch are listed.
const KEY_ORDER: [&str; 6] = [
    ORDER_MISSING,
    ORDER_DUPLICATE,
    NO_QC,
    QC_GAP,
    NO_BLANK,
    STD_LEVELS,
];

/// The limits of the step.
struct Limits {
    qc_every_max: u32,
    std_levels_min: u32,
    blank_per_batch: bool,
}

fn limits_of(params: &[(String, Value)]) -> Limits {
    let param = |name: &str| params.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let whole = |name: &str, default: u32| {
        param(name)
            .and_then(Value::f64)
            .filter(|x| *x >= 0.0 && x.fract() == 0.0 && *x < 4e9)
            .map_or(default, |x| x as u32)
    };
    Limits {
        qc_every_max: whole("qc_every_max", 10),
        std_levels_min: whole("std_levels_min", 2),
        blank_per_batch: param("blank_per_batch")
            .and_then(Value::bool)
            .unwrap_or(true),
    }
}

/// One warning: its key, the batch it concerns (empty for the whole table), the number it measured and the files involved.
struct Warning {
    key: &'static str,
    batch: String,
    value: Option<u32>,
    files: Vec<String>,
}

/// The samples of the file with the files that gave no usable injection order (they are set to 1 so the reader accepts them, and left out of every order check).
fn read_samples(bytes: &[u8]) -> Result<(Vec<Campione>, Vec<bool>)> {
    let mut doc = parse(bytes)?;
    let mut missing = Vec::new();
    if let Value::Obj(fields) = &mut doc {
        if let Some((_, Value::Arr(items))) = fields.iter_mut().find(|(k, _)| k == "campioni") {
            for item in items {
                let Value::Obj(f) = item else {
                    missing.push(false);
                    continue;
                };
                let usable = f
                    .iter()
                    .find(|(k, _)| k == "ordine_iniezione")
                    .and_then(|(_, v)| v.f64())
                    .is_some_and(|x| x >= 1.0 && x.fract() == 0.0 && x < 4e9);
                if !usable {
                    f.retain(|(k, _)| k != "ordine_iniezione");
                    f.push(("ordine_iniezione".into(), Value::Num(1.0)));
                }
                missing.push(!usable);
            }
        }
    }
    Ok((campioni::from_value(&doc)?, missing))
}

/// The identity `batches_of` matches a file of the design by: its name and SHA-256.
fn sample_of(c: &Campione) -> Sample {
    Sample {
        file: c.file.clone(),
        sha256: c.sha256.clone(),
        kind: c.role().to_string(),
        time_min: c.time_min,
        replicate: c.replicate,
        group: c.series.clone(),
        conc: c.conc,
        unit: c.unit.clone(),
        order: c.order,
        istd: c.istd,
        note: c.note.clone(),
    }
}

/// The distinct values of `v`, ascending.
fn distinct(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v.dedup();
    v
}

fn names(files: &[(u32, &Campione)]) -> Vec<String> {
    files.iter().map(|(_, c)| c.file.clone()).collect()
}

/// The `runorder/1` text for the sample table `samples` (`campioni/2` or `/3`); the batches come from `design` (`disegno/1`) when it is given, else from the batch of each file, else one batch.
/// `params` are the parameters of the step.
pub fn runorder_text(
    params: &[(String, Value)],
    samples: &[u8],
    design: Option<&[u8]>,
) -> Result<String> {
    let lim = limits_of(params);
    let (cs, missing) = read_samples(samples)?;
    let batch: Vec<String> = match design {
        Some(d) => {
            let named: Vec<Sample> = cs.iter().map(sample_of).collect();
            batches_of(&named, &disegno::from_json(d)?)
        }
        None => cs
            .iter()
            .map(|c| {
                if c.batch.is_empty() {
                    LOTTO_PREDEFINITO.to_string()
                } else {
                    c.batch.clone()
                }
            })
            .collect(),
    };
    let mut warnings: Vec<Warning> = Vec::new();

    let lost: Vec<String> = sorted_names(
        cs.iter()
            .zip(&missing)
            .filter(|(_, m)| **m)
            .map(|(c, _)| c.file.clone()),
    );
    if !lost.is_empty() {
        warnings.push(Warning {
            key: ORDER_MISSING,
            batch: String::new(),
            value: Some(lost.len() as u32),
            files: lost,
        });
    }
    let mut by_order: Vec<(u32, &str)> = cs
        .iter()
        .zip(&missing)
        .filter(|(_, m)| !**m)
        .map(|(c, _)| (c.order, c.file.as_str()))
        .collect();
    by_order.sort();
    for run in by_order.chunk_by(|a, b| a.0 == b.0).filter(|r| r.len() > 1) {
        warnings.push(Warning {
            key: ORDER_DUPLICATE,
            batch: String::new(),
            value: Some(run[0].0),
            files: run.iter().map(|(_, f)| f.to_string()).collect(),
        });
    }

    let mut batches: Vec<String> = batch.clone();
    batches.sort();
    batches.dedup();
    let mut o = Out::default();
    o.raw("{\"schema\":\"runorder/1\",\"params\":{\"qc_every_max\":")
        .int(lim.qc_every_max as i64)
        .raw(",\"std_levels_min\":")
        .int(lim.std_levels_min as i64)
        .raw(",\"blank_per_batch\":")
        .raw(if lim.blank_per_batch { "true" } else { "false" })
        .raw("},\"batches\":[");
    for (n, name) in batches.iter().enumerate() {
        let mut files: Vec<(u32, &Campione)> = cs
            .iter()
            .zip(&missing)
            .zip(&batch)
            .filter(|((_, m), b)| *b == name && !**m)
            .map(|((c, _), _)| (c.order, c))
            .collect();
        files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.file.cmp(&b.1.file)));
        let unordered: Vec<&Campione> = cs
            .iter()
            .zip(&missing)
            .zip(&batch)
            .filter(|((_, m), b)| *b == name && **m)
            .map(|((c, _), _)| c)
            .collect();
        let of_role = |roles: &[&str]| -> Vec<(u32, &Campione)> {
            files
                .iter()
                .filter(|(_, c)| roles.contains(&c.role()))
                .copied()
                .collect()
        };
        let all_files = sorted_names(
            files
                .iter()
                .map(|(_, c)| c.file.clone())
                .chain(unordered.iter().map(|c| c.file.clone())),
        );
        let (qc, blank, std) = (
            of_role(&["qc"]),
            of_role(&["blank"]),
            of_role(&["std", "calibration"]),
        );
        let gaps: Vec<(u32, &str, &str)> = qc
            .windows(2)
            .map(|w| (w[1].0 - w[0].0, w[0].1.file.as_str(), w[1].1.file.as_str()))
            .collect();
        let widest = gaps.iter().max_by_key(|g| g.0).copied();
        let levels = distinct(std.iter().filter_map(|(_, c)| c.conc).collect());

        let n_files = files.len() + unordered.len();
        if qc.is_empty() {
            warnings.push(Warning {
                key: NO_QC,
                batch: name.clone(),
                value: None,
                files: all_files.clone(),
            });
        }
        if let Some((gap, from, to)) = widest.filter(|g| g.0 > lim.qc_every_max) {
            warnings.push(Warning {
                key: QC_GAP,
                batch: name.clone(),
                value: Some(gap),
                files: vec![from.to_string(), to.to_string()],
            });
        }
        if lim.blank_per_batch && blank.is_empty() {
            warnings.push(Warning {
                key: NO_BLANK,
                batch: name.clone(),
                value: None,
                files: all_files,
            });
        }
        if (levels.len() as u32) < lim.std_levels_min {
            warnings.push(Warning {
                key: STD_LEVELS,
                batch: name.clone(),
                value: Some(levels.len() as u32),
                files: sorted_names(std.iter().map(|(_, c)| c.file.clone())),
            });
        }

        o.raw(if n > 0 { ",{\"batch\":" } else { "{\"batch\":" })
            .str(name)
            .raw(",\"files\":")
            .int(n_files as i64)
            .raw(",\"roles\":{");
        let mut roles: Vec<(&str, i64)> = Vec::new();
        for c in files
            .iter()
            .map(|(_, c)| *c)
            .chain(unordered.iter().copied())
        {
            match roles.iter_mut().find(|(r, _)| *r == c.role()) {
                Some(e) => e.1 += 1,
                None => roles.push((c.role(), 1)),
            }
        }
        roles.sort_by_key(|(r, _)| campioni::TIPI.iter().position(|t| t == r));
        for (k, (role, count)) in roles.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" })
                .str(role)
                .raw(":")
                .int(*count);
        }
        o.raw("},\"first\":");
        match files.first() {
            Some((order, _)) => o.int(*order as i64),
            None => o.raw("null"),
        };
        o.raw(",\"last\":");
        match files.last() {
            Some((order, _)) => o.int(*order as i64),
            None => o.raw("null"),
        };
        o.raw(",\"qc_gap_max\":");
        match widest {
            Some((gap, ..)) => o.int(gap as i64),
            None => o.raw("null"),
        };
        o.raw(",\"blanks\":");
        strings(&mut o, &names(&blank));
        o.raw(",\"std_levels\":[");
        for (k, level) in levels.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).num(*level);
        }
        o.raw("]}");
    }
    o.raw("],\"warnings\":[");
    warnings.sort_by(|a, b| {
        a.batch
            .cmp(&b.batch)
            .then_with(|| rank(a.key).cmp(&rank(b.key)))
            .then_with(|| a.files.cmp(&b.files))
    });
    for (k, w) in warnings.iter().enumerate() {
        o.raw(if k > 0 { ",{\"key\":" } else { "{\"key\":" })
            .str(w.key)
            .raw(",\"batch\":")
            .str(&w.batch)
            .raw(",\"value\":");
        match w.value {
            Some(v) => o.int(v as i64),
            None => o.raw("null"),
        };
        o.raw(",\"files\":");
        strings(&mut o, &w.files);
        o.raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

fn rank(key: &str) -> usize {
    KEY_ORDER
        .iter()
        .position(|k| *k == key)
        .unwrap_or(KEY_ORDER.len())
}

fn sorted_names(it: impl Iterator<Item = String>) -> Vec<String> {
    let mut v: Vec<String> = it.collect();
    v.sort();
    v
}

fn strings(o: &mut Out, list: &[String]) {
    o.raw("[");
    for (k, s) in list.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

#[cfg(test)]
#[path = "runorder_tests.rs"]
mod tests;
