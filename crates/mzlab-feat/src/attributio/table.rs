//! The table of levels of a campaign folder (`run`) and its three renderings: JSON for the page, CSV with `;` and the rows of a spreadsheet. The folder is read
//! through `Io` (closures, no disk); the renderings hand their text to an `emit` callback in pieces so that a caller can write it to a sink at offsets.

use super::annotate::{
    annotations_fingerprint, gnps_comparer, load_annotations, AnnotationSettings, Feature,
};
use super::campaign::Env;
use super::curation::TableCuration;
use super::error::{CampaignError, Result};
use super::io::Io;
use super::sources::load_sources;
use super::Campaign;
use mzlab_core::json::Out;
use std::collections::BTreeMap;

/// Columns of the table, in order.
pub const COLUMNS: [&str; 10] = [
    "pol", "id", "mz", "rt", "level", "sub", "basis", "name", "identity", "reasons",
];

/// One run: the features with their levels, the warnings, the fingerprint of the files and settings, and the count per level (`1`, `2a`, `3b`, `4`, `0`).
pub struct Run {
    pub features: Vec<Feature>,
    pub warnings: Vec<String>,
    pub fingerprint: String,
    pub settings: AnnotationSettings,
    pub counts: BTreeMap<String, usize>,
}

/// The levels of the campaign whose `campaign.toml` is at the root of `io` (`root` is the name of the folder).
pub fn run(io: &Io, root: &str) -> Result<Run> {
    let text = io
        .text("campaign.toml")
        .ok_or_else(|| CampaignError::not_found("campaign.toml is not in the folder"))?;
    let c = Campaign::from_toml(&text, "campaign.toml", root, Env::default())?;
    let src = load_sources(&c, io)?;
    let cur = TableCuration::load(&c, io)?;
    let a = load_annotations(&c, &src, None, &cur, &gnps_comparer)?;
    let settings = AnnotationSettings::for_campaign(&c)?;
    let sites: Vec<String> = c.site_keys().iter().map(|s| s.to_string()).collect();
    let fingerprint = annotations_fingerprint(&c, io, &sites, &settings);
    let mut counts = BTreeMap::new();
    for f in &a.features {
        *counts
            .entry(format!("{}{}", f.level, f.level_sub))
            .or_insert(0) += 1;
    }
    Ok(Run {
        features: a.features,
        warnings: a.warnings,
        fingerprint,
        settings,
        counts,
    })
}

impl Run {
    /// `{n_features, by_level, fingerprint, settings, warnings}`: the small answer that goes to the page, the notebook and the `.mzflow` (no data of the campaign).
    pub fn summary(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"n_features\":").int(self.features.len() as i64);
        o.raw(",\"by_level\":{");
        for (i, (k, v)) in self.counts.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .str(k)
                .raw(":")
                .int(*v as i64);
        }
        o.raw("},\"fingerprint\":").str(&self.fingerprint);
        let s = &self.settings;
        o.raw(",\"settings\":{");
        let nums = [
            ("spec_sim_min", s.spec_sim_min),
            ("spec_peaks_min", s.spec_peaks_min),
            ("cosmic_min", s.cosmic_min),
            ("canopus_min", s.canopus_min),
            ("library_conflict_margin", s.library_conflict_margin),
            ("rt_group_tol_min", s.rt_group_tol_min),
        ];
        for (i, (k, v)) in nums.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").num(*v);
        }
        o.raw("},\"warnings\":[");
        for (i, w) in self.warnings.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(w);
        }
        o.raw("]}");
        o.0
    }

    /// `{"columns":[...],"rows":[[...],...]}`, one row at a time.
    pub fn json_to(&self, emit: &mut dyn FnMut(&str)) {
        let mut o = Out::default();
        o.raw("{\"columns\":[");
        for (i, c) in COLUMNS.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(c);
        }
        o.raw("],\"rows\":[");
        emit(&std::mem::take(&mut o.0));
        for (k, f) in self.features.iter().enumerate() {
            o.raw(if k > 0 { ",[" } else { "[" });
            row_json(&mut o, f);
            emit(&std::mem::take(&mut o.0));
        }
        emit("]}");
    }

    /// The table as text with `;` between cells (a cell with `;`, a quote or a line break between quotes), UTF-8.
    pub fn csv_to(&self, emit: &mut dyn FnMut(&str)) {
        emit(&format!("{}\n", COLUMNS.join(";")));
        for f in &self.features {
            let cells = [
                f.pol.clone(),
                f.id.to_string(),
                num_text(f.mz),
                num_text(f.rt),
                f.level.to_string(),
                f.level_sub.clone(),
                f.level_basis.clone(),
                f.name().unwrap_or("").to_string(),
                f.identity().unwrap_or("").to_string(),
                f.level_why.clone(),
            ];
            let line: Vec<String> = cells.iter().map(|c| quote(c)).collect();
            emit(&format!("{}\n", line.join(";")));
        }
    }

    /// `{"hojas":[{"nombre":"Attributio","filas":[[header],[row]...]}]}`, the model of `mzlab_feat::export::xlsx_rows` (the page zips it into an XLSX).
    pub fn xlsx_rows_to(&self, emit: &mut dyn FnMut(&str)) {
        let mut o = Out::default();
        o.raw("{\"hojas\":[{\"nombre\":\"Attributio\",\"filas\":[[");
        for (i, c) in COLUMNS.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(c);
        }
        o.raw("]");
        emit(&std::mem::take(&mut o.0));
        for f in &self.features {
            o.raw(",[");
            row_json(&mut o, f);
            emit(&std::mem::take(&mut o.0));
        }
        emit("]}]}");
    }
}

/// The cells of one feature as a JSON array body, closing bracket included.
fn row_json(o: &mut Out, f: &Feature) {
    o.str(&f.pol)
        .raw(",")
        .int(f.id)
        .raw(",")
        .opt(f.mz)
        .raw(",")
        .opt(f.rt);
    o.raw(",").int(f.level as i64).raw(",").str(&f.level_sub);
    o.raw(",").str(&f.level_basis).raw(",");
    text_or_null(o, f.name());
    o.raw(",");
    text_or_null(o, f.identity());
    o.raw(",").str(&f.level_why).raw("]");
}

fn text_or_null(o: &mut Out, v: Option<&str>) {
    match v {
        Some(s) => o.str(s),
        None => o.raw("null"),
    };
}

fn num_text(v: Option<f64>) -> String {
    v.filter(|v| v.is_finite())
        .map_or(String::new(), |v| v.to_string())
}

fn quote(s: &str) -> String {
    if s.contains([';', '"', '\r', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
