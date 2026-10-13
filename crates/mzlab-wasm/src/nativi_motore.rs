// The methods of `Nativi` (the slots read straight from the files of a maker) and of `Engine` that attach a slot to its reader (see nativi.rs).
use std::rc::Rc;

use js_sys::Function;
use mzlab_core::lettori_mzml;
use wasm_bindgen::prelude::*;

use crate::nativi::{reader, Dataset, Membri, Nativi, Src, SHELLS};
use crate::{error_json, Engine, JsReader};
use mzlab_core::mzmlhead::Mzml;

impl Nativi {
    pub(crate) fn forget(&mut self, slot: u32) {
        if let Some((ds, _)) = self.slots.remove(&slot) {
            if !self.slots.values().any(|(d, _)| *d == ds) {
                self.sets.remove(&ds);
                self.shells.retain(|(d, _), _| *d != ds);
                self.order.retain(|(d, _)| *d != ds);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Nativi::default();
    }

    /// What the routes read for `slot`: the `guscio` and the reader of its sample when it is native, else the mzML through `read`; with its size.
    pub(crate) fn src(
        &mut self,
        slot: u32,
        read: Function,
        size: f64,
    ) -> Result<(Src<'_>, u64), JsValue> {
        let Some((ds, c)) = self.slots.get(&slot).cloned() else {
            return Ok((Src::Js(JsReader { read }), size as u64));
        };
        let key = (ds.clone(), c);
        let g = match self.shells.get(&key) {
            Some(g) => g.clone(),
            None => {
                let l = reader(self, ds.clone())?;
                let g = Rc::new(lettori_mzml::guscio(l, c).map_err(|e| error_json(&e))?);
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
        let l = reader(self, ds.clone())?;
        Ok((Src::Nat { g, l, c }, len))
    }

    /// The head of the mzML of `slot` (`mzmlhead::Mzml`) over `src`.
    pub(crate) fn mzml(
        &mut self,
        slot: u32,
        read: Function,
        size: f64,
    ) -> Result<Mzml<Src<'_>>, JsValue> {
        let (s, len) = self.src(slot, read, size)?;
        Ok(Mzml::new(s, len))
    }
}

#[wasm_bindgen]
impl Engine {
    /// Makes `slot` sample `c` of the dataset `ds` (`names`, `sizes`, `read(member, offset, length)`): the routes that read the file besides its index
    /// (`open_index`) read it through its reader. `centroid`: the profile scans become centroids (the same value the index was built with).
    #[allow(clippy::too_many_arguments)]
    pub fn native_attach(
        &mut self,
        slot: u32,
        ds: &str,
        names: js_sys::Array,
        sizes: &[f64],
        read: Function,
        c: u32,
        centroid: bool,
    ) -> Result<(), JsValue> {
        if self.nat.slots.get(&slot) == Some(&(ds.to_string(), c as usize))
            && self.nat.sets.contains_key(ds)
        {
            return Ok(()); // already attached (the page opens the index of a file again for each route)
        }
        self.nat.forget(slot);
        if !self.nat.sets.contains_key(ds) {
            let membri = Membri::new(names, sizes, read)?;
            self.nat.sets.insert(
                ds.to_string(),
                Dataset {
                    membri,
                    centroid,
                    l: None,
                    used: 0,
                },
            );
        }
        self.nat.slots.insert(slot, (ds.to_string(), c as usize));
        Ok(())
    }

    /// Forgets the dataset of `slot` (with the index: `close`).
    pub fn native_detach(&mut self, slot: u32) {
        self.nat.forget(slot);
    }

    /// `metodo/1` of the sample of `slot` from its reader; `""` when `slot` is not native.
    pub fn native_metodo(&mut self, slot: u32) -> Result<String, JsValue> {
        let Some((ds, c)) = self.nat.slots.get(&slot).cloned() else {
            return Ok(String::new());
        };
        let l = reader(&mut self.nat, ds)?;
        l.metodo(c).map_err(|e| error_json(&e))
    }
}
