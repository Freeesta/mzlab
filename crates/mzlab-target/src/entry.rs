// Target entry representation and adduct types for HRMS screening lists.
use serde::{Deserialize, Serialize};

/// Supported electrospray ionization (ESI) adduct types for target screening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdductType {
    /// Protonated ion: [M+H]+
    MH,
    /// Sodiated ion: [M+Na]+
    MNa,
    /// Ammoniated ion: [M+NH4]+
    MNH4,
}

impl AdductType {
    /// Theoretical delta mass added to the neutral molecule in Da.
    #[inline]
    pub fn delta_mass(&self) -> f64 {
        match self {
            Self::MH => 1.007276,
            Self::MNa => 22.989218,
            Self::MNH4 => 18.033823,
        }
    }

    /// Technical adduct label notation (e.g. "[M+H]+").
    #[inline]
    pub fn label(&self) -> &'static str {
        match self {
            Self::MH => "[M+H]+",
            Self::MNa => "[M+Na]+",
            Self::MNH4 => "[M+NH4]+",
        }
    }
}

/// Verification flags parsed from the check column of target entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CheckFlags {
    /// True when the entry is a salt or hydrate whose Neutral formula and monoisotopic
    /// mass represent the free active principle detected in HRMS ESI mode.
    pub is_salt_or_hydrate: bool,
    /// True when stereochemistry is specified in InChIKey but omitted in SMILES;
    /// Neutral formula, mass and connectivity are valid for MS1/MS2 screening.
    pub stereo_to_verify: bool,
}

impl CheckFlags {
    /// Parse flags from raw check comment text.
    pub fn from_check_str(s: &str) -> Self {
        let lower = s.to_ascii_lowercase();
        let is_salt_or_hydrate =
            lower.contains("sale/idrato") || lower.contains("sale") || lower.contains("idrato");
        let stereo_to_verify = lower.contains("stereo") || lower.contains("stereochimica");
        Self {
            is_salt_or_hydrate,
            stereo_to_verify,
        }
    }
}

/// A target or suspect substance entry in an HRMS screening list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetEntry {
    /// Unique identifier slug.
    pub id: String,
    /// English common name.
    pub name_en: String,
    /// Italian common name.
    pub name_it: String,
    /// Neutral formula (molecular formula of the neutral active principle).
    pub neutral_formula: String,
    /// Accurate monoisotopic mass of the neutral molecule in Da.
    pub monoisotopic_mass: f64,
    /// Theoretical [M+H]+ m/z.
    pub mz_mh: f64,
    /// Theoretical [M+Na]+ m/z.
    pub mz_mna: f64,
    /// Theoretical [M+NH4]+ m/z.
    pub mz_mnh4: f64,
    /// SMILES molecular structure string.
    pub smiles: String,
    /// Standard 27-character InChIKey.
    pub inchikey: String,
    /// 14-character connectivity block of the InChIKey.
    pub inchikey_connectivity: String,
    /// Optional Chemical Abstracts Service (CAS) registry number.
    pub cas: Option<String>,
    /// Optional PubChem compound identifier (CID).
    pub pubchem_cid: Option<u64>,
    /// Pharmacological or physiological effect classification.
    pub effect_class: String,
    /// Chemical scaffold classification.
    pub chemical_class: String,
    /// Substance role (e.g. active principle, adulterant, residue).
    pub role: String,
    /// Flag indicating whether psychoactive effects are expected.
    pub psychoactive: bool,
    /// Alternative names and synonyms.
    pub synonyms: Vec<String>,
    /// Raw curator annotation from the check column, if present.
    pub check: Option<String>,
    /// Structured flags derived from curator annotations.
    pub check_flags: CheckFlags,
}

impl TargetEntry {
    /// Retrieve the precomputed theoretical m/z for the requested adduct type.
    #[inline]
    pub fn mz_for_adduct(&self, adduct: AdductType) -> f64 {
        match adduct {
            AdductType::MH => self.mz_mh,
            AdductType::MNa => self.mz_mna,
            AdductType::MNH4 => self.mz_mnh4,
        }
    }
}
