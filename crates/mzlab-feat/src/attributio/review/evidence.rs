// Ported from Attributio src/attributio/review/evidence.py.
//! The evidence export of the review: for every feature the chemist decided 2a or 2b (analogs included) a mirror plot (SVG) with the CSV of its data and one row
//! of the evidence table, with the thresholds hash of the decisions, ready for the supplementary file of a manuscript. The features are those whose last level
//! decision in `feature_decisions.csv` says `2b` or `2` (2a kept). The evidence is the reference recorded with the decision (`evidence_entry`; for an analog
//! the delta m recorded, the reference drawn shifted by it and the pairs matched through it in violet); a 2a kept without a recorded reference shows the best
//! exact reference of the same structure; a 2b on fragments has no reference and the plot shows the spectrum and its explained fragments.

use super::banco::Run;
use super::evidence_svg::{reference_rel, rows, svg, Plot, RefDraw, PLOT_COLUMNS};
use super::matching::{compare, Kind, Ms2};
use super::proposal::{Proposal, Reference, View};
use super::settings::ReviewSettings;
use crate::attributio::annotate::{AnnotationSettings, Feature};
use crate::attributio::error::Result;
use crate::attributio::export::pyfmt::py_float;
use crate::attributio::export::sha1::sha1_hex;
use crate::attributio::frame::to_csv;
use crate::attributio::io::Io;
use crate::attributio::readers::feature_decisions::{same_feature, DecisionTable};

pub use crate::attributio::export::VERSION as APP_VERSION;

/// The columns of the evidence table.
pub const TABLE_COLUMNS: [&str; 26] = [
    "pol",
    "feature_id",
    "mz",
    "rt",
    "level",
    "identity",
    "evidence_type",
    "reference_name",
    "reference_library",
    "reference_entry",
    "usi",
    "kind",
    "cosine",
    "forward",
    "reverse",
    "presence",
    "matched",
    "delta_mz",
    "delta_formula",
    "corroboration",
    "confirmation",
    "note",
    "decided_on",
    "app_version",
    "settings_hash",
    "plot",
];

/// `(hash, "key=value;...")` of the thresholds a decision was taken with.
pub fn settings_digest(s: &ReviewSettings, a: &AnnotationSettings) -> (String, String) {
    let kv = [
        ("tol_da", py_float(s.tol_da)),
        ("ppm_frag", py_float(s.ppm_frag)),
        ("exact_ppm", py_float(s.exact_ppm)),
        ("analog_window_da", py_float(s.analog_window_da)),
        ("spec_sim_min", py_float(a.spec_sim_min)),
        ("spec_peaks_min", format!("{}", a.spec_peaks_min)),
        ("l2a_min_cos_vis", py_float(s.l2a_min_cos_vis)),
        ("l2a_min_matched_vis", s.l2a_min_matched_vis.to_string()),
        ("delta_formula_mda", py_float(s.delta_formula_mda)),
    ]
    .iter()
    .map(|(k, v)| format!("{k}={v}"))
    .collect::<Vec<_>>()
    .join(";");
    (sha1_hex(&kv)[..12].to_string(), kv)
}

/// A decided feature: its index among the features, the decision row with the level and the note of the last row.
#[derive(Debug, PartialEq)]
pub struct Decided {
    pub feature: usize,
    pub row: usize,
    pub note: String,
}

/// The features whose last level decision is `2b` or `2`.
pub fn decided_features(d: &DecisionTable, features: &[Feature]) -> Vec<Decided> {
    let mut out = Vec::new();
    for (k, f) in features.iter().enumerate() {
        let key = DecisionTable {
            columns: ["pol", "feature_id", "mz", "rt"].map(String::from).to_vec(),
            rows: vec![vec![
                f.pol.clone(),
                f.id.to_string(),
                f.mz.map_or(String::new(), |v| v.to_string()),
                f.rt.map_or(String::new(), |v| v.to_string()),
            ]],
        };
        let mine: Vec<usize> = (0..d.len())
            .filter(|&r| same_feature(d, r, &key, 0))
            .collect();
        let with_level: Vec<usize> = mine
            .iter()
            .copied()
            .filter(|&r| !d.get(r, "level").trim().is_empty())
            .collect();
        let Some(&last) = with_level.last() else {
            continue;
        };
        if !matches!(d.get(last, "level"), "2b" | "2") {
            continue;
        }
        let note = match mine.last().map(|&r| d.get(r, "justification")) {
            Some(m) if !m.is_empty() => m.to_string(),
            _ => d.get(last, "justification").to_string(),
        };
        out.push(Decided {
            feature: k,
            row: last,
            note,
        });
    }
    out
}

/// The USI of a public library spectrum id.
pub fn usi_for(id: &str) -> Option<String> {
    let mut id = id.trim().to_string();
    if id.starts_with('[') {
        id = id
            .find("mzspec:")
            .map(|i| {
                id[i..]
                    .chars()
                    .take_while(|c| !matches!(c, '"' | '\'' | ',' | ']') && !c.is_whitespace())
                    .collect()
            })
            .unwrap_or_default();
    }
    if id.starts_with("CCMSLIB") {
        Some(format!("mzspec:GNPS:GNPS-LIBRARY:accession:{id}"))
    } else if id.starts_with("MSBNK-") {
        Some(format!("mzspec:MassBank::accession:{id}"))
    } else if id.starts_with("mzspec:") {
        Some(id)
    } else {
        None
    }
}

/// The reference the decision rests on, as its position in `references`.
fn find_candidate(refs: &[Reference], d: &DecisionTable, r: usize) -> Option<usize> {
    let entry = d.get(r, "evidence_entry").trim();
    let have = |k: &usize| refs[*k].available;
    if !entry.is_empty() {
        let hit: Vec<usize> = (0..refs.len())
            .filter(|&k| refs[k].entry.as_deref() == Some(entry))
            .collect();
        if d.get(r, "evidence_type") == "analog" {
            let an: Vec<usize> = hit
                .iter()
                .copied()
                .filter(|&k| refs[k].kind == "analog")
                .collect();
            if !an.is_empty() {
                return an.first().copied();
            }
        }
        return hit.first().copied();
    }
    if d.get(r, "level") == "2" {
        let exact = |k: &usize| refs[*k].kind == "exact" && have(k);
        let same = (0..refs.len()).find(|k| exact(k) && refs[*k].same);
        return same.or_else(|| (0..refs.len()).find(exact));
    }
    None
}

/// `re.sub(r"[^A-Za-z0-9._-]+", "_", s)[:60].strip("_")`.
fn safe(s: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('_');
            gap = true;
        }
    }
    out.chars()
        .take(60)
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn caption(r: Option<&Reference>, dm: f64, d: &DecisionTable, row: usize) -> Vec<String> {
    let Some(r) = r else {
        return vec!["2b on fragments recognised on the structure (no reference spectrum)".into()];
    };
    let opt = |v: Option<f64>| v.map_or("None".to_string(), py_float);
    let mut score = if r.kind == "exact" {
        format!("cosine {}", opt(r.cos_vis))
    } else {
        "cosine (modified)".to_string()
    };
    if let Some(q) = &r.scores {
        score = format!(
            "forward {:.2} · reverse {:.2} · presence {:.2} · matched {}",
            q.fwd, q.rev, q.presence, q.matched
        );
    }
    let mut out = vec![
        format!(
            "reference: {} ({} {})",
            r.name,
            r.library.as_deref().unwrap_or("None"),
            r.entry.as_deref().unwrap_or("")
        ),
        score,
    ];
    if r.kind == "analog" {
        let formula = d.get(row, "evidence_delta_formula");
        out.push(format!(
            "ANALOG: reference shifted by delta m = {dm:+.4} Da ({}); violet = matched through delta m",
            if formula.is_empty() { "?" } else { formula }
        ));
    } else {
        out.push(format!("EXACT: same precursor ion ({} ppm)", opt(r.dm_ppm)));
    }
    out
}

/// The plot of a decision: the experimental peaks of the view, the reference drawn against them.
fn draw(
    f: &Feature,
    v: &View,
    (prec, tol): (f64, f64),
    cand: Option<usize>,
    p: &Proposal,
    d: &DecisionTable,
    row: usize,
) -> (String, Vec<Vec<String>>) {
    let refs = &p.evidence.references;
    let r = cand.map(|k| &refs[k]);
    let dm = match r {
        Some(r) if r.kind == "analog" => d
            .get(row, "evidence_delta_mz")
            .trim()
            .parse::<f64>()
            .ok()
            .or(r.dm)
            .unwrap_or(0.0),
        _ => 0.0,
    };
    let spec: Option<&Ms2> = cand.and_then(|k| v.spectra.get(k)).and_then(|s| s.as_ref());
    let reference = spec.map(|s| {
        let exp = Ms2 {
            mz: v.peaks.iter().map(|k| k.mz).collect(),
            it: v.peaks.iter().map(|k| k.rel).collect(),
            prec,
        };
        let pairs = compare(&exp, s, tol, None, dm)
            .pairs
            .iter()
            .map(|&(i, j, k)| (i, j, k == Kind::Shifted))
            .collect();
        RefDraw {
            rel: reference_rel(&s.mz, &s.it, s.prec),
            mz: s.mz.clone(),
            shift: dm,
            pairs,
        }
    });
    let level = if d.get(row, "level") == "2b" {
        "Level 2b"
    } else {
        "Level 2a"
    };
    let plot = Plot {
        title: format!(
            "{} {} · m/z {:.4} · RT {:.2} min · {} · {level}",
            f.pol,
            f.id,
            f.mz.unwrap_or(f64::NAN),
            f.rt.unwrap_or(f64::NAN),
            f.identity().unwrap_or("")
        ),
        peaks: &v.peaks,
        reference,
        caption: caption(r, dm, d, row),
    };
    (svg(&plot), rows(&plot))
}

fn pick(a: String, b: String) -> String {
    if a.is_empty() {
        b
    } else {
        a
    }
}

/// One row of the evidence table.
fn table_row(
    f: &Feature,
    p: &Proposal,
    cand: Option<usize>,
    d: &DecisionTable,
    dec: &Decided,
    hash: &str,
    plot: &str,
) -> Vec<String> {
    let r = cand.map(|k| &p.evidence.references[k]);
    let row = dec.row;
    let num = |v: Option<f64>| v.map_or(String::new(), py_float);
    let q = r.and_then(|r| r.scores.as_ref());
    let corro = f
        .gnps_component
        .filter(|c| *c >= 0)
        .map(|c| format!("GNPS component {c}"))
        .unwrap_or_default();
    let etype = d.get(row, "evidence_type");
    vec![
        f.pol.clone(),
        f.id.to_string(),
        format!("{:.5}", f.mz.unwrap_or(f64::NAN)),
        format!("{:.3}", f.rt.unwrap_or(f64::NAN)),
        if d.get(row, "level") == "2b" {
            "2b"
        } else {
            "2a"
        }
        .into(),
        f.identity().unwrap_or("").into(),
        if etype.is_empty() {
            if r.is_some() {
                "library"
            } else {
                "fragments"
            }
        } else {
            etype
        }
        .into(),
        r.map_or(String::new(), |r| r.name.clone()),
        r.and_then(|r| r.library.clone()).unwrap_or_default(),
        r.and_then(|r| r.entry.clone()).unwrap_or_default(),
        r.and_then(|r| r.entry.as_deref())
            .and_then(usi_for)
            .unwrap_or_default(),
        r.map_or(String::new(), |r| r.kind.clone()),
        pick(
            d.get(row, "evidence_cosine").into(),
            num(r.and_then(|r| r.cos_vis)),
        ),
        q.map_or(String::new(), |q| py_float(q.fwd)),
        q.map_or(String::new(), |q| py_float(q.rev)),
        q.map_or(String::new(), |q| py_float(q.presence)),
        pick(
            d.get(row, "evidence_matched").into(),
            r.map_or(String::new(), |r| r.matched_vis.to_string()),
        ),
        d.get(row, "evidence_delta_mz").into(),
        d.get(row, "evidence_delta_formula").into(),
        corro,
        d.get(row, "evidence_note").into(),
        dec.note.clone(),
        d.get(row, "decided_on").into(),
        APP_VERSION.into(),
        hash.into(),
        plot.into(),
    ]
}

fn summary_json(hash: &str, kv: &str, n: usize, missing: &[String]) -> String {
    let list = if missing.is_empty() {
        "[]".to_string()
    } else {
        let items: Vec<String> = missing.iter().map(|m| format!("  \"{m}\"")).collect();
        format!("[\n{}\n ]", items.join(",\n"))
    };
    format!(
        "{{\n \"app_version\": \"{APP_VERSION}\",\n \"settings_hash\": \"{hash}\",\n \"settings\": \"{kv}\",\n \"features\": {n},\n \"missing_reference\": {list}\n}}"
    )
}

/// The files of the export as `(name, text)`: for every decided feature `<campaign>_<pol><id>_<identity>.svg` with `.csv`, then `evidence_<campaign>.csv`
/// and `evidence_<campaign>.json`.
pub fn export_evidence(io: &Io, root: &str) -> Result<Vec<(String, String)>> {
    let run = Run::load(io, root)?;
    let b = &run.b;
    let ann = AnnotationSettings::for_campaign(&b.campaign)?;
    let (hash, kv) = settings_digest(&b.settings, &ann);
    let d = b.decisions(io)?;
    let (mut files, mut table, mut missing) = (Vec::new(), Vec::new(), Vec::new());
    for dec in decided_features(&d, &b.features) {
        let f = &b.features[dec.feature];
        let (p, mgf) = run.proposal(io, f);
        let Some(v) = p.evidence.view.as_ref().filter(|v| !v.peaks.is_empty()) else {
            missing.push(format!("{} {}", f.pol, f.id));
            continue;
        };
        let cand = find_candidate(&p.evidence.references, &d, dec.row);
        let etype = d.get(dec.row, "evidence_type");
        if cand.is_none() && !matches!(etype, "fragments" | "") {
            missing.push(format!("{} {}", f.pol, f.id));
        }
        let prec = mgf.as_ref().map_or(f.mz.unwrap_or(0.0), |m| m.prec);
        let (svg_text, plot_rows) = draw(f, v, (prec, b.settings.tol_da), cand, &p, &d, dec.row);
        let base = format!(
            "{}_{}{}_{}",
            b.campaign.id,
            f.pol,
            f.id,
            safe(f.identity().unwrap_or(""))
        );
        let cols: Vec<String> = PLOT_COLUMNS.iter().map(|c| c.to_string()).collect();
        files.push((format!("{base}.svg"), svg_text));
        files.push((format!("{base}.csv"), to_csv(&cols, &plot_rows, ';')));
        table.push(table_row(
            f,
            &p,
            cand,
            &d,
            &dec,
            &hash,
            &format!("{base}.svg"),
        ));
    }
    let cols: Vec<String> = TABLE_COLUMNS.iter().map(|c| c.to_string()).collect();
    files.push((
        format!("evidence_{}.csv", b.campaign.id),
        to_csv(&cols, &table, ';'),
    ));
    files.push((
        format!("evidence_{}.json", b.campaign.id),
        summary_json(&hash, &kv, table.len(), &missing),
    ));
    Ok(files)
}

/// `export_evidence` as the JSON the page reads (the files of `export::run`).
pub fn evidence_json(io: &Io, root: &str) -> Result<String> {
    use crate::attributio::export::run::{files_json, ExportFile};
    let files: Vec<ExportFile> = export_evidence(io, root)?
        .into_iter()
        .map(|(n, t)| ExportFile::Text(n, t))
        .collect();
    Ok(files_json(&files, &[]))
}
