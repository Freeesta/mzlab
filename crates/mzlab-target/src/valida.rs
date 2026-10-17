// Validation logic for target screening entries and adduct mass coherence.
use serde::{Deserialize, Serialize};

use crate::entry::TargetEntry;

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
}

impl ValidationReport {
    /// True when no validation errors exist and every entry is valid.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.valid_entries == self.total_entries
    }
}

/// Inspect target entries for structural presence, mass coherence, and review flags.
pub fn validate_entries(entries: &[TargetEntry]) -> ValidationReport {
    let total_entries = entries.len();
    let mut valid_entries = 0;
    let mut salts_or_hydrates = 0;
    let mut stereo_to_verify = 0;
    let mut errors = Vec::new();

    for entry in entries {
        let mut entry_errors = Vec::new();

        if entry.id.trim().is_empty() {
            entry_errors.push("Empty target identifier".to_string());
        }
        if entry.neutral_formula.trim().is_empty() {
            entry_errors.push("Missing required Neutral formula".to_string());
        }
        if entry.monoisotopic_mass <= 0.0 {
            entry_errors.push(format!(
                "Invalid monoisotopic mass {} <= 0",
                entry.monoisotopic_mass
            ));
        }

        // Theoretical mass coherence checks:
        // [M+H]+ m/z: monoisotopic mass + 1.007276 +/- 0.001
        let expected_mh = entry.monoisotopic_mass + 1.007276;
        if (entry.mz_mh - expected_mh).abs() > 0.001 {
            entry_errors.push(format!(
                "Incoherent [M+H]+ m/z: got {}, expected theoretical {}",
                entry.mz_mh, expected_mh
            ));
        }

        // [M+Na]+ m/z: monoisotopic mass + 22.989218 +/- 0.001
        let expected_mna = entry.monoisotopic_mass + 22.989218;
        if (entry.mz_mna - expected_mna).abs() > 0.001 {
            entry_errors.push(format!(
                "Incoherent [M+Na]+ m/z: got {}, expected theoretical {}",
                entry.mz_mna, expected_mna
            ));
        }

        // [M+NH4]+ m/z: monoisotopic mass + 18.033823 +/- 0.001
        let expected_mnh4 = entry.monoisotopic_mass + 18.033823;
        if (entry.mz_mnh4 - expected_mnh4).abs() > 0.001 {
            entry_errors.push(format!(
                "Incoherent [M+NH4]+ m/z: got {}, expected theoretical {}",
                entry.mz_mnh4, expected_mnh4
            ));
        }

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
    }
}
