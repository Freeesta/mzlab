// Ported from Attributio annotate.py.
//! One annotated feature: the evidence of every source joined on `(pol, id)`, the outcome of each gate and the level that follows.

use crate::attributio::readers::standards::StandardMatch;

/// The five CANOPUS levels, in the order of `Canopus::classes`.
pub const CLASS_COLUMNS: [&str; 5] = [
    "npc_pathway",
    "npc_superclass",
    "npc_class",
    "cf_superclass",
    "cf_class",
];

/// Gates of the library sources, in the order of precedence used to break ties.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gates {
    pub l1: bool,
    pub sirius_spectral: bool,
    pub gnps2: bool,
    pub mzmine_library: bool,
    pub csi: bool,
    pub std_rt: bool,
    pub spec_putative: bool,
    pub gnps_putative: bool,
    pub mzlib_putative: bool,
}

/// The evidence of the library sources.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpecEvidence {
    pub name: Option<String>,
    pub sim: Option<f64>,
    pub peaks: Option<f64>,
    pub inchikey2d: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GnpsEvidence {
    pub name: Option<String>,
    pub score: Option<f64>,
    pub peaks: Option<f64>,
    pub massdiff: Option<f64>,
    pub exact: bool,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MzlibEvidence {
    pub name: Option<String>,
    pub score: Option<f64>,
    pub peaks: Option<f64>,
    pub precursor_mz: Option<f64>,
    pub adduct: Option<String>,
    pub formula: Option<String>,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub massdiff_ppm: Option<f64>,
    pub exact: bool,
}

/// The best MS2 Engine hit: evidence only, never a gate, a level or an identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EngineEvidence {
    pub name: Option<String>,
    pub score: Option<f64>,
    pub precursor_mz: Option<f64>,
    pub library_id: Option<String>,
}

/// CSI:FingerID: the structure with its COSMIC confidence.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CsiEvidence {
    pub name: Option<String>,
    pub formula: Option<String>,
    pub xlogp: Option<f64>,
    pub conf: Option<f64>,
    pub cosmic: Option<f64>,
    pub inchikey2d: Option<String>,
    pub smiles: Option<String>,
}

/// A feature that the chemist's decision file moved or named.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decision {
    /// The reason the chemist wrote.
    pub feature_decision: String,
    pub group_override: Option<String>,
    pub decision_2b: bool,
    /// The chemist wrote `3b` (a generic class with many plausible isomers): the Level 3 is 3b instead of 3a.
    pub sub_3b: bool,
    /// `evidence_*` columns of a 2b decision.
    pub evidence: Vec<(String, String)>,
}

/// The retention-time rule's outcome for a feature (`None` fields: the feature is not Level 1-3 or has no identity).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RtInfo {
    pub identity_override: Option<String>,
    pub rt_key: Option<String>,
    pub rt_group: Option<usize>,
    pub rt_groups: Option<usize>,
    pub rt_ref: Option<f64>,
    pub rt_min: Option<f64>,
    pub rt_max: Option<f64>,
    pub rt_reference_group: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feature {
    pub pol: String,
    pub id: i64,
    pub mz: Option<f64>,
    pub rt: Option<f64>,
    pub in_site: Vec<(String, bool)>,
    pub any_sorbent: Vec<(String, bool)>,
    /// CANOPUS classes shown (those under `canopus_min` are `None`) and their raw probabilities.
    pub classes: [Option<String>; 5],
    pub class_probs: [Option<f64>; 5],
    pub csi: CsiEvidence,
    pub spec: SpecEvidence,
    pub gnps: GnpsEvidence,
    pub mzlib: MzlibEvidence,
    pub engine: EngineEvidence,
    pub std: Option<StandardMatch>,
    pub gnps_component: Option<i64>,
    pub gnps_component_size: Option<i64>,
    pub gates: Gates,
    pub level: i32,
    pub level_sub: String,
    pub level_basis: String,
    pub level_label: String,
    pub level_flag: Option<String>,
    pub high_conf_struct: bool,
    pub inchikey_skeleton: Option<String>,
    pub smiles: Option<String>,
    pub inchikey_source: Option<String>,
    pub sorbent_marker: String,
    pub sites: Vec<String>,
    pub n_sites: usize,
    pub sources: Vec<&'static str>,
    pub disp_name: Option<String>,
    pub cec: Option<(String, String)>,
    pub cec_headline: bool,
    pub name_conflict: bool,
    pub library_conflict: bool,
    pub name_blocks: String,
    pub name_choice: Option<String>,
    pub library_margin: Option<f64>,
    pub decision: Decision,
    pub rt_info: RtInfo,
    pub level_why: String,
    pub level_decisive: String,
}

impl Feature {
    /// The text of a site flag.
    pub fn in_site(&self, site: &str) -> bool {
        self.in_site.iter().any(|(s, v)| s == site && *v)
    }
    /// Whether the MS2 Engine hit names what the cascade calls the feature (letters and digits, case ignored); `None` without a hit or without a name to compare.
    pub fn engine_agrees(&self) -> Option<bool> {
        let key = |t: &str| {
            t.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let hit = self.engine.name.as_deref().filter(|n| !n.is_empty())?;
        let ours = self.identity().or(self.disp_name.as_deref())?;
        Some(key(hit) == key(ours))
    }
    /// The CSI:FingerID structure name (the column `name` of the reference tool).
    pub fn name(&self) -> Option<&str> {
        self.csi.name.as_deref()
    }
    /// `identity_override` or the structure name: what the reference tool writes as the identity of a feature.
    pub fn identity(&self) -> Option<&str> {
        self.rt_info
            .identity_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.csi.name.as_deref().filter(|s| !s.is_empty()))
    }
}
