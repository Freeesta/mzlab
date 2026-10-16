// Ported from Attributio quantify.py.
//! Concentrations of the Level 1 compounds from the external calibration (`[standards] calibration` and `[quantification] spe_enrichment_factor`).
//! `quantify_lvl1` inverts the calibration line of the compound (the row marked `recommended`) on the area of each site where the feature is DETECTED:
//! the concentration in the analytical extract, ng/mL. `quantify_lvl1_environmental` brings it back to the water sample (ng/L) dividing by the
//! enrichment factor of the SPE: always a lower bound, without correction for recovery, and `< LOQ` where the area is under the lowest calibration point.

use super::annotate::Feature;
use super::campaign::Campaign;
use super::detection::{has_site, site_area, Agg};
use super::error::Result;
use super::frame::{Cell, Frame};
use super::io::Io;
use super::readers::read_table;
use super::sources::Sources;

/// Columns of the table, in order; the environmental table adds `ENV_COLUMNS`.
pub const QUANT_COLUMNS: [&str; 12] = [
    "_pol",
    "_id",
    "std_name",
    "site",
    "adduct",
    "area",
    "conc_ng_mL",
    "r2",
    "r2_ok",
    "below_loq",
    "area_loq",
    "loq_ratio",
];
pub const ENV_COLUMNS: [&str; 4] = ["conc_env_ng_L", "ef_spe", "recovery_corrected", "env_note"];

const NOTE_BELOW: &str =
    "< LOQ (area below the lowest calibration point): extrapolation, report as '<LOQ'";
const NOTE_ESTIMATE: &str =
    "environmental estimate without SPE recovery correction (true conc probably higher)";

/// One Level 1 feature at one site where it is DETECTED.
#[derive(Clone, Debug, PartialEq)]
pub struct QuantRow {
    pub pol: String,
    pub id: i64,
    pub std_name: String,
    pub site: String,
    pub adduct: String,
    pub area: f64,
    pub conc_ng_ml: f64,
    pub r2: f64,
    pub r2_ok: bool,
    pub below_loq: Option<bool>,
    pub area_loq: Option<f64>,
    pub loq_ratio: Option<f64>,
    /// Only in the environmental table.
    pub env: Option<EnvPart>,
}

/// What the environmental table adds to a row.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvPart {
    pub conc_env_ng_l: f64,
    pub ef_spe: f64,
    pub note: &'static str,
}

/// The calibration curves, one row per compound, adduct and polarity (an empty frame when the campaign has none or the file is not there).
pub fn load_calibration(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Frame> {
    let Some(path) = &c.calibration_csv else {
        return Ok(Frame::default());
    };
    match read_table(io, path, ';')? {
        Some(f) => Ok(f),
        None => {
            warnings.push(format!(
                "[{}] calibration curves not found: {path} -> no quantification in this run.",
                c.id
            ));
            Ok(Frame::default())
        }
    }
}

fn is_recommended(cal: &Frame, r: usize, k: Option<usize>) -> bool {
    k.is_some_and(|k| matches!(cal.cell(r, k).trim().to_lowercase().as_str(), "true" | "1"))
}

/// The first recommended row of the compound for the polarity.
fn curve_row(cal: &Frame, pol: &str, name: &str) -> Option<usize> {
    let (kp, kn) = (cal.col("polarity")?, cal.col("compound_name")?);
    let kr = cal.col("recommended");
    (0..cal.len()).find(|&r| {
        is_recommended(cal, r, kr) && cal.cell(r, kp) == pol && cal.cell(r, kn) == name
    })
}

/// `quantify_lvl1`: one row per Level 1 feature and per site (`sites`, the sites of the campaign when `None`) where it is DETECTED with a positive area.
pub fn quantify_lvl1(
    c: &Campaign,
    src: &Sources,
    cal: &Frame,
    feats: &[Feature],
    sites: Option<&[String]>,
) -> Result<Vec<QuantRow>> {
    let own: Vec<String>;
    let sites = match sites {
        Some(s) => s,
        None => {
            own = c.site_keys().iter().map(|s| s.to_string()).collect();
            &own
        }
    };
    let r2_min = c.quantification.get("r2_min").and_then(|v| v.as_f64()).unwrap_or(0.90);
    let loq_col = c
        .quantification
        .get("loq_area_column")
        .and_then(|v| v.as_str())
        .unwrap_or("area_200ppb")
        .to_string();
    let mut rows = Vec::new();
    for f in feats.iter().filter(|f| f.level == 1) {
        let Some(name) = f.std.as_ref().and_then(|s| s.name.clone()) else {
            continue;
        };
        let Some(cr) = curve_row(cal, &f.pol, &name) else {
            continue;
        };
        let Some(table) = src.pol(&f.pol).map(|p| &p.table) else {
            continue;
        };
        let Some(kid) = table.col("id") else { continue };
        let Some(tr) = (0..table.len()).find(|&r| table.int(r, kid) == Some(f.id)) else {
            continue;
        };
        let num = |col: &str| cal.col(col).and_then(|k| cal.num(cr, k));
        let (Some(slope), Some(intercept)) = (num("slope"), num("intercept")) else {
            continue;
        };
        let r2 = num("r2").unwrap_or(f64::NAN);
        let adduct = cal.col("adduct").map_or("", |k| cal.cell(cr, k)).to_string();
        let area_loq = num(&loq_col);
        for s in sites {
            if !has_site(c, table, s, &f.pol) {
                continue;
            }
            let area = site_area(c, table, s, &f.pol, None, Agg::Sum)?[tr];
            if area <= 0.0 {
                continue;
            }
            rows.push(QuantRow {
                pol: f.pol.clone(),
                id: f.id,
                std_name: name.clone(),
                site: s.clone(),
                adduct: adduct.clone(),
                area,
                conc_ng_ml: (area - intercept) / slope,
                r2,
                r2_ok: r2 >= r2_min,
                below_loq: area_loq.map(|l| area < l),
                area_loq,
                loq_ratio: area_loq.filter(|l| *l > 0.0).map(|l| area / l),
                env: None,
            });
        }
    }
    Ok(rows)
}

/// `quantify_lvl1_environmental`: `quantify_lvl1` plus `conc_env_ng_L = conc / EF * 1000` when the campaign declares the enrichment factor, with the warning the
/// reference tool raises each time (returned, not printed).
pub fn quantify_lvl1_environmental(
    c: &Campaign,
    src: &Sources,
    cal: &Frame,
    feats: &[Feature],
    sites: Option<&[String]>,
) -> Result<(Vec<QuantRow>, Option<String>)> {
    let mut rows = quantify_lvl1(c, src, cal, feats, sites)?;
    let ef = c
        .quantification
        .get("spe_enrichment_factor")
        .and_then(|v| v.as_f64())
        .filter(|e| *e != 0.0);
    let Some(ef) = ef.filter(|_| !rows.is_empty()) else {
        return Ok((rows, None));
    };
    for r in rows.iter_mut() {
        r.env = Some(EnvPart {
            conc_env_ng_l: r.conc_ng_ml / ef * 1000.0,
            ef_spe: ef,
            note: if r.below_loq == Some(true) { NOTE_BELOW } else { NOTE_ESTIMATE },
        });
    }
    let warning = format!(
        "quantify_lvl1_environmental(): conc_env_ng_L divides by EF={} only, WITHOUT recovery correction (RE/ME/PE not validated) -> the true concentration is \
         likely HIGHER. Where below_loq is True it is an extrapolation: report '< LOQ'.",
        mzlab_core::json::fmt_g(ef)
    );
    Ok((rows, Some(warning)))
}

impl QuantRow {
    /// The cells in the order of `QUANT_COLUMNS`, then `ENV_COLUMNS` when `with_env`.
    pub fn cells(&self, with_env: bool) -> Vec<Cell> {
        let mut v = vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::Str(self.std_name.clone()),
            Cell::Str(self.site.clone()),
            Cell::Str(self.adduct.clone()),
            Cell::num(Some(self.area)),
            Cell::num(Some(self.conc_ng_ml)),
            Cell::num(Some(self.r2)),
            Cell::Bool(self.r2_ok),
            self.below_loq.map_or(Cell::Null, Cell::Bool),
            Cell::num(self.area_loq),
            Cell::num(self.loq_ratio),
        ];
        if with_env {
            match &self.env {
                Some(e) => {
                    v.push(Cell::num(Some(e.conc_env_ng_l)));
                    v.push(Cell::num(Some(e.ef_spe)));
                    v.push(Cell::Bool(false));
                    v.push(Cell::Str(e.note.to_string()));
                }
                None => v.extend([Cell::Null, Cell::Null, Cell::Null, Cell::Null]),
            }
        }
        v
    }
}
