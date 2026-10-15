// Ported from Attributio campaign.py, paths.py, compat.py.
//! A campaign described by ONE file, `campaign.toml`: where the data are, how the sites are called in the raw files, which sites make a matrix, the sorbents,
//! the reference standards and the chemist's decisions. The defaults are those of Attributio. Paths are text relative to the campaign folder; the places outside
//! it (`{libraries}`, `{lab}`) become the absolute paths of `Env`, and what a leading «/» means is up to the `Io` that reads.

use super::io::{file_name, join};
use super::toml::Table;

pub const POLARITIES: [&str; 2] = ["POS", "NEG"];
pub const PROTON: f64 = 1.007276;
pub const CAMPAIGN_FILE: &str = "campaign.toml";
pub(super) const DEFAULT_SAMPLE_COLUMN: &str = "datafile:{prefix}_{sorbent}_{tag}.mzML:{field}";
pub(super) const DEFAULT_NSTAT: &str = "[n = 1 per site · exploratory · no statistical test]";

impl Campaign {
    // ----------------------------------------------------------------- names and places
    pub fn tag(&self, pol: &str) -> &str {
        self.polarity_tag
            .iter()
            .find(|(k, _)| k == pol)
            .map_or("", |(_, v)| v)
    }
    pub fn site_keys(&self) -> Vec<&str> {
        self.sites.iter().map(|(k, _)| k.as_str()).collect()
    }
    pub fn matrix_sites(&self, matrix: &str) -> Option<&[String]> {
        self.matrices
            .iter()
            .find(|(k, _)| k == matrix)
            .map(|(_, m)| m.sites.as_slice())
    }
    /// Per-sample column name, e.g. `datafile:CP_IN_OA_ddaPOS.mzML:feature_state`; a site not in the file is used as it is.
    pub fn col(&self, site: &str, sorbent: &str, pol: &str, field: &str) -> String {
        let prefix = self
            .site_prefix
            .iter()
            .find(|(k, _)| k == site)
            .map_or(site, |(_, v)| v.as_str());
        self.sample_column.render(&[
            ("prefix", prefix),
            ("sorbent", sorbent),
            ("tag", self.tag(pol)),
            ("field", field),
        ])
    }
    pub fn merged_dir(&self, pol: &str) -> String {
        join(
            &self.mzmine_dir,
            &self
                .merged_dir
                .render(&[("pol", pol), ("pol_lower", &pol.to_lowercase())]),
        )
    }
    pub fn has_sorbent_tables(&self) -> bool {
        self.sorbent_table.is_some()
    }
    /// The feature list of one sorbent (`None` when the campaign has no `sorbent_table`).
    pub fn sorbent_table_path(&self, pol: &str, sorbent: &str) -> Option<String> {
        self.sorbent_table.as_ref().map(|t| {
            join(
                &self.mzmine_dir,
                &t.render(&[
                    ("pol", pol),
                    ("pol_lower", &pol.to_lowercase()),
                    ("sorbent", sorbent),
                ]),
            )
        })
    }
    pub fn gnps_output_dir(&self, pol: &str) -> String {
        join(
            &self.gnps_dir,
            &self
                .gnps_pattern
                .render(&[("pol", pol), ("pol_lower", &pol.to_lowercase())]),
        )
    }
    pub fn explicit_sirius_dir(&self, pol: &str) -> Option<&str> {
        self.sirius_outputs
            .iter()
            .find(|(k, _)| k == pol)
            .map(|(_, v)| v.as_str())
    }
    /// The file name of the campaign file as a path (for messages).
    pub fn config_name(&self) -> &str {
        file_name(&self.file_name)
    }
}

/// The places outside the campaign folder that a path may name: `{lab}` and `{libraries}`.
#[derive(Clone, Debug, PartialEq)]
pub struct Env {
    pub lab: String,
    pub libraries: String,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            lab: "/HRMS_Lab".into(),
            libraries: "/HRMS_Lab/Librerie HRMS".into(),
        }
    }
}

/// A text with `{name}` fields (`str.format`): the fields are checked when the campaign is read.
#[derive(Clone, Debug, PartialEq)]
pub struct Template(Vec<(bool, String)>);

impl Template {
    pub fn parse(t: &str, allowed: &[&str]) -> std::result::Result<Template, String> {
        let mut out: Vec<(bool, String)> = vec![(false, String::new())];
        let mut it = t.chars().peekable();
        while let Some(c) = it.next() {
            match (c, it.peek()) {
                ('{', Some('{')) | ('}', Some('}')) => {
                    it.next();
                    out.last_mut().expect("piece").1.push(c);
                }
                ('{', _) => {
                    let name: String = it.by_ref().take_while(|x| *x != '}').collect();
                    if !allowed.contains(&name.as_str()) {
                        return Err(name);
                    }
                    out.push((true, name));
                    out.push((false, String::new()));
                }
                (c, _) => out.last_mut().expect("piece").1.push(c),
            }
        }
        Ok(Template(out))
    }
    pub fn render(&self, vals: &[(&str, &str)]) -> String {
        let mut s = String::new();
        for (field, text) in &self.0 {
            if *field {
                s.push_str(vals.iter().find(|(k, _)| k == text).map_or("", |(_, v)| v));
            } else {
                s.push_str(text);
            }
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub sites: Vec<String>,
    pub dir: Option<String>,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct Campaign {
    pub file_name: String,
    pub root_name: String,
    pub id: String,
    pub name: String,
    pub instrument: String,
    pub fragmentation: String,
    pub nstat_tag: String,
    pub data_dir: String,
    pub stats_dir: String,
    pub scripts_dir: String,
    pub characterization_csv: String,
    pub mzmine_dir: String,
    pub(super) merged_dir: Template,
    pub(super) sample_column: Template,
    pub polarity_tag: Vec<(String, String)>,
    pub sorbents: Vec<String>,
    pub(super) sorbent_table: Option<Template>,
    pub collapse_sorbent_twins: bool,
    pub twin_mz_ppm: f64,
    pub twin_rt_min: f64,
    pub neutral_mass_columns: Vec<String>,
    pub sirius_dir: String,
    pub sirius_outputs: Vec<(String, String)>,
    pub gnps_dir: String,
    pub(super) gnps_pattern: Template,
    pub standards: Table,
    pub library_csv: Option<String>,
    pub calibration_csv: Option<String>,
    pub quantification: Table,
    pub count_2b_as_2: bool,
    pub review: Table,
    pub annotation: Table,
    pub presence_blank_fold: Option<f64>,
    pub feature_decisions_csv: Option<String>,
    /// The curation tables of `[curation]` (`name_map`, `category_override`, `identity_formula_override`), read by `curation::TableCuration`.
    pub curation_csv: [Option<String>; 3],
    pub sites: Vec<(String, Table)>,
    pub site_prefix: Vec<(String, String)>,
    pub matrices: Vec<(String, Matrix)>,
    pub env: Env,
}

/// `paths.expand`: `{libraries}`, `{lab}` and `{campaign}` in a path written in the campaign file; a path without a token stays relative to the campaign folder.
pub fn expand(template: &str, env: &Env) -> String {
    let s = template
        .replace("{libraries}", &env.libraries)
        .replace("{lab}", &env.lab);
    match s.strip_prefix("{campaign}") {
        Some(rest) => rest.trim_start_matches('/').to_string(),
        None => s.replace("{campaign}", "."),
    }
}

/// The names of the old scripts of Attributio (`compat.py`) and the field of `Campaign` (or the function) that gives the same thing.
pub const HISTORICAL_NAMES: &[(&str, &str)] = &[
    ("ROOT", "root_name"),
    ("DATA", "data_dir"),
    ("MZMINE", "mzmine_dir"),
    ("MZ", "mzmine_dir"),
    ("SIRIUS", "sirius_dir"),
    ("SIR", "sirius_dir"),
    ("GNPS", "gnps_dir"),
    ("THAI_DIR", "stats_dir"),
    ("STATS_DIR", "stats_dir"),
    ("CHAR_CSV", "characterization_csv"),
    ("LIBRARY_CSV", "library_csv"),
    ("CALIBRATION_CSV", "calibration_csv"),
    ("SITE_PHYSICAL", "site_prefix"),
    ("SITE_FILE_PREFIX", "site_prefix"),
    ("SORBENTS", "sorbents"),
    ("MATRICES", "matrices"),
    ("SITE_META", "site_meta"),
    ("NSTAT_TAG", "nstat_tag"),
    ("matrix_sites", "Campaign::matrix_sites"),
    ("_resolve_merged", "sources::merged_table_path"),
    ("load_csv", "sources::Loader::feature_table"),
    ("col", "Campaign::col"),
    ("has_site", "detection::has_site"),
    ("detected_mask", "detection::detected_mask"),
    ("site_detected_any", "detection::site_detected_any"),
    ("matrix_detected", "detection::matrix_detected"),
    ("neutral_mass", "detection::neutral_mass"),
    ("site_id_set", "detection::site_id_set"),
    ("spe_overlap", "detection::spe_overlap"),
    ("site_area", "detection::site_area"),
    ("SIR_DIRS", "sources::sirius_output_dir"),
    ("load_sirius", "sources::Loader::sirius_frame"),
    ("load_mix_pharma", "sources::Loader::standards_library"),
    ("_load_mzmine_lib", "readers::mzmine::load_mzmine_library"),
    ("_load_gnps", "readers::gnps::load_gnps_library"),
    ("_load_gnps_network", "readers::gnps::load_gnps_network"),
    ("_clean_gnps_name", "readers::gnps::clean_library_hit_name"),
    ("_detection", "detection::detection_table"),
];
