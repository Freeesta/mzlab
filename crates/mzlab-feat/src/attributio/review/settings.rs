// Ported from Attributio src/attributio/review/settings.py.
//
// Every rule and threshold of the review in one place. A campaign may override any of them in its `campaign.toml`:
//
//     [review]
//     ppm_ion = 5.0
//     exclude_raw_prefixes = ["Po_Carignano"]
//
// Unknown keys are refused (a typo would silently keep the default). The thresholds decide only the automatic proposal shown to the chemist and
// which candidates are listed; nothing here changes a level.

/// A value of the `[review]` table: a number, a text or a list of them.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    List(Vec<Value>),
}

trait Field: Sized {
    fn from_value(v: &Value) -> Option<Self>;
    fn to_value(&self) -> Value;
}

impl Field for f64 {
    fn from_value(v: &Value) -> Option<f64> {
        if let Value::Num(n) = v {
            Some(*n)
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Num(*self)
    }
}

impl Field for i64 {
    fn from_value(v: &Value) -> Option<i64> {
        if let Value::Num(n) = v {
            (n.fract() == 0.0).then_some(*n as i64)
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Num(*self as f64)
    }
}

impl Field for String {
    fn from_value(v: &Value) -> Option<String> {
        if let Value::Str(s) = v {
            Some(s.clone())
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::Str(self.clone())
    }
}

impl<T: Field> Field for Vec<T> {
    fn from_value(v: &Value) -> Option<Vec<T>> {
        if let Value::List(l) = v {
            l.iter().map(T::from_value).collect()
        } else {
            None
        }
    }
    fn to_value(&self) -> Value {
        Value::List(self.iter().map(Field::to_value).collect())
    }
}

macro_rules! settings {
    ($( $(#[$doc:meta])* $name:ident : $ty:ty = $def:expr ;)*) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct ReviewSettings { $( $(#[$doc])* pub $name: $ty, )* }

        impl Default for ReviewSettings {
            fn default() -> Self { ReviewSettings { $( $name: $def, )* } }
        }

        impl ReviewSettings {
            /// The keys a campaign may set.
            pub const KEYS: &'static [&'static str] = &[ $( stringify!($name), )* ];

            fn set(&mut self, key: &str, v: &Value) -> Result<(), String> {
                match key {
                    $( stringify!($name) => {
                        self.$name = <$ty as Field>::from_value(v).ok_or_else(|| format!("[review] {key}: wrong type"))?;
                        Ok(())
                    } )*
                    _ => Err(format!("[review]: unknown key {key}")),
                }
            }

            /// Every setting with its value, in declaration order.
            pub fn entries(&self) -> Vec<(&'static str, Value)> {
                vec![ $( (stringify!($name), Field::to_value(&self.$name)), )* ]
            }
        }
    };
}

settings! {
    // explanation of the spectrum by the candidate structure
    /// precursor vs named structure (adduct search)
    ppm_ion: f64 = 5.0;
    /// fragment vs sub-formula of the ion (SIRIUS MS2 tolerance)
    ppm_frag: f64 = 10.0;
    decoy_shifts: Vec<f64> = vec![-0.050, -0.035, -0.020, 0.020, 0.035, 0.050];
    /// above this the sub-formula table is not built
    max_subformulas: i64 = 3_000_000;
    // proposal tiers (sorting aid, never a verdict)
    min_explained_peaks: i64 = 3;
    /// of the MS2 intensity
    min_explained_frac: f64 = 0.70;
    /// explained fraction minus decoy fraction
    min_margin: f64 = 0.30;
    /// matched visible peaks with a same-structure reference (2b-strong)
    ref_min_matched: i64 = 3;
    /// cosine on visible peaks (2b-strong)
    ref_min_cos: f64 = 0.50;
    l2a_min_cos_vis: f64 = 0.70;
    l2a_min_matched_vis: i64 = 4;
    /// co-isolated ion / precursor in the MS1 isolation window (reported)
    chim_rel: f64 = 0.10;
    /// a co-isolated ion this large caps a proposal
    chim_cap: f64 = 0.50;
    iso_tol_abs: f64 = 0.06;
    iso_tol_rel: f64 = 0.35;
    // spectral matching
    /// fragment matching between spectra (GNPS convention)
    tol_da: f64 = 0.02;
    /// library precursor vs feature precursor («same precursor ion»)
    precursor_ppm: f64 = 10.0;
    /// best library matches kept per feature
    top_n: i64 = 5;
    /// library precursors within ± this of the feature (analogs)
    analog_window_da: f64 = 250.0;
    /// candidates scored with the modified cosine after the prefilter
    analog_prefilter: i64 = 200;
    /// |delta m| below this is not an analog (it is the same ion)
    analog_min_shift_da: f64 = 0.5;
    /// |delta m| within this (ppm) = exact match, not analog
    exact_ppm: f64 = 10.0;
    /// free CHNOPS search of delta m: within this (mDa)
    delta_formula_mda: f64 = 5.0;
    // input spectra
    /// peaks below 1 % of the base peak are dropped
    rel_min: f64 = 0.01;
    // extra libraries: files or folders, besides the libraries root and the downloaded ones
    extra_libraries: Vec<String> = vec![];
    // raw files
    /// where the mzML files are (default: searched)
    raw_dir: String = String::new();
    /// samples never used for the raw scan (e.g. a control)
    exclude_raw_prefixes: Vec<String> = vec![];
    // MS1 tab (isotope pattern)
    /// MS1 window below the precursor (Da)
    ms1_lo_da: f64 = 1.5;
    /// and above it (A+1 ... A+6, Cl/Br/S A+2 visible)
    ms1_hi_da: f64 = 6.0;
    /// MS1 peaks below this fraction of the precursor peak are not stored
    ms1_rel_min: f64 = 0.003;
    /// MS1 resolving power for the optional broadening (0 = unknown)
    ms1_resolution: f64 = 0.0;
    ms1_resolution_at: f64 = 200.0;
    /// candidate formulas drawn in the MS1 tab
    ms1_max_options: i64 = 6;
    // EIC panel and co-elution (raw MS1)
    /// EIC from apex - this to apex + this (min)
    eic_half_min: f64 = 0.75;
    /// XICs of the co-isolated ions around the MS2 scan (min)
    coelution_half_min: f64 = 0.30;
    /// procedural blank files in the raw folder
    blank_raw_prefixes: Vec<String> = ["blk", "blank", "Blank", "BLK"].map(String::from).to_vec();
    /// ... but not the instrument blanks
    blank_raw_exclude: Vec<String> = vec!["instr".to_string()];
    // paired polarity (POS <-> NEG)
    /// neutral masses of the two ions within this (ppm)
    pair_ppm: f64 = 5.0;
    /// and retention times within this (min)
    pair_rt_min: f64 = 0.15;
    // in-source ions and adducts
    /// co-elution window (min) for the adduct / in-source-fragment steps
    insource_rt_min: f64 = 0.05;
    insource_ppm: f64 = 5.0;
    // RT versus logP: a plausibility check, never applied
    /// `[[t_min, %B], ...]` of the LC method; empty: RT is fitted directly
    lc_gradient: Vec<Vec<f64>> = vec![];
    /// gradient delay (dwell volume / flow), min
    lc_dwell_min: f64 = 0.0;
    /// prediction window = ± k × residual MAD
    rt_logp_k: f64 = 3.0;
    /// its floor (0 = 3 %B with a gradient, 1.0 min without)
    rt_logp_floor: f64 = 0.0;
    // reference fetch by USI (fallback when a spectrum is not in a local library)
    fetch_timeout_s: f64 = 15.0;
    /// the whole fetch step never takes longer than this
    fetch_budget_s: f64 = 600.0;
    fetch_workers: i64 = 2;
    // page size
    /// experimental m/z in the page
    exp_decimals: i64 = 5;
    /// reference m/z in the page (library precision is often lower)
    ref_decimals: i64 = 5;
}

impl ReviewSettings {
    /// The settings of a campaign from the text of its `campaign.toml` (`config_name` is the file name, for the messages): the defaults with the
    /// keys of the `[review]` table set. An unknown key is refused as a whole, before any is applied.
    pub fn from_campaign_toml(config_name: &str, text: &str) -> Result<ReviewSettings, String> {
        let cfg = review_table(text)?;
        let mut bad: Vec<&str> = cfg
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| !Self::KEYS.contains(k))
            .collect();
        if !bad.is_empty() {
            bad.sort_unstable();
            let list: Vec<String> = bad.iter().map(|k| format!("'{k}'")).collect();
            return Err(format!(
                "{config_name} [review]: unknown keys [{}]",
                list.join(", ")
            ));
        }
        let mut s = ReviewSettings::default();
        for (k, v) in &cfg {
            s.set(k, v)?;
        }
        Ok(s)
    }
}

// ------------------------------------------------------------------------------ the [review] table of a TOML text

/// The entries of the `[review]` table: numbers, quoted texts and (nested, multi-line) arrays of them.
fn review_table(text: &str) -> Result<Vec<(String, Value)>, String> {
    let (mut out, mut inside, mut pending) = (Vec::new(), false, String::new());
    for raw in text.lines() {
        let line = strip_comment(raw);
        let line = line.trim();
        if pending.is_empty() && line.starts_with('[') && !line.contains('=') {
            inside = line.trim_end_matches(']').trim_start_matches('[').trim() == "review"
                && line.ends_with(']');
            continue;
        }
        if !inside || (line.is_empty() && pending.is_empty()) {
            continue;
        }
        pending.push_str(line);
        pending.push(' ');
        if depth(&pending) > 0 {
            continue;
        }
        let entry = std::mem::take(&mut pending);
        let (k, v) = entry
            .split_once('=')
            .ok_or_else(|| format!("[review]: cannot read «{}»", entry.trim()))?;
        let mut p = Parser {
            s: v.trim().chars().collect(),
            i: 0,
        };
        out.push((k.trim().trim_matches('"').to_string(), p.value()?));
    }
    Ok(out)
}

/// A line without its `#` comment (a `#` inside a quoted text stays).
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '#') => return &line[..i],
            _ => {}
        }
    }
    line
}

/// Open brackets not yet closed (outside quoted texts).
fn depth(s: &str) -> i32 {
    let (mut d, mut quote) = (0, None::<char>);
    for c in s.chars() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '[') => d += 1,
            (None, ']') => d -= 1,
            _ => {}
        }
    }
    d
}

struct Parser {
    s: Vec<char>,
    i: usize,
}

impl Parser {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.s.get(self.i) {
            Some('[') => {
                self.i += 1;
                let mut l = Vec::new();
                loop {
                    self.ws();
                    match self.s.get(self.i) {
                        Some(']') => {
                            self.i += 1;
                            return Ok(Value::List(l));
                        }
                        Some(',') => self.i += 1,
                        Some(_) => l.push(self.value()?),
                        None => return Err("[review]: unclosed array".to_string()),
                    }
                }
            }
            Some(&q) if q == '"' || q == '\'' => {
                self.i += 1;
                let mut t = String::new();
                while let Some(&c) = self.s.get(self.i) {
                    self.i += 1;
                    match c {
                        c if c == q => return Ok(Value::Str(t)),
                        '\\' if q == '"' => {
                            let e = self.s.get(self.i).copied().unwrap_or('\\');
                            self.i += 1;
                            t.push(match e {
                                'n' => '\n',
                                't' => '\t',
                                e => e,
                            });
                        }
                        c => t.push(c),
                    }
                }
                Err("[review]: unclosed text".to_string())
            }
            Some(_) => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| !matches!(c, ',' | ']') && !c.is_whitespace())
                {
                    self.i += 1;
                }
                let tok: String = self.s[start..self.i]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                tok.parse::<f64>()
                    .map(Value::Num)
                    .map_err(|_| format!("[review]: cannot read the value «{tok}»"))
            }
            None => Err("[review]: missing value".to_string()),
        }
    }
}
