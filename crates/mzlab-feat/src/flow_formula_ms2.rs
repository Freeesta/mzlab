//! The text of `formula.ms2@1`: sub-formulas of the candidate formulas (`formulas/1`) that explain the peaks of the MS2 spectra of an MGF (`review::explain`, `ms2_engine::rearrange`).
//! Takes names, bytes and the parameters and returns the text: `flow run` (`mzlab-cli`) and the browser (`mzlab-attributio-wasm`) run this one function.

use crate::sortc::SortC;
use mzlab_core::error::Result;
use mzlab_core::json::{self, round, Out, Value};
use mzlab_lib::flowmatch::{query_id, read_spectra, Named};

use crate::attributio::ms2_engine::rearrange::attribute;
use crate::attributio::review::explain::{
    explain, fmt_formula, match_adduct, parse_formula, prep_ms2, too_many_subformulas, Formula,
};
use crate::attributio::review::settings::ReviewSettings;
use crate::flow_ms2::num;

/// The warning of a candidate whose ion has more sub-formulas than `max_subformulas`: kept, with no fragment explained.
pub const WARN_SUB: &str = "warn.flow.formula.subformulas";

/// One candidate formula of a `formulas/1` file with the feature it belongs to.
struct Cand {
    feature: String,
    formula: String,
}

fn candidates(files: &[&[u8]]) -> Result<Vec<Cand>> {
    let mut all = Vec::new();
    for f in files {
        let v = json::parse(f)?;
        for feat in v.get("features").map(Value::arr).unwrap_or(&[]) {
            let id = feat.get("id").and_then(Value::str).unwrap_or("");
            let list = feat
                .get("result")
                .and_then(|r| r.get("candidates"))
                .map(Value::arr)
                .unwrap_or(&[]);
            for c in list {
                let neutral = c.get("neutral").and_then(Value::str);
                if let Some(formula) = neutral.or_else(|| c.get("formula").and_then(Value::str)) {
                    all.push(Cand {
                        feature: id.to_string(),
                        formula: formula.to_string(),
                    });
                }
            }
        }
    }
    Ok(all)
}

/// What one candidate explains of one spectrum.
struct Explained {
    adduct: String,
    ion: String,
    ppm: f64,
    frac: f64,
    decoy: Option<f64>,
    score: f64,
    too_many: bool,
    // m/z and intensity of the peak, formula, theoretical m/z, ppm, loss, hydrogens moved
    frags: Vec<(f64, f64, String, f64, f64, String, i32)>,
}

/// A candidate whose ion fits but whose sub-formula grid is over `max_subformulas`: kept with no fragment and the warning, as `subformula`
/// does, never dropped in silence; any other reason `attribute` gives nothing stays a candidate that is not listed.
fn over_the_limit(fd: &Formula, prec: f64, pol: &str, s: &ReviewSettings) -> Option<Explained> {
    let am = match_adduct(fd, prec, pol)?;
    if am.ppm.abs() > s.ppm_ion || !too_many_subformulas(&am.ion, s.max_subformulas) {
        return None;
    }
    Some(Explained {
        adduct: am.adduct,
        ion: fmt_formula(&am.ion),
        ppm: am.ppm,
        frac: 0.0,
        decoy: None,
        score: 0.0,
        too_many: true,
        frags: vec![],
    })
}

fn run_candidate(
    alg: &str,
    c: &Cand,
    prec: f64,
    pol: &str,
    peaks: (&[f64], &[f64]),
    s: &ReviewSettings,
) -> Option<Explained> {
    let fd = parse_formula(&c.formula);
    if alg == "h_rearrangement" {
        let Some(a) = attribute(&fd, prec, pol, peaks, s) else {
            return over_the_limit(&fd, prec, pol, s);
        };
        let ppm = match_adduct(&fd, prec, pol).map_or(0.0, |m| m.ppm);
        let frags = a
            .hits
            .iter()
            .enumerate()
            .filter_map(|(i, h)| {
                h.as_ref().map(|h| {
                    (
                        peaks.0[i],
                        peaks.1[i],
                        h.formula.clone(),
                        h.mz_th,
                        h.ppm,
                        h.loss.clone(),
                        h.shift,
                    )
                })
            })
            .collect();
        return Some(Explained {
            adduct: a.adduct,
            ion: a.ion_formula,
            ppm,
            frac: a.frac_explained,
            decoy: None,
            score: a.score,
            too_many: false,
            frags,
        });
    }
    let e = explain(Some(&fd), prec, pol, peaks, s);
    if !e.fits(s) {
        return None;
    }
    let frags = e
        .frags
        .iter()
        .enumerate()
        .filter_map(|(i, h)| {
            h.as_ref().map(|h| {
                (
                    peaks.0[i],
                    peaks.1[i],
                    h.formula.clone(),
                    h.mz_th,
                    h.ppm,
                    h.loss.clone(),
                    0,
                )
            })
        })
        .collect();
    let ion = e.ion.as_ref().map(fmt_formula).unwrap_or_default();
    Some(Explained {
        adduct: e.adduct.unwrap_or_default(),
        ion,
        ppm: e.ion_ppm,
        frac: e.frac_expl,
        decoy: Some(e.decoy_frac),
        score: e.frac_expl,
        too_many: e.too_many,
        frags,
    })
}

fn write_explained(o: &mut Out, c: &Cand, x: &Explained) {
    o.raw("{\"feature\":")
        .str(&c.feature)
        .raw(",\"formula\":")
        .str(&c.formula)
        .raw(",\"adduct\":")
        .str(&x.adduct);
    o.raw(",\"ion_formula\":")
        .str(&x.ion)
        .raw(",\"ppm_ion\":")
        .num(round(x.ppm, 3));
    o.raw(",\"n_explained\":")
        .int(x.frags.len() as i64)
        .raw(",\"frac_explained\":")
        .num(round(x.frac, 4));
    o.raw(",\"decoy_frac\":")
        .opt(x.decoy.map(|d| round(d, 4)))
        .raw(",\"score\":")
        .num(round(x.score, 4));
    if x.too_many {
        o.raw(",\"warning\":").str(WARN_SUB);
    }
    o.raw(",\"fragments\":[");
    for (k, f) in x.frags.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"mz\":")
            .num(round(f.0, 5))
            .raw(",\"intensity\":")
            .num(round(f.1, 3));
        o.raw(",\"formula\":")
            .str(&f.2)
            .raw(",\"mz_th\":")
            .num(round(f.3, 5))
            .raw(",\"ppm\":")
            .num(round(f.4, 3));
        o.raw(",\"loss\":")
            .str(&f.5)
            .raw(",\"shift\":")
            .int(f.6 as i64)
            .raw("}");
    }
    o.raw("]}");
}

/// `formula.ms2`: every candidate formula whose ion fits the precursor of a spectrum explains its cleaned peaks with sub-formulas.
/// `algorithm` is the id the plan picked (`subformula`, `h_rearrangement`); empty is the default of the step.
pub fn ms2_text(
    algorithm: &str,
    params: &[(String, Value)],
    spectra: Named,
    formulas: &[&[u8]],
) -> Result<String> {
    let alg = if algorithm.is_empty() {
        "subformula"
    } else {
        algorithm
    };
    let s = ReviewSettings {
        ppm_ion: num(params, "ppm_ion"),
        ppm_frag: num(params, "ppm_frag"),
        max_subformulas: num(params, "max_subformulas") as i64,
        ..ReviewSettings::default()
    };
    let cands = candidates(formulas)?;
    let mut out = Out::default();
    out.raw("{\"format\":\"formulas/1\",\"step\":\"formula.ms2@1\",\"algorithm\":")
        .str(alg)
        .raw(",\"spectra\":[");
    for (k, sp) in read_spectra(spectra)?.iter().enumerate() {
        let pol = if sp.pol < 0 { "NEG" } else { "POS" };
        let (m, y, _) = prep_ms2(&sp.mz, &sp.it, sp.prec, s.rel_min);
        let mut found: Vec<(&Cand, Explained)> = cands
            .iter()
            .filter(|_| !m.is_empty())
            .filter_map(|c| run_candidate(alg, c, sp.prec, pol, (&m, &y), &s).map(|x| (c, x)))
            .collect();
        found.sortc_by(|a, b| {
            b.1.score
                .total_cmp(&a.1.score)
                .then(a.0.formula.cmp(&b.0.formula))
        });
        out.raw(if k > 0 { "," } else { "" })
            .raw("{\"query\":")
            .str(&query_id(k, sp))
            .raw(",\"precursor\":")
            .num(sp.prec);
        out.raw(",\"peaks\":")
            .int(m.len() as i64)
            .raw(",\"candidates\":[");
        for (i, (c, x)) in found.iter().enumerate() {
            out.raw(if i > 0 { "," } else { "" });
            write_explained(&mut out, c, x);
        }
        out.raw("]}");
    }
    out.raw("]}");
    Ok(out.0)
}
