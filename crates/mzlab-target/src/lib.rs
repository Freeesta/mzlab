//! Target and suspect screening library for HRMS (high-resolution mass spectrometry).
//!
//! Provides RFC 4180 CSV parsing, structural and mass validation, and fast O(log N)
//! binary search indexed by m/z across multiple adduct types ([M+H]+, [M+Na]+, [M+NH4]+).

pub mod csv;
pub mod entry;
pub mod index;
pub mod search;
pub mod validate;

pub use csv::{parse_rfc4180_str, parse_target_csv, ParseError};
pub use entry::{AdductType, CheckFlags, TargetEntry};
pub use index::TargetList;
pub use search::{search_targets, TargetMatch, TargetSearchOptions};
pub use validate::{validate_entries, ValidationError, ValidationReport};
