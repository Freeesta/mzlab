//! Rules `R-rt-1` … `R-form-2` of `docs/studi/assistente-dati/03-parametri.md` §5: retention-time tolerance and alignment, the working window, the
//! filters of the table (blank, QC, detection, MS2), drift, isolation, centroiding and formulas. Used by `params_rules::propose`.

use mzlab_core::json::Value;

use crate::params_rules::{sg, Acc, Conf, Sig};
use crate::params_rules_det::num;

pub(crate) fn run(sig: &Sig, answers: &Value, acc: &mut Acc) {
    let qc = sig.series.get("qc").filter(|q| q.get("schema").is_some());
    if !rt_from_qc(qc, acc) {
        rt_gradient(sig, acc);
    }
    warp(qc, acc);
    window(sig, acc);
    blank(sig, acc);
    qc_filter(sig, answers, qc, acc);
    detected(sig, answers, acc);
    ms2(sig, answers, acc);
    drift(qc, acc);
    isolation(sig, acc);
    centroid(sig, acc);
    formula(sig, answers, acc);
}

fn qc_count(qc: Option<&Value>) -> f64 {
    qc.and_then(|q| q.get("qc")).map_or(0, |a| a.arr().len()) as f64
}

fn qc_conf(qc: Option<&Value>) -> Conf {
    let high = qc.and_then(|q| q.get("confidence")).and_then(Value::str) == Some("high");
    if high && qc_count(qc) >= 3.0 {
        Conf::High
    } else {
        Conf::Medium
    }
}

/// R-rt-1: the RT tolerance the QC files proposed (three robust deviations of the drift, never below the peak width); true when it fired.
fn rt_from_qc(qc: Option<&Value>, acc: &mut Acc) -> bool {
    let Some(gap) = qc
        .and_then(|q| q.get("proposal"))
        .and_then(|p| p.get("features.align.rt_gap"))
        .and_then(Value::f64)
    else {
        return false;
    };
    let signals = [sg("rt_gap_min", gap), sg("qc_files", qc_count(qc))];
    acc.emit("R-rt-1", "rt_gap", num(gap), &signals, qc_conf(qc), "qc");
    acc.emit("R-rt-1", "rt_tol", num(gap), &signals, qc_conf(qc), "qc");
    true
}

/// R-rt-2: without QC files, half a percent of the gradient (at least 0.05 min).
fn rt_gradient(sig: &Sig, acc: &mut Acc) {
    let Some(len) = gradient_min(sig) else {
        return acc.ask("gradient", "Q-gradient");
    };
    let tol = (0.005 * len).max(0.05);
    let signals = [sg("gradient_min", len)];
    acc.emit(
        "R-rt-2",
        "rt_gap",
        num(tol),
        &signals,
        Conf::Medium,
        "gradient",
    );
    acc.emit(
        "R-rt-2",
        "rt_tol",
        num(tol),
        &signals,
        Conf::Medium,
        "gradient",
    );
}

/// The length of the gradient in minutes: from its steps, else from the range of retention times of the file.
fn gradient_min(sig: &Sig) -> Option<f64> {
    let times: Vec<f64> = sig
        .file
        .get("gradient")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|p| p.get("t_min")?.f64())
        .collect();
    let span = |a: &[f64]| {
        a.iter().cloned().fold(f64::MIN, f64::max) - a.iter().cloned().fold(f64::MAX, f64::min)
    };
    if times.len() >= 2 {
        return Some(span(&times));
    }
    let r: Vec<f64> = sig
        .file
        .get("rt_range_min")?
        .arr()
        .iter()
        .filter_map(Value::f64)
        .collect();
    (r.len() == 2).then(|| span(&r))
}

/// R-warp-1: the warp of the retention times the QC files ask for.
fn warp(qc: Option<&Value>, acc: &mut Acc) {
    let Some(on) = qc
        .and_then(|q| q.get("proposal"))
        .and_then(|p| p.get("features.align.rt_warp"))
        .and_then(Value::bool)
    else {
        return;
    };
    let v = Value::Str(if on { "piecewise" } else { "off" }.into());
    acc.emit(
        "R-warp-1",
        "rt_warp",
        v,
        &[format!("rt_warp={on}"), sg("qc_files", qc_count(qc))],
        qc_conf(qc),
        "qc",
    );
}

/// R-win-1: the working window of a unit-resolution file starts after the dead time.
fn window(sig: &Sig, acc: &mut Acc) {
    if sig.hr != Some(false) {
        return;
    }
    if let Some(dead) = sig.num(&["dead_min"]) {
        acc.emit(
            "R-win-1",
            "rt_min",
            num(dead),
            &[sg("dead_min", dead)],
            Conf::Medium,
            "dead_time",
        );
    }
}

/// R-blank-1: three times the blank when the series has blanks, else the question.
fn blank(sig: &Sig, acc: &mut Acc) {
    let n = sig.role_count("blank");
    if n >= 1.0 {
        acc.emit(
            "R-blank-1",
            "blank_ratio",
            num(3.0),
            &[sg("blank_files", n)],
            Conf::Medium,
            "roles",
        );
    } else {
        acc.ask("blank_files", "Q-blank");
    }
}

/// R-qc-1: 30 % of CV in at least half of the QC files, with three or more QC; asked when a quantitative aim has none.
fn qc_filter(sig: &Sig, answers: &Value, qc: Option<&Value>, acc: &mut Acc) {
    let n = qc_count(qc).max(sig.role_count("qc"));
    if n >= 3.0 {
        let signals = [sg("qc_files", n)];
        acc.emit(
            "R-qc-1",
            "qc_cv_max",
            num(30.0),
            &signals,
            Conf::Medium,
            "roles",
        );
        acc.emit(
            "R-qc-1",
            "qc_min_files",
            num((n / 2.0).ceil().max(2.0)),
            &signals,
            Conf::Medium,
            "roles",
        );
    } else if matches!(
        answers.get("objective").and_then(Value::str),
        Some("nontarget" | "quant")
    ) {
        acc.ask("qc_files", "Q-qc");
    }
}

/// R-det-1: half of the smallest group (at least 2), the groups from the answers or the samples of the series as one.
fn detected(sig: &Sig, answers: &Value, acc: &mut Acc) {
    let sizes: Vec<f64> = answers
        .get("groups")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(Value::f64)
        .collect();
    let smallest = sizes.iter().cloned().fold(f64::MAX, f64::min);
    let (size, proof) = if sizes.is_empty() {
        (sig.role_count("sample"), "roles")
    } else {
        (smallest, "answer")
    };
    if size < 1.0 {
        return acc.ask("groups", "Q-groups");
    }
    acc.emit(
        "R-det-1",
        "min_detected",
        num((size / 2.0).floor().max(2.0)),
        &[sg("smallest_group", size)],
        Conf::Medium,
        proof,
    );
}

/// R-ms2-1: MS2 required only when the user wants to identify and the files have MS2 from a data-dependent method.
fn ms2(sig: &Sig, answers: &Value, acc: &mut Acc) {
    let Some(identify) = answers.get("identify").and_then(Value::bool) else {
        return acc.ask("identify", "Q-identify");
    };
    let dda = sig.texts("acquisition").contains(&"dda");
    if identify && !dda {
        acc.warnings.push("warn.params.no_ms2".into());
    }
    let signals = [format!("identify={identify}"), format!("dda={dda}")];
    acc.emit(
        "R-ms2-1",
        "require_ms2",
        Value::Bool(identify && dda),
        &signals,
        Conf::Medium,
        "answer",
    );
}

/// R-drift-1: the correction of the signal drift only with five or more QC files; a smoother span when there are many.
fn drift(qc: Option<&Value>, acc: &mut Acc) {
    let n = qc_count(qc);
    if n < 5.0 {
        return;
    }
    let signals = [sg("qc_files", n)];
    let method = if n >= 8.0 { "loess" } else { "linear" };
    acc.emit(
        "R-drift-1",
        "method",
        Value::Str(method.into()),
        &signals,
        Conf::Medium,
        "qc",
    );
    if method == "loess" {
        acc.emit("R-drift-1", "span", num(0.75), &signals, Conf::Medium, "qc");
    }
}

/// R-iso-1: half of the isolation window read from the method.
fn isolation(sig: &Sig, acc: &mut Acc) {
    let widths = sig
        .file
        .get("isolation")
        .and_then(|i| i.get("width_mz"))
        .map_or(&[][..], Value::arr);
    let Some(hi) = widths
        .iter()
        .filter_map(Value::f64)
        .reduce(f64::max)
        .filter(|w| *w > 0.0)
    else {
        return;
    };
    acc.emit(
        "R-iso-1",
        "default_half_window",
        num(hi / 2.0),
        &[sg("isolation_width_mz", hi)],
        Conf::High,
        "isolation",
    );
}

/// R-cent-1: the threshold of a profile file is its noise; the width in m/z is not measured yet, so it is asked.
fn centroid(sig: &Sig, acc: &mut Acc) {
    if sig.file.get("mode").and_then(Value::str) != Some("profile") {
        return;
    }
    acc.ask("fwhm_mz", "Q-centroid");
    if let Some(noise) = sig.num(&["noise"]) {
        acc.emit(
            "R-cent-1",
            "threshold",
            num(noise),
            &[sg("noise", noise), "mode=profile".into()],
            Conf::Medium,
            "noise",
        );
    }
}

/// R-form-1 and R-form-2: the formula tolerance is the one of the ions; the elements are CHNO unless the user names others.
fn formula(sig: &Sig, answers: &Value, acc: &mut Acc) {
    if sig.hr != Some(true) {
        return;
    }
    let ppm = acc
        .proposals
        .iter()
        .find(|p| p.get("name").and_then(Value::str) == Some("ppm"))
        .map(|p| {
            (
                p.get("value").cloned(),
                p.get("confidence").and_then(Value::str).map(String::from),
            )
        });
    if let Some((Some(v), conf)) = ppm {
        let c = if conf.as_deref() == Some("high") {
            Conf::High
        } else {
            Conf::Medium
        };
        acc.emit(
            "R-form-1",
            "tol_ppm",
            v.clone(),
            &[sg("ppm", v.f64().unwrap_or(0.0))],
            c,
            "ppm",
        );
        acc.emit(
            "R-form-1",
            "ppm_ion",
            v.clone(),
            &[sg("ppm", v.f64().unwrap_or(0.0))],
            c,
            "ppm",
        );
    }
    let named = answers
        .get("elements")
        .and_then(Value::str)
        .filter(|s| !s.is_empty());
    let (elements, conf) = named.map_or(("CHNO", Conf::Medium), |e| (e, Conf::High));
    acc.emit(
        "R-form-2",
        "elements",
        Value::Str(elements.into()),
        &[format!("elements={elements}")],
        conf,
        "answer",
    );
}
