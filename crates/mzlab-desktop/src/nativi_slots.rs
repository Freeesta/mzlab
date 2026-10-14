//! The slots of `Nativi`: which sample of which dataset each one is, its reader and its `guscio` (see `nativi.rs`).

use std::sync::Arc;

use mzlab_core::lettori::Lettore;
use mzlab_core::{lettori_mzml, Error, Result};

use crate::mapped::Held;
use crate::nativi::{open, Dataset, Nativi, Src, OPEN, SHELLS};
use crate::store::Nat;

impl Nativi {
    /// Makes `slot` sample `nat.c` of the dataset `nat.ds`.
    pub(crate) fn attach(&mut self, slot: u32, nat: &Nat) {
        if self.slots.get(&slot) == Some(&(nat.ds.clone(), nat.c))
            && self.sets.contains_key(&nat.ds)
        {
            return;
        }
        self.forget(slot);
        self.sets.entry(nat.ds.clone()).or_insert_with(|| Dataset {
            members: nat.members.clone(),
            centroid: nat.centroid,
            l: None,
            used: 0,
        });
        self.slots.insert(slot, (nat.ds.clone(), nat.c));
    }

    pub(crate) fn is_native(&self, slot: u32) -> bool {
        self.slots.contains_key(&slot)
    }

    /// Forgets the slot; the dataset goes with its last slot.
    pub(crate) fn forget(&mut self, slot: u32) {
        if let Some((ds, _)) = self.slots.remove(&slot) {
            if !self.slots.values().any(|(d, _)| *d == ds) {
                self.sets.remove(&ds);
                self.shells.retain(|(d, _), _| *d != ds);
                self.order.retain(|(d, _)| *d != ds);
            }
        }
    }

    /// Forgets the slots below `base` (the ones of the session; the page's list keeps its own).
    pub(crate) fn forget_below(&mut self, base: u32) {
        let gone: Vec<u32> = self.slots.keys().copied().filter(|k| *k < base).collect();
        for k in gone {
            self.forget(k);
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Nativi::default();
    }

    /// The open reader of dataset `ds` (opened again when it was closed; the least used ones are closed beyond `OPEN`).
    fn reader(&mut self, ds: &str) -> Result<&mut dyn Lettore> {
        self.clock += 1;
        let open_now = self.sets.values().filter(|d| d.l.is_some()).count();
        if open_now >= OPEN && self.sets.get(ds).is_some_and(|d| d.l.is_none()) {
            if let Some(old) = self
                .sets
                .values_mut()
                .filter(|d| d.l.is_some())
                .min_by_key(|d| d.used)
            {
                old.l = None;
            }
        }
        let d = self
            .sets
            .get_mut(ds)
            .ok_or_else(|| Error::new("error.badIndex"))?;
        d.used = self.clock;
        if d.l.is_none() {
            d.l = Some(Held(open(&d.members, d.centroid)?));
        }
        match d.l.as_mut() {
            Some(h) => Ok(&mut *h.0),
            None => Err(Error::new("error.badIndex")),
        }
    }

    /// What the routes read for `slot` and its length; None when the slot is not a native sample.
    pub(crate) fn src(&mut self, slot: u32) -> Result<Option<(Src<'_>, u64)>> {
        let Some((ds, c)) = self.slots.get(&slot).cloned() else {
            return Ok(None);
        };
        let key = (ds.clone(), c);
        let g = match self.shells.get(&key) {
            Some(g) => g.clone(),
            None => {
                let g = Arc::new(lettori_mzml::guscio(self.reader(&ds)?, c)?);
                let mut held: usize = self.shells.values().map(|g| g.len()).sum();
                while held + g.len() > SHELLS {
                    let Some(old) = self.order.pop_front() else {
                        break;
                    };
                    held -= self.shells.remove(&old).map_or(0, |g| g.len());
                }
                self.shells.insert(key.clone(), g.clone());
                self.order.push_back(key);
                g
            }
        };
        let len = g.len() as u64;
        let l = self.reader(&ds)?;
        Ok(Some((Src::Nat { g, l, c }, len)))
    }
}
