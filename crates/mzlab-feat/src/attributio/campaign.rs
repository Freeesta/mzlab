// Ported from Attributio campaign.py, paths.py, compat.py.
//! A campaign described by ONE file, `campaign.toml`: where the data are, how the sites are called in the raw files, which sites make a matrix, the sorbents,
//! the reference standards and the chemist's decisions. The defaults are those of Attributio. Paths are text relative to the campaign folder; the places outside
//! it (`{libraries}`, `{lab}`) become the absolute paths of `Env`, and what a leading «/» means is up to the `Io` that reads.

use super::error::{py_list, CampaignError, Result};
use super::io::{file_name, join};
use super::toml::{self, Table, Value};

pub const POLARITIES: [&str; 2] = ["POS", "NEG"];
pub const PROTON: f64 = 1.007276;
pub const CAMPAIGN_FILE: &str = "campaign.toml";
const DEFAULT_SAMPLE_COLUMN: &str = "datafile:{prefix}_{sorbent}_{tag}.mzML:{field}";
const DEFAULT_NSTAT: &str = "[n = 1 per site · exploratory · no statistical test]";

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
    merged_dir: Template,
    sample_column: Template,
    pub polarity_tag: Vec<(String, String)>,
    pub sorbents: Vec<String>,
    sorbent_table: Option<Template>,
    pub collapse_sorbent_twins: bool,
    pub twin_mz_ppm: f64,
    pub twin_rt_min: f64,
    pub neutral_mass_columns: Vec<String>,
    pub sirius_dir: String,
    pub sirius_outputs: Vec<(String, String)>,
    pub gnps_dir: String,
    gnps_pattern: Template,
    pub standards: Table,
    pub library_csv: Option<String>,
    pub calibration_csv: Option<String>,
    pub quantification: Table,
    pub count_2b_as_2: bool,
    pub review: Table,
    pub annotation: Table,
    pub presence_blank_fold: Option<f64>,
    pub feature_decisions_csv: Option<String>,
    pub sites: Vec<(String, Table)>,
    pub site_prefix: Vec<(String, String)>,
    pub matrices: Vec<(String, Matrix)>,
    pub env: Env,
}

struct Sec<'a> {
    file: &'a str,
    name: &'a str,
    t: Table,
}

impl Sec<'_> {
    fn bad(&self, key: &str, what: &str) -> CampaignError {
        CampaignError::campaign(format!(
            "{} [{}]: {key} must be {what}",
            self.file, self.name
        ))
    }
    fn str_opt(&self, key: &str) -> Result<Option<String>> {
        match self.t.get(key) {
            None => Ok(None),
            Some(Value::Str(s)) => Ok(Some(s.clone())),
            Some(_) => Err(self.bad(key, "a string")),
        }
    }
    fn str_or(&self, key: &str, default: &str) -> Result<String> {
        Ok(self.str_opt(key)?.unwrap_or_else(|| default.to_string()))
    }
    fn f64_or(&self, key: &str, default: f64) -> Result<f64> {
        match self.t.get(key) {
            None => Ok(default),
            Some(v) => v.as_f64().ok_or_else(|| self.bad(key, "a number")),
        }
    }
    fn bool_or(&self, key: &str, default: bool) -> Result<bool> {
        match self.t.get(key) {
            None => Ok(default),
            Some(v) => v.as_bool().ok_or_else(|| self.bad(key, "true or false")),
        }
    }
    fn list_or(&self, key: &str, default: &[&str]) -> Result<Vec<String>> {
        match self.t.get(key) {
            None => Ok(default.iter().map(|s| s.to_string()).collect()),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(String::from)
                        .ok_or_else(|| self.bad(key, "a list of strings"))
                })
                .collect(),
            Some(_) => Err(self.bad(key, "a list of strings")),
        }
    }
    fn template(&self, key: &str, default: &str, allowed: &[&str]) -> Result<Template> {
        let text = self.str_or(key, default)?;
        Template::parse(&text, allowed).map_err(|f| {
            CampaignError::campaign(format!(
                "{} [{}]: {key} uses the unknown field '{{{f}}}'",
                self.file, self.name
            ))
        })
    }
}

impl Campaign {
    /// Reads the text of a `campaign.toml`. `file_name` is the name used in messages, `root_name` the name of the campaign folder (the id when the file has none).
    pub fn from_toml(text: &str, file_name: &str, root_name: &str, env: Env) -> Result<Campaign> {
        let cfg = toml::parse(text).map_err(|e| {
            CampaignError::campaign(format!("{file_name}: line {}: {}", e.line, e.msg))
        })?;
        let sec = |name: &'static str| Sec {
            file: file_name,
            name,
            t: cfg.table(name).cloned().unwrap_or_default(),
        };
        let c = sec("campaign");
        let id = c
            .str_opt("id")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| root_name.to_string());
        let mut k = Campaign::blank(file_name, root_name, &id, &env);
        k.name = c.str_or("name", &id)?;
        k.instrument = c.str_or("instrument", "")?;
        k.fragmentation = c.str_or("fragmentation", "")?;
        k.nstat_tag = c.str_or("nstat_tag", DEFAULT_NSTAT)?;
        k.read_paths(&sec("paths"))?;
        k.read_mzmine(&sec("mzmine"))?;
        k.read_sirius_gnps(&sec("sirius"), &sec("gnps"))?;
        k.read_standards(&cfg, &sec("curation"))?;
        let lv = sec("levels");
        let unknown: Vec<&str> = lv.t.keys().filter(|x| *x != "count_2b_as_2").collect();
        if !unknown.is_empty() {
            let mut u = unknown;
            u.sort();
            return Err(CampaignError::campaign(format!(
                "{file_name} [levels]: unknown keys {}",
                py_list(&u)
            )));
        }
        k.count_2b_as_2 = lv.bool_or("count_2b_as_2", true)?;
        k.presence_blank_fold = sec("presence")
            .t
            .get("blank_fold")
            .and_then(Value::as_f64)
            .filter(|f| *f != 0.0);
        k.review = sec("review").t;
        k.annotation = sec("annotation").t;
        k.quantification = sec("quantification").t;
        k.read_sites(&cfg)?;
        Ok(k)
    }

    fn blank(file_name: &str, root_name: &str, id: &str, env: &Env) -> Campaign {
        let tpl = |t: &str, a: &[&str]| Template::parse(t, a).expect("fixed template");
        Campaign {
            file_name: file_name.into(),
            root_name: root_name.into(),
            id: id.into(),
            name: id.into(),
            instrument: String::new(),
            fragmentation: String::new(),
            nstat_tag: DEFAULT_NSTAT.into(),
            data_dir: String::new(),
            stats_dir: String::new(),
            scripts_dir: String::new(),
            characterization_csv: String::new(),
            mzmine_dir: String::new(),
            merged_dir: tpl("DDA_{pol}/merged", &["pol", "pol_lower"]),
            sample_column: tpl(
                DEFAULT_SAMPLE_COLUMN,
                &["prefix", "sorbent", "tag", "field"],
            ),
            polarity_tag: vec![
                ("POS".into(), "ddaPOS".into()),
                ("NEG".into(), "ddaNEG".into()),
            ],
            sorbents: vec!["OA".into(), "EC".into()],
            sorbent_table: None,
            collapse_sorbent_twins: false,
            twin_mz_ppm: 5.0,
            twin_rt_min: 0.1,
            neutral_mass_columns: Vec::new(),
            sirius_dir: String::new(),
            sirius_outputs: Vec::new(),
            gnps_dir: String::new(),
            gnps_pattern: tpl("GNPS2_output_{pol_lower}", &["pol", "pol_lower"]),
            standards: Table::default(),
            library_csv: None,
            calibration_csv: None,
            quantification: Table::default(),
            count_2b_as_2: true,
            review: Table::default(),
            annotation: Table::default(),
            presence_blank_fold: None,
            feature_decisions_csv: None,
            sites: Vec::new(),
            site_prefix: Vec::new(),
            matrices: Vec::new(),
            env: env.clone(),
        }
    }

    fn read_paths(&mut self, p: &Sec) -> Result<()> {
        self.data_dir = p.str_or("data", "02 Data")?;
        self.stats_dir = p.str_or("stats", "03 Stats")?;
        self.scripts_dir = join(&self.stats_dir, "scripts");
        self.characterization_csv = p.str_or("characterization", "02 Data/Characterization.csv")?;
        Ok(())
    }

    fn read_mzmine(&mut self, mz: &Sec) -> Result<()> {
        self.mzmine_dir = join(&self.data_dir, &mz.str_or("dir", "1 - mzMine")?);
        self.merged_dir = mz.template("merged_dir", "DDA_{pol}/merged", &["pol", "pol_lower"])?;
        self.sample_column = mz.template(
            "sample_column",
            DEFAULT_SAMPLE_COLUMN,
            &["prefix", "sorbent", "tag", "field"],
        )?;
        if let Some(Value::Table(t)) = mz.t.get("polarity_tag") {
            for (k, v) in &t.0 {
                let v = v
                    .as_str()
                    .ok_or_else(|| mz.bad("polarity_tag", "a table of strings"))?;
                match self.polarity_tag.iter_mut().find(|(x, _)| x == k) {
                    Some(e) => e.1 = v.to_string(),
                    None => self.polarity_tag.push((k.clone(), v.to_string())),
                }
            }
        }
        self.sorbents = mz.list_or("sorbents", &["OA", "EC"])?;
        if let Some(pat) = mz.str_opt("sorbent_table")? {
            self.sorbent_table =
                Some(mz.template("sorbent_table", &pat, &["pol", "pol_lower", "sorbent"])?);
        }
        self.collapse_sorbent_twins = mz.bool_or("collapse_sorbent_twins", false)?;
        self.twin_mz_ppm = mz.f64_or("twin_mz_ppm", 5.0)?;
        self.twin_rt_min = mz.f64_or("twin_rt_min", 0.1)?;
        if self.collapse_sorbent_twins && self.sorbent_table.is_none() {
            return Err(CampaignError::campaign(format!(
                "{} [mzmine]: collapse_sorbent_twins needs sorbent_table",
                self.file_name
            )));
        }
        self.neutral_mass_columns = mz.list_or(
            "neutral_mass_columns",
            &[
                "ion_identities:neutral_mass",
                "preferred_annotation:neutral_mass",
            ],
        )?;
        Ok(())
    }

    fn read_sirius_gnps(&mut self, si: &Sec, gn: &Sec) -> Result<()> {
        self.sirius_dir = join(&self.data_dir, &si.str_or("dir", "2 - SIRIUS")?);
        if let Some(Value::Table(t)) = si.t.get("output_dirs") {
            for (k, v) in &t.0 {
                let v = v
                    .as_str()
                    .ok_or_else(|| si.bad("output_dirs", "a table of strings"))?;
                self.sirius_outputs.push((k.clone(), v.to_string()));
            }
        }
        self.gnps_dir = join(&self.data_dir, &gn.str_or("dir", "3 - GNPS")?);
        self.gnps_pattern = gn.template(
            "output_dir",
            "GNPS2_output_{pol_lower}",
            &["pol", "pol_lower"],
        )?;
        Ok(())
    }

    fn read_standards(&mut self, cfg: &Table, cu: &Sec) -> Result<()> {
        let st = cfg.table("standards").cloned().unwrap_or_default();
        let sec = Sec {
            file: &self.file_name,
            name: "standards",
            t: st.clone(),
        };
        self.library_csv = sec
            .str_opt("library")?
            .filter(|s| !s.is_empty())
            .map(|s| expand(&s, &self.env));
        self.calibration_csv = sec
            .str_opt("calibration")?
            .filter(|s| !s.is_empty())
            .map(|s| expand(&s, &self.env));
        self.standards = st;
        self.feature_decisions_csv = cu
            .str_opt("feature_decisions")?
            .filter(|s| !s.is_empty())
            .map(|s| expand(&s, &self.env));
        Ok(())
    }

    fn read_sites(&mut self, cfg: &Table) -> Result<()> {
        if let Some(sites) = cfg.table("sites") {
            for (k, v) in &sites.0 {
                if let Value::Table(t) = v {
                    self.sites.push((k.clone(), t.clone()));
                    let prefix = t.get("file_prefix").and_then(Value::as_str).unwrap_or(k);
                    self.site_prefix.push((k.clone(), prefix.to_string()));
                }
            }
        }
        if let Some(ms) = cfg.table("matrices") {
            for (k, v) in &ms.0 {
                let Value::Table(m) = v else { continue };
                let sites: Vec<String> = match m.get("sites") {
                    Some(Value::Array(a)) => a
                        .iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect(),
                    _ => {
                        return Err(CampaignError::campaign(format!(
                            "{} [matrices.{k}]: sites is missing",
                            self.file_name
                        )))
                    }
                };
                let dir = m
                    .get("dir")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(String::from);
                let label = m
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(k)
                    .to_string();
                self.matrices
                    .push((k.clone(), Matrix { sites, dir, label }));
            }
        }
        for (mk, m) in &self.matrices {
            let unknown: Vec<&String> = m
                .sites
                .iter()
                .filter(|s| !self.sites.iter().any(|(k, _)| k == *s))
                .collect();
            if !unknown.is_empty() {
                return Err(CampaignError::campaign(format!(
                    "{}: matrix '{mk}' lists unknown sites {}",
                    self.file_name,
                    py_list(&unknown)
                )));
            }
        }
        Ok(())
    }

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
