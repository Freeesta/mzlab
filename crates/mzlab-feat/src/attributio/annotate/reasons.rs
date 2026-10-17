// Ported from Attributio annotate.py.
//! `level_reasons`: two texts that EXPLAIN the level of a feature (no level changes here). `level_why` has one line per gate of the rule with the value, the gate
//! and `pass` / `FAIL`; `level_decisive` is the decisive factor in one sentence. The review page shows them ("Why not 2a?").

use super::settings::AnnotationSettings;
use super::text::{fmt_g, num_txt};
use super::types::Feature;
use crate::sortc::SortC;

fn gate(ok: bool) -> &'static str {
    if ok {
        "pass"
    } else {
        "FAIL"
    }
}

/// A library source as the lines see it.
struct Src<'a> {
    label: &'static str,
    name: Option<&'a str>,
    cos: Option<f64>,
    n: Option<f64>,
    valid: bool,
}

fn sources(f: &Feature) -> [Src<'_>; 3] {
    [
        Src {
            label: "SIRIUS spectral",
            name: f.spec.name.as_deref(),
            cos: f.spec.sim,
            n: f.spec.peaks,
            valid: f.gates.sirius_spectral,
        },
        Src {
            label: "GNPS2",
            name: f.gnps.name.as_deref(),
            cos: f.gnps.score,
            n: f.gnps.peaks,
            valid: f.gates.gnps2,
        },
        Src {
            label: "mzMine library",
            name: f.mzlib.name.as_deref(),
            cos: f.mzlib.score,
            n: f.mzlib.peaks,
            valid: f.gates.mzmine_library,
        },
    ]
}

/// `(precursor text, precursor ok)` of GNPS2 and mzMine; SIRIUS has no precursor test.
fn precursor(f: &Feature, label: &str, s: &AnnotationSettings) -> Option<(String, bool)> {
    match label {
        "GNPS2" => {
            let ok = f.gnps.exact;
            let md = f.gnps.massdiff;
            let ppm = match (md, f.mz) {
                (Some(m), Some(mz)) if mz != 0.0 => Some(m / mz * 1e6),
                _ => None,
            };
            Some((
                format!(
                    "precursor difference {} Da = {} ppm vs {:.0} ppm ({}) {}",
                    num_txt(md, 4),
                    num_txt(ppm, 1),
                    s.gnps_mass_tol_ppm,
                    if ok { "exact" } else { "analog" },
                    gate(ok)
                ),
                ok,
            ))
        }
        "mzMine library" => {
            let ok = f.mzlib.exact;
            Some((
                format!(
                    "precursor {} ppm vs {:.0} ppm {}",
                    num_txt(f.mzlib.massdiff_ppm, 1),
                    s.mzlib_mass_tol_ppm,
                    gate(ok)
                ),
                ok,
            ))
        }
        _ => None,
    }
}

fn at_least(v: Option<f64>, min: f64) -> bool {
    v.is_some_and(|x| x >= min)
}

/// The line of one source and, when it is not a valid source, the closest-miss text with the number of gates missed.
fn source_line(f: &Feature, x: &Src, s: &AnnotationSettings) -> (String, Option<(usize, String)>) {
    let Some(name) = x.name else {
        return (format!("{}: no hit", x.label), None);
    };
    let ok_cos = at_least(x.cos, s.spec_sim_min);
    let ok_n = at_least(x.n, s.spec_peaks_min);
    let mut parts = vec![
        format!(
            "cosine {} vs {:.2} {}",
            num_txt(x.cos, 2),
            s.spec_sim_min,
            gate(ok_cos)
        ),
        format!(
            "shared ions {} vs {} {}",
            num_txt(x.n, 0),
            fmt_int(s.spec_peaks_min),
            gate(ok_n)
        ),
    ];
    let prec = precursor(f, x.label, s);
    let ok_p = prec.as_ref().is_none_or(|p| p.1);
    if let Some((t, _)) = prec {
        parts.push(t);
    }
    let line = format!(
        "{}: {} -- {} => {}",
        x.label,
        name,
        parts.join("; "),
        if x.valid {
            "valid source"
        } else {
            "not a source"
        }
    );
    if x.valid {
        return (line, None);
    }
    let mut miss = Vec::new();
    if !ok_cos {
        miss.push(format!(
            "cosine {} < {:.2}",
            num_txt(x.cos, 2),
            s.spec_sim_min
        ));
    }
    if !ok_n {
        miss.push(format!(
            "{} shared ions < {}",
            num_txt(x.n, 0),
            fmt_int(s.spec_peaks_min)
        ));
    }
    if !ok_p {
        miss.push("analog, not the same precursor".into());
    }
    let text = format!("{} ({}): {}", x.label, name, miss.join(", "));
    (line, Some((miss.len(), text)))
}

fn fmt_int(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

fn lines_of(f: &Feature, s: &AnnotationSettings) -> (String, Vec<(usize, String)>) {
    let mut lines = Vec::new();
    let mut failed = Vec::new();
    for x in sources(f) {
        let (line, miss) = source_line(f, &x, s);
        lines.push(line);
        failed.extend(miss);
    }
    match f.csi.name.as_deref() {
        None => lines.push("CSI:FingerID: no structure".into()),
        Some(n) => lines.push(format!(
            "CSI:FingerID: {} -- COSMIC {} vs {:.2} {}",
            n,
            num_txt(f.csi.cosmic, 2),
            s.cosmic_min,
            gate(f.gates.csi)
        )),
    }
    if f.name_conflict {
        lines.push(format!(
            "name conflict: valid sources name different structures (InChIKey first blocks {})",
            f.name_blocks.replace('|', ", ")
        ));
    }
    if let Some(c) = f.name_choice.as_deref().filter(|c| !c.is_empty()) {
        lines.push(format!("name: {c}"));
    }
    if !f.decision.feature_decision.is_empty() {
        lines.push(format!(
            "chemist's decision: {}",
            f.decision.feature_decision
        ));
    }
    if let Some(ng) = f.rt_info.rt_groups.filter(|n| *n > 1) {
        lines.push(format!(
            "retention time: the identity is carried by features at {} retention times (group {}, 0 = reference)",
            ng,
            f.rt_info.rt_group.unwrap_or(0)
        ));
    }
    (lines.join("\n"), failed)
}

fn decisive(f: &Feature, s: &AnnotationSettings, mut failed: Vec<(usize, String)>) -> String {
    let srcs = sources(f);
    let valid: Vec<&str> = srcs.iter().filter(|x| x.valid).map(|x| x.label).collect();
    match f.level_basis.as_str() {
        _ if f.level == 1 => "Level 1: reference standard, mass + RT + MS/MS.".into(),
        "diagnostic evidence" => "The chemist's decision fixes the level: Level 2b on diagnostic evidence.".into(),
        "expert review" => format!(
            "{}{}, but the chemist's decision takes it to Level 3.",
            if valid.is_empty() { "A chemist decision" } else { "All gates passed" },
            if valid.is_empty() { String::new() } else { format!(" ({})", valid.join(", ")) }
        ),
        "name conflict" => {
            let tail = match f.name_choice.as_deref().filter(|c| !c.is_empty()) {
                Some(c) => c.to_string(),
                None => srcs
                    .iter()
                    .filter(|x| x.valid)
                    .map(|x| format!("{}: {}", x.label, x.name.unwrap_or("nan")))
                    .collect::<Vec<_>>()
                    .join(", "),
            };
            format!(
                "Valid libraries name different structures and the best match is ahead by less than {} cosine: the spectrum does not tell them apart (Level 3). {tail}.",
                fmt_g(s.library_conflict_margin)
            )
        }
        "isomers not discriminated" => "The same compound is matched at two or more retention times: the spectrum does not tell the isomers apart.".into(),
        "RT isomer" => "A reference standard of this compound elutes at another retention time: another isomer.".into(),
        "putative library" => "The library match is putative (e.g. an analogue search or in-silico spectrum), not a confirmed reference standard (L3a).".into(),
        "spectral library" => format!(
            "Level 2a: {} {} every gate.",
            valid.join(", "),
            if valid.len() == 1 { "passes" } else { "pass" }
        ),
        "CSI:FingerID" => {
            let mut d = format!(
                "No library source passes its gates; the level rests on the CSI:FingerID structure (COSMIC {} ≥ {:.2}).",
                num_txt(f.csi.cosmic, 2),
                s.cosmic_min
            );
            if !failed.is_empty() {
                failed.sortc_by_key(|t| t.0);
                d.push_str(&format!(" Closest library hit: {}.", failed[0].1));
            }
            d
        }
        "suspect-RT" => "Reference standard matched on mass + RT only: its MS/MS did not confirm it (suspect-RT).".into(),
        _ if failed.is_empty() => "Nothing above the gates.".into(),
        _ => format!("Nothing above the gates; {}.", failed[0].1),
    }
}

/// Fills `level_why` and `level_decisive` of every feature.
pub fn level_reasons(feats: &mut [Feature], s: &AnnotationSettings) {
    for f in feats.iter_mut() {
        let (why, failed) = lines_of(f, s);
        f.level_decisive = decisive(f, s, failed);
        f.level_why = why;
    }
}
