//! What a series lacks, as stable codes (the pages put the words), and what the TP path needs: indices by role and the minutes of
//! `feature_pipeline/1` (`genitore.minuti`: the time of a sample or a time zero, null otherwise; the infusion is never a control).

use super::Campione;
use crate::json::Out;

/// One finding: `code` (`timeMissing`, `noT0`, ...), `severity` (`error` blocks mzFinder, `warning`, `info`), the file (index) or the series.
#[derive(Debug, Clone, PartialEq)]
pub struct Avviso {
    pub code: &'static str,
    pub severity: &'static str,
    pub file: Option<usize>,
    pub series: String,
}

fn push(
    out: &mut Vec<Avviso>,
    code: &'static str,
    severity: &'static str,
    file: Option<usize>,
    series: &str,
) {
    out.push(Avviso {
        code,
        severity,
        file,
        series: series.to_string(),
    });
}

/// The checks of one file: a sample needs a confirmed time, a time zero has no other time.
fn file_checks(out: &mut Vec<Avviso>, i: usize, c: &Campione) {
    match c.role() {
        "sample" if c.time_min.is_none() => push(out, "timeMissing", "error", Some(i), &c.series),
        "sample" if !c.time_confirmed => push(out, "timeUnconfirmed", "error", Some(i), &c.series),
        "t0" if c.kind == "t0" && c.time_min.is_some_and(|t| t != 0.0) => {
            push(out, "t0WithTime", "error", Some(i), &c.series)
        }
        "t0" if c.kind == "sample" && !c.time_confirmed => {
            push(out, "timeUnconfirmed", "error", Some(i), &c.series)
        }
        _ => {}
    }
}

/// The checks of one series: the files of `series` and the ones of no series (shared by all).
fn series_checks(out: &mut Vec<Avviso>, cs: &[Campione], series: &str) {
    let member = |c: &&Campione| series.is_empty() || c.series.is_empty() || c.series == series;
    let has = |role: &str| cs.iter().filter(member).any(|c| c.role() == role);
    let treated: Vec<&Campione> = cs
        .iter()
        .filter(member)
        .filter(|c| c.role() == "sample" && c.time_min.is_some_and(|t| t > 0.0))
        .collect();
    let mut times: Vec<f64> = treated.iter().filter_map(|c| c.time_min).collect();
    super::keep_first(&mut times);
    if times.len() == 1 {
        push(out, "onePoint", "warning", None, series);
    }
    let (t0, blank, dark) = (has("t0"), has("blank"), has("dark"));
    if !t0 && !blank && !dark {
        push(out, "noControls", "error", None, series);
    } else {
        if !t0 {
            push(out, "noT0", "warning", None, series);
        }
        if !blank {
            push(out, "noBlank", "warning", None, series);
        }
        if !dark {
            push(out, "noDark", "info", None, series);
        }
    }
    let same = |a: &Campione, b: &Campione| a.time_min == b.time_min && a.conc == b.conc;
    if treated
        .iter()
        .enumerate()
        .any(|(i, a)| treated[..i].iter().any(|b| same(a, b)))
    {
        push(out, "replicates", "info", None, series);
    }
}

/// The findings of a series of files, errors first per file, then per series, then for the whole set.
pub fn valida(cs: &[Campione]) -> Vec<Avviso> {
    let mut out = Vec::new();
    for (i, c) in cs.iter().enumerate() {
        file_checks(&mut out, i, c);
    }
    let mut series: Vec<&str> = cs
        .iter()
        .filter(|c| c.role() == "sample")
        .map(|c| c.series.as_str())
        .collect();
    super::keep_first(&mut series);
    if series.is_empty() {
        push(&mut out, "noTreated", "error", None, "");
        if !cs
            .iter()
            .any(|c| matches!(c.role(), "t0" | "blank" | "dark"))
        {
            push(&mut out, "noControls", "error", None, "");
        }
    }
    for s in series.iter().filter(|s| !s.is_empty() || series.len() == 1) {
        series_checks(&mut out, cs, s);
    }
    if !cs.iter().any(|c| c.kind == "infusion") {
        push(&mut out, "noInfusion", "info", None, "");
    }
    out
}

/// `{"avvisi":[{"codice","gravita","file","serie"}],"bloccante"}`: `bloccante` when one finding is an error (mzFinder does not start).
pub fn valida_json(a: &[Avviso]) -> String {
    let mut j = Out::default();
    j.raw("{\"avvisi\":[");
    for (i, x) in a.iter().enumerate() {
        j.raw(if i > 0 {
            ",{\"codice\":"
        } else {
            "{\"codice\":"
        })
        .str(x.code);
        j.raw(",\"gravita\":").str(x.severity).raw(",\"file\":");
        match x.file {
            Some(f) => j.int(f as i64),
            None => j.raw("null"),
        };
        j.raw(",\"serie\":").str(&x.series).raw("}");
    }
    let blocks = a.iter().any(|x| x.severity == "error");
    j.raw("],\"bloccante\":")
        .raw(if blocks { "true" } else { "false" })
        .raw("}");
    j.0
}

/// Indices (0-based, in the order of the files) by role, and the minutes of `feature_pipeline/1`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerTp {
    /// Samples with a time over zero.
    pub treated: Vec<usize>,
    pub t0: Vec<usize>,
    pub blank: Vec<usize>,
    pub dark: Vec<usize>,
    pub infusion: Vec<usize>,
    /// Standards and calibration.
    pub std: Vec<usize>,
    pub qc: Vec<usize>,
    /// Samples without a usable time (`valida` reports them).
    pub untimed: Vec<usize>,
    /// The time of a treated file or 0 for a time zero; None for every other file (controls, infusion, standards, QC, untimed).
    pub minutes: Vec<Option<f64>>,
}

pub fn per_tp(cs: &[Campione]) -> PerTp {
    let mut p = PerTp::default();
    for (i, c) in cs.iter().enumerate() {
        let mut minutes = None;
        let list = match c.role() {
            "sample" if c.time_min.is_some_and(|t| t > 0.0) => {
                minutes = c.time_min;
                &mut p.treated
            }
            "sample" => &mut p.untimed,
            "t0" => {
                minutes = Some(0.0);
                &mut p.t0
            }
            "blank" => &mut p.blank,
            "dark" => &mut p.dark,
            "infusion" => &mut p.infusion,
            "qc" => &mut p.qc,
            _ => &mut p.std,
        };
        list.push(i);
        p.minutes.push(minutes);
    }
    p
}

fn ints(j: &mut Out, key: &str, v: &[usize]) {
    j.raw(",\"").raw(key).raw("\":[");
    for (i, x) in v.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).int(*x as i64);
    }
    j.raw("]");
}

/// `{"schema":"campioni/2","treated":[...],...,"minuti":[...]}`.
pub fn per_tp_json(p: &PerTp) -> String {
    let mut j = Out::default();
    j.raw("{\"schema\":\"campioni/2\"");
    for (k, v) in [
        ("treated", &p.treated),
        ("t0", &p.t0),
        ("blank", &p.blank),
        ("dark", &p.dark),
        ("infusion", &p.infusion),
        ("std", &p.std),
        ("qc", &p.qc),
        ("untimed", &p.untimed),
    ] {
        ints(&mut j, k, v);
    }
    j.raw(",\"minuti\":[");
    for (i, m) in p.minutes.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).opt(*m);
    }
    j.raw("]}");
    j.0
}
