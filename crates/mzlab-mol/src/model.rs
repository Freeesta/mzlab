//! Types of the record `mol/1`. Field order is the order of the JSON output (deterministic); optional layers are left out when empty.

use serde::{Deserialize, Serialize};

/// The value of the `format` field.
pub const FORMAT: &str = "mol/1";

/// One atom. `iso` 0 = natural abundance; `h` = implicit hydrogens on the atom (a hydrogen with an isotope is an atom of its own).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Atom {
    pub el: String,
    #[serde(default)]
    pub iso: u16,
    #[serde(default)]
    pub chg: i8,
    #[serde(default)]
    pub rad: u8,
    #[serde(default)]
    pub h: u8,
}

/// A bond in Kekulé form: `o` is 1, 2 or 3 and `a < b`. Aromaticity is never stored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub a: u32,
    pub b: u32,
    pub o: u8,
}

/// Whether a stereo element is specified. An element absent from the record means "not a stereo element at all".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Defined,
    Unknown,
}

/// Sense of the other refs seen from the first one (the implicit hydrogen or lone pair of a three-ref centre comes last).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Parity {
    Cw,
    Ccw,
}

/// Position of the two refs of a double bond relative to each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cfg {
    Cis,
    Trans,
}

/// A stereo element. `defined` ones carry refs and parity/configuration, `unknown` ones carry only their position.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Stereo {
    /// Tetrahedral centre `c`; `refs` are its neighbours in ascending index order (3 or 4).
    #[serde(rename = "tetra")]
    Tetra {
        c: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refs: Option<Vec<u32>>,
        #[serde(default, rename = "implicitH", skip_serializing_if = "Option::is_none")]
        implicit_h: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        par: Option<Parity>,
        status: Status,
    },
    /// Double bond `a`-`b` (`a < b`); `refs[0]` is a neighbour of `a`, `refs[1]` of `b`.
    #[serde(rename = "db")]
    Db {
        a: u32,
        b: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refs: Option<[u32; 2]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cfg: Option<Cfg>,
        status: Status,
    },
}

/// Enhanced-stereo group kind: `abs` absolute, `and` racemic mixture, `or` relative (one of the two).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupKind {
    Abs,
    And,
    Or,
}

/// Defined tetrahedral centres that move together (ascending indices).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StereoGroup {
    pub kind: GroupKind,
    pub atoms: Vec<u32>,
}

/// Optional coordinates, one row of `dim` numbers (x, y[, z]) per atom; never the source of the stereo after import.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coords {
    pub dim: u8,
    pub xy: Vec<Vec<f64>>,
    pub from: String,
}

/// Identity layer, filled by later packages; not part of the core hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ids {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inchikey: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skeleton: Option<String>,
}

/// A per-atom property vector (one value per atom) with its provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub name: String,
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can: Option<String>,
    pub v: Vec<f64>,
}

/// What a region of uncertainty covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionKind {
    Zone,
    Multicenter,
    Whole,
}

/// Where a change (`delta`, e.g. `+O`) may sit, without choosing one structure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub atoms: Vec<u32>,
    pub delta: String,
    pub kind: RegionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support: Option<f64>,
}

/// The record. `atoms`, `bonds`, `stereo` and `groups` are the core (hashed); the other layers are not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mol {
    pub format: String,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    #[serde(default)]
    pub stereo: Vec<Stereo>,
    #[serde(default)]
    pub groups: Vec<StereoGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coords: Option<Coords>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Ids>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub props: Vec<Prop>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
}

impl Mol {
    /// A record with the core only, in the order given (no canonical ordering yet).
    pub fn new(
        atoms: Vec<Atom>,
        bonds: Vec<Bond>,
        stereo: Vec<Stereo>,
        groups: Vec<StereoGroup>,
    ) -> Self {
        Mol {
            format: FORMAT.into(),
            atoms,
            bonds,
            stereo,
            groups,
            coords: None,
            id: None,
            props: Vec::new(),
            regions: Vec::new(),
        }
    }
}
