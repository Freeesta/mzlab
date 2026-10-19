//! The exports of a campaign folder as the page takes them: `run(io, root, kind, opts)` answers one JSON `{"files":[...],"warnings":[...]}` where a file is
//! `{"name","text"}` (CSV, SVG, JSON) or `{"name","rows"}` (the rows of a spreadsheet, `{"hojas":[...]}`, that the page zips into an XLSX). The kinds are
//! `schymanski`, `publication`, `quantify`, `gradient` and `sirius`; the evidence export (`evidence`) needs the review and lives in `review::evidence`.

use super::cells::{xlsx_rows, Sheet};
use super::ctx::{matrix_and_sites, Loaded};
use super::publication::build as build_publication;
use super::publication_ids::{canonical_identities, feature_frame};
use super::publication_readme::{readme, Provenance, ReadmeInput};
use super::schymanski::build as build_schymanski;
use super::sirius::{collect, write_mgf, write_ms};
use crate::attributio::campaign::{Campaign, Env as CEnv};
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::frame::Cell;
use crate::attributio::gradient::{
    feature_cloud_gradient, identity_gradient, FeatureIdentity, GradIdentity, CLOUD_COLUMNS,
    GRADIENT_COLUMNS,
};
use crate::attributio::io::Io;
use crate::attributio::quantify::{
    load_calibration, quantify_lvl1, quantify_lvl1_environmental, ENV_COLUMNS, QUANT_COLUMNS,
};
use mzlab_core::json::Out;

/// What the page can set.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The matrix to export (the campaign's diagnostics matrix or its first when `None`).
    pub matrix: Option<String>,
    /// The date of the export, written in the provenance of the publication table.
    pub generated: String,
}

/// A file of an export.
pub enum ExportFile {
    Text(String, String),
    Rows(String, Vec<Sheet>),
}

/// The campaign of a folder.
pub fn load_campaign(io: &Io, root: &str) -> Result<Campaign> {
    let text = io
        .text("campaign.toml")
        .ok_or_else(|| CampaignError::not_found("campaign.toml is not in the folder"))?;
    Campaign::from_toml(&text, "campaign.toml", root, CEnv::default())
}

/// The JSON the page reads.
pub fn files_json(files: &[ExportFile], warnings: &[String]) -> String {
    let mut o = Out::default();
    o.raw("{\"files\":[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        match f {
            ExportFile::Text(n, t) => o.raw("{\"name\":").str(n).raw(",\"text\":").str(t),
            ExportFile::Rows(n, s) => o
                .raw("{\"name\":")
                .str(n)
                .raw(",\"rows\":")
                .raw(&xlsx_rows(s)),
        };
        o.raw("}");
    }
    o.raw("],\"warnings\":[");
    for (i, w) in warnings.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(w);
    }
    o.raw("]}");
    o.0
}

fn with_csv(stem: &str, sheets: Vec<Sheet>) -> Vec<ExportFile> {
    let csv = sheets.first().map(Sheet::csv).unwrap_or_default();
    vec![
        ExportFile::Rows(format!("{stem}.xlsx"), sheets),
        ExportFile::Text(format!("{stem}.csv"), csv),
    ]
}

fn publication(c: &Campaign, io: &Io, o: &Options) -> Result<Vec<ExportFile>> {
    let p = build_publication(c, io, o.matrix.as_deref())?;
    let prov = Provenance {
        script: "mzLab, Attributio export".into(),
        core: crate::attributio::export::VERSION.into(),
        generated: o.generated.clone(),
        ..Provenance::default()
    };
    let rd = readme(&ReadmeInput {
        campaign: c,
        counts: &p.counts,
        sites: &p.sites,
        short: &p.short,
        sorbents: &p.sorbents,
        title: &c.name,
        description: "Compounds identified in the campaign with the evidence behind each level of identification (Schymanski scale).",
        prov: &prov,
    })?;
    Ok(with_csv(
        &format!("publication_{}", c.id),
        vec![p.identities, rd, p.excluded],
    ))
}

fn quantify(
    c: &Campaign,
    io: &Io,
    o: &Options,
    warnings: &mut Vec<String>,
) -> Result<Vec<ExportFile>> {
    let l = Loaded::load(c, io, o.matrix.as_deref())?;
    let cal = load_calibration(c, io, warnings)?;
    let q = quantify_lvl1(c, &l.src, &cal, l.features(), None)?;
    let (e, warning) = quantify_lvl1_environmental(c, &l.src, &cal, l.features(), None)?;
    warnings.extend(warning);
    let mut s1 = Sheet::new("Level 1", &QUANT_COLUMNS);
    s1.rows = q.iter().map(|r| r.cells(false)).collect();
    let head: Vec<&str> = QUANT_COLUMNS
        .iter()
        .chain(ENV_COLUMNS.iter())
        .copied()
        .collect();
    let mut s2 = Sheet::new("Environmental", &head);
    s2.rows = e.iter().map(|r| r.cells(true)).collect();
    Ok(with_csv(&format!("quantification_{}", c.id), vec![s1, s2]))
}

fn gradient(c: &Campaign, io: &Io, o: &Options) -> Result<Vec<ExportFile>> {
    let l = Loaded::load(c, io, o.matrix.as_deref())?;
    let (_, sites) = matrix_and_sites(c, o.matrix.as_deref())?;
    let [us, ds] = match sites.as_slice() {
        [a, b, ..] => [a.as_str(), b.as_str()],
        _ => {
            return Err(CampaignError::campaign(
                "the gradient needs two sites in the matrix",
            ))
        }
    };
    let (gt, dd) = canonical_identities(&l);
    let rows = super::ctx::feature_rows(c, io, &l)?;
    let ids: Vec<GradIdentity> = dd
        .iter()
        .map(|d| GradIdentity {
            name: d.nome.clone(),
            present: Some((d.in_site[0], d.in_site[1])),
        })
        .collect();
    let fi: Vec<FeatureIdentity> = feature_frame(c, io, &l, &rows, &gt)?
        .iter()
        .map(|p| FeatureIdentity {
            pol: p.f().pol.clone(),
            id: p.f().id,
            rt: p.f().rt,
            name: p.nome.clone(),
        })
        .collect();
    let (table, s) = identity_gradient(c, &l.src, &ids, &fi, [us, ds], 3.0, None)?;
    let mut t = Sheet::new("Identity gradient", &GRADIENT_COLUMNS);
    t.rows = table.iter().map(|r| r.cells()).collect();
    let mut cloud = Sheet::new("Feature cloud", &CLOUD_COLUMNS);
    cloud.rows = feature_cloud_gradient(c, &l.src, l.features(), [us, ds])?;
    let mut sum = Sheet::new("Summary", &["item", "value"]);
    let n = |v: usize| Cell::Int(v as i64);
    for (k, v) in [
        ("fold threshold", Cell::Num(s.fold_threshold)),
        ("upstream site", Cell::Str(s.site_us.clone())),
        ("downstream site", Cell::Str(s.site_ds.clone())),
        ("in both", n(s.n_both)),
        ("enriched upstream", n(s.n_us_enriched)),
        ("enriched downstream", n(s.n_ds_enriched)),
        ("within the fold", n(s.n_within_fold)),
        ("upstream only", n(s.n_us_only)),
        ("downstream only", n(s.n_ds_only)),
        ("not comparable", n(s.n_not_comparable)),
        ("discordant sorbent", n(s.n_sorbent_discordant)),
    ] {
        sum.rows.push(vec![Cell::Str(k.into()), v]);
    }
    Ok(with_csv(&format!("gradient_{}", c.id), vec![t, cloud, sum]))
}

fn sirius(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Vec<ExportFile>> {
    let features = collect(c, io, warnings)?;
    Ok(vec![
        ExportFile::Text(format!("sirius_{}.ms", c.id), write_ms(&features)),
        ExportFile::Text(format!("sirius_{}.mgf", c.id), write_mgf(&features)),
    ])
}

/// The export `kind` of the folder as the JSON the page reads.
pub fn run(io: &Io, root: &str, kind: &str, o: &Options) -> Result<String> {
    let c = load_campaign(io, root)?;
    let mut warnings = Vec::new();
    let files = match kind {
        "schymanski" => {
            let sheets = build_schymanski(&c, io, o.matrix.as_deref())?;
            with_csv(&format!("schymanski_{}", c.id), sheets)
        }
        "publication" => publication(&c, io, o)?,
        "quantify" => quantify(&c, io, o, &mut warnings)?,
        "gradient" => gradient(&c, io, o)?,
        "sirius" => sirius(&c, io, &mut warnings)?,
        _ => return Err(CampaignError::value(format!("unknown export: {kind}"))),
    };
    Ok(files_json(&files, &warnings))
}
