//! Target and suspect screening library for HRMS (high-resolution mass spectrometry).
//!
//! Provides RFC 4180 CSV parsing, structural and mass validation, and fast O(log N)
//! binary search indexed by m/z across multiple adduct types ([M+H]+, [M+Na]+, [M+NH4]+).

pub mod cerca;
pub mod csv;
pub mod entry;
pub mod indice;
pub mod valida;

pub use cerca::{cerca_targets, TargetMatch, TargetSearchOptions};
pub use csv::{parse_rfc4180_str, parse_target_csv, ParseError};
pub use entry::{AdductType, CheckFlags, TargetEntry};
pub use indice::TargetList;
pub use valida::{validate_entries, ValidationError, ValidationReport};
