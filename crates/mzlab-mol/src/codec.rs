//! JSON (de)serialisation of `mol/1` and the core hash. Output is compact and deterministic: struct field order, no map types.

use crate::model::{Atom, Bond, Mol, Stereo, StereoGroup};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;

/// Why a record could not be read or is not valid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MolError {
    /// The text is not JSON of the shape of `mol/1`.
    Json(String),
    /// The record breaks a rule of the format (message names the field).
    Invalid(String),
    /// The text is not a SMILES (or CXSMILES) the reader understands, or the record cannot be written as one.
    Smiles(String),
}

impl fmt::Display for MolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MolError::Json(m) => write!(f, "mol/1 JSON: {m}"),
            MolError::Invalid(m) => write!(f, "mol/1 invalid: {m}"),
            MolError::Smiles(m) => write!(f, "SMILES: {m}"),
        }
    }
}

impl std::error::Error for MolError {}

#[derive(Serialize)]
struct Core<'a> {
    atoms: &'a [Atom],
    bonds: &'a [Bond],
    stereo: &'a [Stereo],
    groups: &'a [StereoGroup],
}

/// Lower-case hexadecimal of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl Mol {
    /// Reads and validates a record.
    pub fn from_json(text: &str) -> Result<Mol, MolError> {
        let m: Mol = serde_json::from_str(text).map_err(|e| MolError::Json(e.to_string()))?;
        m.validate()?;
        Ok(m)
    }

    /// The compact, deterministic JSON of the whole record.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Compact JSON of the core alone (atoms, bonds, stereo, groups, in this order): the bytes the hash covers.
    pub fn core_json(&self) -> String {
        let core = Core {
            atoms: &self.atoms,
            bonds: &self.bonds,
            stereo: &self.stereo,
            groups: &self.groups,
        };
        serde_json::to_string(&core).unwrap_or_default()
    }

    /// SHA-256 of `core_json()`. Coordinates, ids, properties and regions never change it; so far it follows the order of the atoms as given.
    pub fn core_hash(&self) -> [u8; 32] {
        Sha256::digest(self.core_json().as_bytes()).into()
    }

    /// `core_hash()` in lower-case hexadecimal.
    pub fn core_hash_hex(&self) -> String {
        hex(&self.core_hash())
    }
}
