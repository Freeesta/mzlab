//! The record `trasf/1` (docs/studi/STRUTTURA_DATO_TRASFORMAZIONI.md): a parent → product transformation with the atom map, written on the canonical
//! indices of two `mol/1` records that it names by their core hash and never copies. A certain product has a complete map; an uncertain one (only
//! the change is known) has `delta`, `sites` and a region and no product. Chains, remapping and the stereo rule are in `trasf_chain.rs`.

use crate::codec::MolError;
use crate::delta;
use crate::model::{Mol, RegionKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The value of the `format` field.
pub const TRASF_FORMAT: &str = "trasf/1";

impl Trasf {
    /// Reads and validates a record.
    pub fn from_json(text: &str) -> Result<Trasf, MolError> {
        let t: Trasf = serde_json::from_str(text).map_err(|e| MolError::Json(e.to_string()))?;
        t.validate()?;
        Ok(t)
    }

    /// The compact, deterministic JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Whether the product is known (a complete map) and not only the change.
    pub fn is_certain(&self) -> bool {
        self.product.is_some()
    }

    /// Rules of the format that need no molecule: the shape of the map and of the lists, hashes, delta, probabilities.
    pub fn validate(&self) -> Result<(), MolError> {
        if self.format != TRASF_FORMAT {
            return invalid(format!("format `{}`", self.format));
        }
        if !is_hash(&self.parent) || self.product.as_deref().is_some_and(|p| !is_hash(p)) {
            return invalid("parent and product are 64 hex digits");
        }
        delta::parse(&self.delta)?;
        if self.level.as_deref().is_some_and(str::is_empty) {
            return invalid("level is absent or not empty");
        }
        if self.is_certain() {
            self.validate_certain()
        } else {
            self.validate_uncertain()
        }
    }

    fn validate_certain(&self) -> Result<(), MolError> {
        if !self.sites.is_empty() || self.region.is_some() {
            return invalid("a certain product has no sites and no region");
        }
        let n = self.map.len();
        if self.map.iter().enumerate().any(|(i, m)| m.0 as usize != i) {
            return invalid("map lists every atom of the parent once, in order");
        }
        let hit: Vec<u32> = self.map.iter().filter_map(|m| m.1).collect();
        let distinct: BTreeSet<u32> = hit.iter().copied().collect();
        if distinct.len() != hit.len() {
            return invalid("two parent atoms map to one product atom");
        }
        let size = hit.len() + self.added.len();
        if !ascending(&self.added, size) || self.added.iter().any(|a| distinct.contains(a)) {
            return invalid("added: ascending product atoms without a parent atom");
        }
        if hit.iter().any(|&h| h as usize >= size) {
            return invalid("map points outside the product");
        }
        if !ascending(&self.centre, n) {
            return invalid("centre: ascending parent atoms");
        }
        Ok(())
    }

    fn validate_uncertain(&self) -> Result<(), MolError> {
        if !self.map.is_empty() || !self.added.is_empty() || !self.centre.is_empty() {
            return invalid("without a product there is no map, added or centre");
        }
        if self.sites.is_empty() {
            return invalid("without a product the change needs sites");
        }
        let atoms: BTreeSet<u32> = self.sites.iter().map(|s| s.atom).collect();
        if atoms.len() != self.sites.len() || self.sites.iter().any(|s| !(s.p > 0.0 && s.p <= 1.0))
        {
            return invalid("sites: distinct atoms, p in (0, 1]");
        }
        if let Some(r) = &self.region {
            if !ascending(&r.atoms, usize::MAX) || !atoms.iter().all(|a| r.atoms.contains(a)) {
                return invalid("region: ascending atoms that include every site");
            }
        }
        Ok(())
    }

    /// The record against the two molecules it names: the hashes, the sizes, the elements (equal except at the centre) and the stereo rule.
    pub fn validate_against(&self, parent: &Mol, product: Option<&Mol>) -> Result<(), MolError> {
        self.validate()?;
        if self.parent != parent.core_hash_hex() {
            return invalid("parent hash is not the one of the parent record");
        }
        let n = parent.atoms.len();
        if self.centre.iter().any(|&c| c as usize >= n)
            || self.sites.iter().any(|s| s.atom as usize >= n)
            || self
                .region
                .as_ref()
                .is_some_and(|r| r.atoms.iter().any(|&a| a as usize >= n))
        {
            return invalid("an atom outside the parent");
        }
        match (&self.product, product) {
            (None, None) => Ok(()),
            (Some(h), Some(p)) if *h == p.core_hash_hex() => self.check_pairs(parent, p),
            _ => invalid("product hash is not the one of the product record"),
        }
    }

    fn check_pairs(&self, parent: &Mol, product: &Mol) -> Result<(), MolError> {
        if self.map.len() != parent.atoms.len()
            || self.map.iter().filter(|m| m.1.is_some()).count() + self.added.len()
                != product.atoms.len()
        {
            return invalid("map and added do not cover the two records");
        }
        for &(a, b) in &self.map {
            let Some(b) = b else { continue };
            if parent.atoms[a as usize].el != product.atoms[b as usize].el
                && !self.centre.contains(&a)
            {
                return invalid(format!("atom {a} changes element outside the centre"));
            }
        }
        self.check_stereo(parent, product)
    }

    /// A parent stereo element at the centre must not come out `defined` in the product unless `stereo_rule` says retention or inversion.
    fn check_stereo(&self, parent: &Mol, product: &Mol) -> Result<(), MolError> {
        if matches!(self.stereo_rule.as_deref(), Some("retention" | "inversion")) {
            return Ok(());
        }
        for k in self.touched_stereo(parent) {
            let image: Option<Vec<u32>> = parent.stereo[k]
                .key_atoms()
                .iter()
                .map(|&a| self.product_atom(a))
                .collect();
            let Some(mut image) = image else { continue };
            image.sort_unstable();
            for s in product.stereo.iter().filter(|s| s.is_defined()) {
                let mut key = s.key_atoms();
                key.sort_unstable();
                if key == image {
                    return invalid("a stereo element at the centre is copied as defined");
                }
            }
        }
        Ok(())
    }
}

/// One entry of `sites`: a parent atom where the change may sit, with its support.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TSite {
    pub atom: u32,
    pub p: f64,
}

/// The part of the parent an uncertain product is spread over (the `regions` of `mol/1`, here with the atoms of the parent).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TRegion {
    pub atoms: Vec<u32>,
    pub kind: RegionKind,
}

/// A piece of MS/MS evidence tied to atoms of the product (`on` = "product") or of the parent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mz: Option<f64>,
    pub atoms: Vec<u32>,
    pub on: String,
}

/// The record. `map` is `[parent atom, product atom or null if lost]`; field order is the order of the JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trasf {
    pub format: String,
    pub parent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    pub delta: String,
    #[serde(default)]
    pub rule: String,
    #[serde(default)]
    pub rule_set: String,
    #[serde(default)]
    pub rule_version: String,
    #[serde(default)]
    pub map: Vec<(u32, Option<u32>)>,
    #[serde(default)]
    pub added: Vec<u32>,
    #[serde(default)]
    pub centre: Vec<u32>,
    #[serde(default)]
    pub sites: Vec<TSite>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<TRegion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereo_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

fn invalid<T>(what: impl Into<String>) -> Result<T, MolError> {
    Err(MolError::Invalid(format!("trasf/1: {}", what.into())))
}

fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Strictly ascending and below `limit`.
fn ascending(list: &[u32], limit: usize) -> bool {
    list.windows(2).all(|w| w[0] < w[1]) && list.iter().all(|&x| (x as usize) < limit)
}
