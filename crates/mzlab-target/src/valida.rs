// Validation logic for target screening entries and adduct mass coherence.
use serde::{Deserialize, Serialize};

use mzlab_core::elements::{mass, parse_formula};

use crate::entry::{AdductType, TargetEntry};

/// Largest accepted gap (Da) between the stated monoisotopic mass and the mass of the Neutral formula (the same 1 mDa as the adduct checks).
const FORMULA_MASS_TOLERANCE: f64 = 0.001;

/// Validation error encountered during entry inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Identifier of the target entry with an issue.
    pub entry_id: String,
    /// Technical description of the validation failure.
    pub message: String,
}

/// Comprehensive validation report for a target screening collection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Total count of inspected target entries.
    pub total_entries: usize,
    /// Total count of target entries passing all validation checks.
    pub valid_entries: usize,
    /// Count of target entries marked as salt or hydrate forms.
    pub salts_or_hydrates: usize,
    /// Count of target entries requiring stereochemical verification.
    pub stereo_to_verify: usize,
    /// Collection of validation errors encountered.
    pub errors: Vec<ValidationError>,
    /// Findings that do not invalidate an entry (a mass that disagrees with its Neutral formula).
    #[serde(default)]
    pub warnings: Vec<ValidationError>,
}

impl ValidationReport {
    /// True when no validation errors exist and every entry is valid.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.valid_entries == self.total_entries
    }
}

/// A formula that parses but weighs something else than the stated mass would give a wrong isotope pattern.
fn formula_mass_warning(entry: &TargetEntry) -> Option<ValidationError> {
    let from_formula = mass(&parse_formula(&entry.neutral_formula).ok()?);
    ((entry.monoisotopic_mass - from_formula).abs() > FORMULA_MASS_TOLERANCE).then(|| {
        ValidationError {
            entry_id: entry.id.clone(),
            message: format!(
                "Monoisotopic mass {} does not match Neutral formula {} (theoretical {:.6})",
                entry.monoisotopic_mass, entry.neutral_formula, from_formula
            ),
        }
    })
}

/// Inspect target entries for structural presence, mass coherence, and review flags.
pub fn validate_entries(entries: &[TargetEntry]) -> ValidationReport {
    let total_entries = entries.len();
    let mut valid_entries = 0;
    let mut salts_or_hydrates = 0;
    let mut stereo_to_verify = 0;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for entry in entries {
        let mut entry_errors = Vec::new();

        if entry.id.trim().is_empty() {
            entry_errors.push("Empty target identifier".to_string());
        }
        if entry.neutral_formula.trim().is_empty() {
            entry_errors.push("Missing required Neutral formula".to_string());
        }
        if !entry.monoisotopic_mass.is_finite() || entry.monoisotopic_mass <= 0.0 {
            entry_errors.push(format!(
                "Invalid monoisotopic mass {} <= 0",
                entry.monoisotopic_mass
            ));
        }

        // Theoretical mass coherence checks: each adduct m/z is the monoisotopic mass plus the adduct delta, +/- 0.001
        let given = [
            (AdductType::MH, "[M+H]+", entry.mz_mh),
            (AdductType::MNa, "[M+Na]+", entry.mz_mna),
            (AdductType::MNH4, "[M+NH4]+", entry.mz_mnh4),
        ];
        for (adduct, label, got) in given {
            let expected = entry.monoisotopic_mass + adduct.delta_mass();
            let gap = (got - expected).abs();
            if gap.is_nan() || gap > 0.001 {
                entry_errors.push(format!(
                    "Incoherent {label} m/z: got {got}, expected theoretical {expected}"
                ));
            }
        }
        warnings.extend(formula_mass_warning(entry));

        if entry.check_flags.is_salt_or_hydrate {
            salts_or_hydrates += 1;
        }
        if entry.check_flags.stereo_to_verify {
            stereo_to_verify += 1;
        }

        if entry_errors.is_empty() {
            valid_entries += 1;
        } else {
            for msg in entry_errors {
                errors.push(ValidationError {
                    entry_id: entry.id.clone(),
                    message: msg,
                });
            }
        }
    }

    ValidationReport {
        total_entries,
        valid_entries,
        salts_or_hydrates,
        stereo_to_verify,
        errors,
        warnings,
    }
}
