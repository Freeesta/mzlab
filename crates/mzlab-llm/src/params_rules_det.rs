//! Rules `R-pol-1` … `R-gap-1` of `docs/studi/assistente-dati/03-parametri.md` §5: polarity, m/z axis and tolerances, noise, peak width and points.
//! One function per rule, each reading only the signals of `ispezione/1` it names. Used by `params_rules::propose`.

use mzlab_core::json::Value;

use crate::params_rules::{sg, Acc, Conf, Sig};

pub(crate) fn num(x: f64) -> Value {
    Value::Num(x)
}

/// `x` rounded up to a multiple of `step`.
pub(crate) fn up(x: f64, step: f64) -> f64 {
    ((x / step) - 1e-9).ceil() * step
}

pub(crate) fn run(sig: &Sig, answers: &Value, acc: &mut Acc) {
    pol(sig, acc);
    axis(sig, acc);
    if !ppm_measured(sig, acc) {
        ppm_model(sig, acc);
    }
    da(sig, acc);
    floor(sig, acc);
    height(sig, acc);
    snr(sig, answers, acc);
    let width_s = sig.num(&["peak_width", "fwhm_min"]).map(|m| m * 60.0);
    width(sig, width_s, acc);
    points(sig, width_s, acc);
    gap(sig, acc);
}

/// R-pol-1: the polarity of the series when there is only one; mixed or alternating files are a warning and a question.
fn pol(sig: &Sig, acc: &mut Acc) {
    let pols = sig
        .series
        .get("polarities")
        .map_or(&[][..], Value::arr)
        .len();
    let own = sig.file.get("polarity").and_then(Value::str).unwrap_or("");
    if pols > 1 || own == "alternating" {
        acc.warnings.push("warn.params.mixed_polarity".into());
        acc.ask("polarity", "Q-pol");
        return;
    }
    let v = match own {
        "positive" => 1.0,
        "negative" => -1.0,
        _ => return acc.ask("polarity", "Q-pol"),
    };
    acc.emit(
        "R-pol-1",
        "polarity",
        num(v),
        &[format!("polarity={own}")],
        Conf::High,
        "polarity",
    );
}

/// R-axis-1: Da on a unit-resolution file, ppm on a high-resolution one (the registry calls the unit axis `Da`).
fn axis(sig: &Sig, acc: &mut Acc) {
    let Some(hr) = sig.hr else {
        return acc.ask("resolution", "Q-resolution");
    };
    let basis = sig
        .texts("class_basis")
        .first()
        .copied()
        .unwrap_or("")
        .to_string();
    let class = if hr { "high" } else { "unit" };
    let conf = if basis.is_empty() {
        Conf::Medium
    } else {
        Conf::High
    };
    let signals = [format!("class={class}"), format!("class_basis={basis}")];
    acc.emit(
        "R-axis-1",
        "axis",
        Value::Str(if hr { "ppm" } else { "Da" }.into()),
        &signals,
        conf,
        "class",
    );
}

fn mass_error(sig: &Sig, unit: &str) -> Option<(f64, Option<f64>, f64)> {
    let m = sig.file.get("mass_error")?;
    (m.get("unit").and_then(Value::str) == Some(unit)).then_some(())?;
    let value = m.get("value")?.f64()?;
    Some((
        value,
        m.get("sigma").and_then(Value::f64),
        m.get("n").and_then(Value::f64).unwrap_or(0.0),
    ))
}

/// R-ppm-1: three sigma of the measured error, up to half a ppm, between 2 and 30 ppm; true when it fired.
fn ppm_measured(sig: &Sig, acc: &mut Acc) -> bool {
    let Some((value, Some(sigma), n)) = mass_error(sig, "ppm").filter(|_| sig.hr == Some(true))
    else {
        return false;
    };
    let tol = up(3.0 * sigma, 0.5).clamp(2.0, 30.0);
    let conf = if n >= 30.0 { Conf::High } else { Conf::Medium };
    acc.emit(
        "R-ppm-1",
        "ppm",
        num(tol),
        &[
            sg("mass_error_ppm", value),
            sg("sigma_ppm", sigma),
            sg("n", n),
        ],
        conf,
        "mass_error",
    );
    true
}

/// R-ppm-2: without a measure, the typical tolerance of the analyzer named by the file.
fn ppm_model(sig: &Sig, acc: &mut Acc) {
    if sig.hr != Some(true) {
        return;
    }
    let model = sig
        .file
        .get("model")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string();
    let low = model.to_lowercase();
    let (tol, conf) = if ["orbitrap", "exploris", "exactive", "fusion", "astral"]
        .iter()
        .any(|k| low.contains(k))
    {
        (5.0, Conf::Medium)
    } else if low.contains("tof") {
        (10.0, Conf::Medium)
    } else {
        (10.0, Conf::Low)
    };
    acc.emit(
        "R-ppm-2",
        "ppm",
        num(tol),
        &[format!("model={model}")],
        conf,
        "model",
    );
}

/// R-da-1: the Da tolerance of a unit-resolution file from its measured error, else the typical 0.35 Da as a question.
fn da(sig: &Sig, acc: &mut Acc) {
    if sig.hr != Some(false) {
        return;
    }
    match mass_error(sig, "Da") {
        Some((value, sigma, n)) => {
            let tol = up(value.abs() + 3.0 * sigma.unwrap_or(0.0), 0.05).clamp(0.1, 1.0);
            let conf = if n >= 30.0 { Conf::High } else { Conf::Medium };
            acc.emit(
                "R-da-1",
                "tol_da",
                num(tol),
                &[sg("mass_error_da", value), sg("n", n)],
                conf,
                "mass_error",
            );
        }
        None => acc.emit(
            "R-da-1",
            "tol_da",
            num(0.35),
            &["typical=unit_resolution".into()],
            Conf::Low,
            "typical",
        ),
    }
}

/// R-floor-1: the noise measured in the file is the floor of detection.
fn floor(sig: &Sig, acc: &mut Acc) {
    if let Some(noise) = sig.num(&["noise"]) {
        acc.emit(
            "R-floor-1",
            "floor",
            num(noise),
            &[sg("noise", noise)],
            Conf::Medium,
            "noise",
        );
    }
}

/// R-height-1: ten times the noise, never below three times.
fn height(sig: &Sig, acc: &mut Acc) {
    if let Some(noise) = sig.num(&["noise"]) {
        acc.emit(
            "R-height-1",
            "min_height",
            num(10.0 * noise),
            &[sg("noise", noise)],
            Conf::Medium,
            "noise",
        );
    }
}

/// R-snr-1: 10 as a rule, 5 when the user looks for weak signals.
fn snr(_sig: &Sig, answers: &Value, acc: &mut Acc) {
    match answers.get("weak_signals").and_then(Value::bool) {
        Some(weak) => acc.emit(
            "R-snr-1",
            "snr",
            num(if weak { 5.0 } else { 10.0 }),
            &[format!("weak_signals={weak}")],
            Conf::Medium,
            "answer",
        ),
        None => acc.emit(
            "R-snr-1",
            "snr",
            num(10.0),
            &["typical=snr".into()],
            Conf::Low,
            "typical",
        ),
    }
}

/// R-width-1: from half to twice the median FWHM, in seconds (the percentiles of the widths are not in `ispezione/1`, so it never reaches `high`).
fn width(sig: &Sig, width_s: Option<f64>, acc: &mut Acc) {
    let Some(w) = width_s else {
        return acc.ask("peak_width", "Q-width");
    };
    let n = sig.num(&["peak_width", "n"]).unwrap_or(0.0);
    let v = Value::Arr(vec![num(0.5 * w), num(2.0 * w)]);
    acc.emit(
        "R-width-1",
        "peak_width_s",
        v,
        &[sg("fwhm_median_s", w), sg("traces", n)],
        Conf::Medium,
        "peak_width",
    );
}

/// R-pts-1: half the points of a peak, at least 3; points = FWHM / MS1 cycle.
fn points(sig: &Sig, width_s: Option<f64>, acc: &mut Acc) {
    let (Some(w), Some(cycle)) = (width_s, sig.num(&["cycle_ms1_s"])) else {
        return acc.ask("cycle", "Q-cycle");
    };
    let pts = w / cycle;
    let min = ((pts / 2.0).floor()).max(3.0);
    acc.emit(
        "R-pts-1",
        "min_points",
        num(min),
        &[
            sg("fwhm_median_s", w),
            sg("cycle_ms1_s", cycle),
            sg("points", pts),
        ],
        Conf::Medium,
        "cycle",
    );
}

/// R-gap-1: one missing scan for a regular cycle, two for a top-N method with more than five MS2 scans per cycle.
fn gap(sig: &Sig, acc: &mut Acc) {
    let dda = sig.texts("acquisition").contains(&"dda");
    let per = sig.num(&["ms2_per_cycle"]).unwrap_or(0.0);
    let v = if dda && per > 5.0 { 2.0 } else { 1.0 };
    acc.emit(
        "R-gap-1",
        "max_gap",
        num(v),
        &[format!("dda={dda}"), sg("ms2_per_cycle", per)],
        Conf::Medium,
        "cycle",
    );
}
