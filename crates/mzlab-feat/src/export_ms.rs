//! MS2 exports of the features that have a spectrum: the MGF that FBMN reads (`FEATURE_ID` links a block to the row of the table) and the `.ms` of
//! SIRIUS 6. The spectra come from the caller (`ms2spec`): one per row of `Ctx::rows`, `None` where the row has none. Spectra and areas only; the
//! programs of the destination do their own search.

use crate::export::{fix, Ctx};

/// The MS2 spectrum chosen for a row.
#[derive(Debug, Clone, PartialEq)]
pub struct Ms2 {
    /// m/z of the precursor as the scan declares it (centre of the isolation window).
    pub prec: f64,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    /// Files in which the row has an MS2.
    pub files: u32,
}

fn mode(polarity: i8) -> (&'static str, &'static str) {
    if polarity < 0 {
        ("1-", "Negative")
    } else {
        ("1+", "Positive")
    }
}

fn peaks(out: &mut Vec<String>, s: &Ms2) {
    for (m, y) in s.mz.iter().zip(&s.it) {
        out.push(format!("{} {}", fix(*m, 5), fix(*y, 1)));
    }
}

/// MGF with one block per row that has a spectrum; the comment lines (`#`) say what table it comes from.
pub fn mgf(c: &Ctx, spectra: &[Option<Ms2>]) -> String {
    let (charge, ionmode) = mode(c.polarity);
    let mut out = c.header();
    for (&r, s) in c.rows().iter().zip(spectra) {
        let Some(s) = s.as_ref().filter(|s| !s.mz.is_empty()) else { continue };
        let id = c.table.id(r);
        let prec = if s.prec > 0.0 { s.prec } else { c.table.mz[r] };
        out.extend(["BEGIN IONS".to_string(), format!("FEATURE_ID={id}"), format!("PEPMASS={prec:.4}"), format!("SCANS={id}")]);
        out.extend([format!("RTINSECONDS={:.2}", c.table.rt[r] * 60.0), format!("CHARGE={charge}"), "MSLEVEL=2".to_string(), format!("IONMODE={ionmode}")]);
        if s.files > 0 {
            out.push(format!("MERGED_ACROSS_N_SAMPLES={}", s.files));
        }
        peaks(&mut out, s);
        out.push("END IONS".into());
    }
    out.join("\n") + "\n"
}

/// SIRIUS 6 `.ms`: one compound per row with a spectrum, named after the row (`feature_<id>`), no formula (SIRIUS finds it).
pub fn sirius(c: &Ctx, spectra: &[Option<Ms2>]) -> String {
    let ion = if c.polarity < 0 { "[M-H]-" } else { "[M+H]+" };
    let mut out = vec!["# mzLab export for SIRIUS 6 (.ms). Spectra only; nothing here is an identification.".to_string()];
    out.extend(c.header());
    for (&r, s) in c.rows().iter().zip(spectra) {
        let Some(s) = s.as_ref().filter(|s| !s.mz.is_empty()) else { continue };
        let mass = if s.prec > 0.0 { s.prec } else { c.table.mz[r] };
        out.extend([String::new(), format!(">compound feature_{}", c.table.id(r)), format!(">parentmass {}", fix(mass, 5)), format!(">ionization {ion}")]);
        out.extend([format!(">rt {}s", fix(c.table.rt[r] * 60.0, 1)), ">ms2".to_string()]);
        peaks(&mut out, s);
    }
    out.join("\n") + "\n"
}
