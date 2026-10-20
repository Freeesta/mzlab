//! What a `trasf/1` record can be asked after it exists: which stereo elements its centre touches, where an atom goes and comes from, the
//! composition of two steps of a chain (genitore → TP1 → TP2) and the rewriting of the map on the canonical order of the two records.

use crate::codec::MolError;
use crate::delta;
use crate::model::{Mol, Status, Stereo};
use crate::trasf::{Evidence, TRegion, TSite, Trasf};
use std::collections::BTreeSet;

fn invalid<T>(what: impl Into<String>) -> Result<T, MolError> {
    Err(MolError::Invalid(format!("trasf/1: {}", what.into())))
}

impl Stereo {
    /// The atoms that name the element: the centre of a tetrahedral one, both ends of a double bond.
    pub(crate) fn key_atoms(&self) -> Vec<u32> {
        match self {
            Stereo::Tetra { c, .. } => vec![*c],
            Stereo::Db { a, b, .. } => vec![*a, *b],
        }
    }

    pub(crate) fn is_defined(&self) -> bool {
        let status = match self {
            Stereo::Tetra { status, .. } | Stereo::Db { status, .. } => status,
        };
        *status == Status::Defined
    }
}

impl Trasf {
    /// The product atom a parent atom becomes (`None` if it is lost, or if the product is not known).
    pub fn product_atom(&self, parent_atom: u32) -> Option<u32> {
        self.map.get(parent_atom as usize).and_then(|m| m.1)
    }

    /// The parent atom a product atom comes from (`None` for an added atom).
    pub fn parent_atom(&self, product_atom: u32) -> Option<u32> {
        self.map
            .iter()
            .find(|m| m.1 == Some(product_atom))
            .map(|m| m.0)
    }

    /// Indices (into `parent.stereo`) of the stereo elements whose atoms are at the reaction centre: their configuration is not carried over.
    pub fn touched_stereo(&self, parent: &Mol) -> Vec<usize> {
        parent
            .stereo
            .iter()
            .enumerate()
            .filter(|(_, s)| s.key_atoms().iter().any(|a| self.centre.contains(a)))
            .map(|(k, _)| k)
            .collect()
    }

    /// The step after this one, as one step from the parent of `self` to the product of `next`. Both must have a product; an atom lost in either
    /// step is lost. Evidence is kept where it still names the same atoms (parent evidence of `self`, product evidence of `next`).
    pub fn compose(&self, next: &Trasf) -> Result<Trasf, MolError> {
        if self.product.is_none() || next.product.is_none() {
            return invalid("a chain is composed from certain products only");
        }
        if self.product.as_deref() != Some(next.parent.as_str()) {
            return invalid("the product of the first step is not the parent of the second");
        }
        let middle = self.map.iter().filter(|m| m.1.is_some()).count() + self.added.len();
        if middle != next.map.len() {
            return invalid("the two steps disagree on the size of the intermediate");
        }
        let map: Vec<(u32, Option<u32>)> = self
            .map
            .iter()
            .map(|&(a, b)| (a, b.and_then(|b| next.product_atom(b))))
            .collect();
        let size = next.map.iter().filter(|m| m.1.is_some()).count() + next.added.len();
        let hit: BTreeSet<u32> = map.iter().filter_map(|m| m.1).collect();
        let added: Vec<u32> = (0..size as u32).filter(|x| !hit.contains(x)).collect();
        let centre: Vec<u32> = (0..self.map.len() as u32)
            .filter(|a| {
                self.centre.contains(a)
                    || self
                        .product_atom(*a)
                        .is_some_and(|b| next.centre.contains(&b))
            })
            .collect();
        let both = |a: &str, b: &str| match (a.is_empty(), b.is_empty(), a == b) {
            (_, _, true) | (_, true, _) => a.to_string(),
            (true, _, _) => b.to_string(),
            _ => format!("{a} > {b}"),
        };
        let mut evidence: Vec<Evidence> = self
            .evidence
            .iter()
            .filter(|e| e.on == "parent")
            .cloned()
            .collect();
        evidence.extend(next.evidence.iter().filter(|e| e.on == "product").cloned());
        let out = Trasf {
            format: self.format.clone(),
            parent: self.parent.clone(),
            product: next.product.clone(),
            delta: delta::add(&self.delta, &next.delta)?,
            rule: both(&self.rule, &next.rule),
            rule_set: both(&self.rule_set, &next.rule_set),
            rule_version: both(&self.rule_version, &next.rule_version),
            map,
            added,
            centre,
            sites: Vec::new(),
            region: None,
            stereo_rule: self
                .stereo_rule
                .clone()
                .filter(|r| next.stereo_rule.as_ref() == Some(r)),
            level: next.level.clone(),
            evidence,
        };
        out.validate()?;
        Ok(out)
    }

    /// The same transformation written on the canonical order of its two records, with the hashes of the renumbered records.
    pub fn canonical(&self, parent: &Mol, product: Option<&Mol>) -> Result<Trasf, MolError> {
        self.validate_against(parent, product)?;
        let po = parent.canonical_order();
        let qo = product.map(Mol::canonical_order);
        let p = |a: u32| po[a as usize];
        let q = |b: u32| qo.as_ref().map_or(b, |o| o[b as usize]);
        let sorted = |mut v: Vec<u32>| {
            v.sort_unstable();
            v
        };
        let on = |e: &Evidence| {
            let f: &dyn Fn(u32) -> u32 = if e.on == "product" { &q } else { &p };
            sorted(e.atoms.iter().map(|&a| f(a)).collect())
        };
        let mut map: Vec<(u32, Option<u32>)> =
            self.map.iter().map(|m| (p(m.0), m.1.map(q))).collect();
        map.sort_unstable_by_key(|m| m.0);
        let mut out = Trasf {
            parent: parent.renumbered(&po)?.core_hash_hex(),
            product: match (product, &qo) {
                (Some(m), Some(o)) => Some(m.renumbered(o)?.core_hash_hex()),
                _ => None,
            },
            map,
            added: sorted(self.added.iter().map(|&b| q(b)).collect()),
            centre: sorted(self.centre.iter().map(|&a| p(a)).collect()),
            sites: self
                .sites
                .iter()
                .map(|s| TSite {
                    atom: p(s.atom),
                    p: s.p,
                })
                .collect(),
            region: self.region.as_ref().map(|r| TRegion {
                atoms: sorted(r.atoms.iter().map(|&a| p(a)).collect()),
                kind: r.kind,
            }),
            ..self.clone()
        };
        out.sites.sort_by_key(|s| s.atom);
        out.evidence = self
            .evidence
            .iter()
            .map(|e| Evidence {
                atoms: on(e),
                ..e.clone()
            })
            .collect();
        out.validate()?;
        Ok(out)
    }
}
