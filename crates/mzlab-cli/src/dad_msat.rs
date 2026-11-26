//! The honest answers of `dad ms-at` and `dad delay`: the MS scan nearest to a time (a point window matches no scan, so the old answer was an empty
//! spectrum with exit 0), and the refusal to hand back an empty spectrum or a delay with no peak behind it. Failures are exit 1 (`err.dad.*`).

use mzlab_core::error::{Error, Result};
pub(crate) use mzlab_core::scanpick::{nearest, Pick};

/// `Ok` when the spectrum answer has peaks; `err.dad.msEmpty` (the scan chosen, nothing in it) otherwise.
pub(crate) fn has_peaks(spectrum: &str, pick: &Pick, level: u8) -> Result<()> {
    let n = mzlab_core::json::parse(spectrum.as_bytes())
        .ok()
        .and_then(|v| v.get("mz").map(|m| m.arr().len()))
        .unwrap_or(0);
    if n > 0 {
        return Ok(());
    }
    Err(Error::new("err.dad.msEmpty")
        .with("level", level)
        .with("scan_rt", pick.rt))
}

/// The `/api/delay` answer as it is when there is evidence behind it; otherwise the reason as an error: no matrix (`err.pda.none`), traces too short to
/// compare (`err.dad.delayShort`) or an estimate that no UV peak matches in the TIC (`err.dad.delayNoPeaks`, with the delay and correlation found).
pub(crate) fn delay_evidence(answer: String, has_uv: bool) -> Result<String> {
    let v =
        mzlab_core::json::parse(answer.as_bytes()).map_err(|_| Error::new("err.dad.delayShort"))?;
    let Some(d) = v.get("delay").filter(|d| d.get("delay").is_some()) else {
        return Err(Error::new(if has_uv {
            "err.dad.delayShort"
        } else {
            "err.pda.none"
        }));
    };
    if d.get("peaks").and_then(|p| p.f64()).unwrap_or(0.0) >= 1.0 {
        return Ok(answer);
    }
    let num = |k: &str| d.get(k).and_then(|x| x.f64()).unwrap_or(0.0);
    Err(Error::new("err.dad.delayNoPeaks")
        .with("delay", num("delay"))
        .with("corr", num("corr")))
}

/// The offset a `dad peaks` answer used, as a line for stderr: the user sees which delay was applied and on what evidence.
pub(crate) fn offset_note(answer: &str) -> String {
    let Some(v) = mzlab_core::json::parse(answer.as_bytes()).ok() else {
        return String::new();
    };
    let Some(o) = v.get("offset") else {
        return String::new();
    };
    let num = |k: &str| o.get(k).and_then(|x| x.f64());
    let min = num("min").unwrap_or(0.0);
    match o.get("source").and_then(|s| s.str()) {
        Some("delay") => format!(
            "dad: offset auto = {min} min (delay estimate: {} peaks match the TIC, correlation {})\n",
            num("peaks").unwrap_or(0.0),
            num("corr").unwrap_or(0.0)
        ),
        Some("none") => "dad: offset auto: no UV peak matches the TIC, 0 min used (warning delay_unmatched)\n".to_string(),
        _ => format!("dad: offset {min} min (given)\n"),
    }
}

/// The stderr line of `dad delay`: the estimate is evidence only, so say where a number of minutes is used (never a changed value of the answer).
pub(crate) fn delay_note(answer: &str) -> String {
    let min = mzlab_core::json::parse(answer.as_bytes())
        .ok()
        .and_then(|v| {
            v.get("delay")
                .and_then(|d| d.get("delay"))
                .and_then(|x| x.f64())
        });
    min.map_or_else(String::new, |m| {
        format!("dad: delay {m} min, not applied; use it with `dad ms-at FILE --rt T --offset {m}` (`dad peaks` applies it by default, --offset auto)\n")
    })
}

/// `dad peaks --format csv`: a row per ion (a peak without ions has one row with the ion columns empty); the numbers are the answer's own.
pub(crate) fn peaks_csv(answer: &str) -> Result<String> {
    let bad = || Error::new("error.badIndex");
    let v = mzlab_core::json::parse(answer.as_bytes()).map_err(|_| bad())?;
    let mut out =
        String::from("peak,rt_uv_min,height_mau,lambda_max_nm,rt_ms_min,mz,intensity,rel_pct\n");
    for p in v.get("peaks").ok_or_else(bad)?.arr() {
        let num = |k: &str| {
            p.get(k)
                .and_then(|x| x.f64())
                .map_or(String::new(), |x| x.to_string())
        };
        let head = format!(
            "{},{},{},{},{}",
            num("n"),
            num("rt_uv"),
            num("height"),
            num("lambda_max_nm"),
            num("rt_ms")
        );
        let ions = p.get("ions").map_or(&[][..], |i| i.arr());
        if ions.is_empty() {
            out += &format!("{head},,,\n");
        }
        for i in ions {
            let f = |k: &str| {
                i.get(k)
                    .and_then(|x| x.f64())
                    .map_or(String::new(), |x| x.to_string())
            };
            out += &format!("{head},{},{},{}\n", f("mz"), f("y"), f("rel"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_with_no_matching_peak_or_no_evidence_is_refused() {
        let ok = "{\"delay\":{\"delay\":0.002,\"peaks\":2,\"corr\":0.9}}".to_string();
        assert_eq!(delay_evidence(ok.clone(), true).unwrap(), ok);
        let none = "{\"delay\":{\"delay\":0.1,\"peaks\":0,\"corr\":0.2}}".to_string();
        assert_eq!(
            delay_evidence(none, true).unwrap_err().key,
            "err.dad.delayNoPeaks"
        );
        let null = "{\"delay\":null}".to_string();
        assert_eq!(
            delay_evidence(null.clone(), false).unwrap_err().key,
            "err.pda.none"
        );
        assert_eq!(
            delay_evidence(null, true).unwrap_err().key,
            "err.dad.delayShort"
        );
    }
}
