// Ported from Attributio campaign.py, paths.py.
//! Reading `campaign.toml` into a `Campaign`: sections, defaults and the messages of Attributio for what is wrong.

use super::campaign::*;
use super::error::{py_list, CampaignError, Result};
use super::io::join;
use super::toml::{self, Table, Value};

/// Keys of `[curation]` that name a curation table, in the order of `Campaign::curation_csv`.
pub(crate) const CURATION_KEYS: [&str; 3] =
    ["name_map", "category_override", "identity_formula_override"];

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
        k.gradient_same_sorbent = sec("gradient")
            .t
            .get("same_sorbent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
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
            gradient_same_sorbent: false,
            count_2b_as_2: true,
            review: Table::default(),
            annotation: Table::default(),
            presence_blank_fold: None,
            feature_decisions_csv: None,
            curation_csv: [None, None, None],
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
        for (slot, key) in self.curation_csv.iter_mut().zip(CURATION_KEYS) {
            *slot = cu
                .str_opt(key)?
                .filter(|s| !s.is_empty())
                .map(|s| expand(&s, &self.env));
        }
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
