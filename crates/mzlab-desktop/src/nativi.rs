//! The samples of a maker's file (`.raw`, `.wiff` + `.wiff.scan`, a `.D` folder) read as they are, no mzML written: the native twin of
//! `mzlab-wasm/src/nativi.rs`. A dataset is a `FileSet` of memory-mapped members opened with the readers of the registry (`lettori::apri`); a slot is
//! sample `c` of a dataset, and the routes that read the file besides its index read the `guscio` of the sample (the mzML without its spectra) and the
//! reader. The registered file of such a sample is an entry of the store with `nat` (`store.rs`).

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use mzlab_core::idxread::ReadAt;
use mzlab_core::json::Out;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::{lettori_centroidi, lettori_mzml, Error, Result};

use crate::mapped::{Held, Mapped};
use crate::rpc::MapAt;
use crate::store::Nat;

/// Readers kept open at once (a `.wiff` holds its container in memory).
const OPEN: usize = 4;
/// Bytes of `guscio` kept at once.
const SHELLS: usize = 32 << 20;

/// The members of a dataset mapped from the disk.
pub(crate) fn fileset(members: &[(String, PathBuf)]) -> Result<FileSet> {
    let mut out = Vec::new();
    for (nome, path) in members {
        let map = Mapped::open(path)
            .map_err(|e| Error::new("err.file.raw").with("detail", e.to_string()))?;
        let dim = map.bytes().len() as u64;
        out.push(Membro {
            nome: nome.clone(),
            dim,
            dati: Box::new(MapAt(Arc::new(map))),
        });
    }
    Ok(FileSet { membri: out })
}

/// The reader of a dataset; with `centroid` its profile scans become centroids.
pub(crate) fn open(members: &[(String, PathBuf)], centroid: bool) -> Result<Box<dyn Lettore>> {
    let l = lettori::apri(fileset(members)?, &Limiti::desktop())?;
    Ok(if centroid {
        lettori_centroidi::centroidato(l)
    } else {
        l
    })
}

/// The samples of a dataset: JSON `{"format": id, "samples": [{"id", "name", "scans"}]}`.
pub(crate) fn samples(members: &[(String, PathBuf)]) -> Result<String> {
    let l = open(members, false)?;
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(&format!("{:?}", l.formato()))
        .raw(",\"samples\":[");
    for (i, s) in l.campioni().iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(i64::from(s.id))
            .raw(",\"name\":")
            .str(&s.nome)
            .raw(",\"scans\":")
            .int(s.scansioni as i64)
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

struct Dataset {
    members: Vec<(String, PathBuf)>,
    centroid: bool,
    l: Option<Held>,
    used: u64,
}

/// The datasets of the session and which sample of which dataset each slot is.
#[derive(Default)]
pub struct Nativi {
    sets: HashMap<String, Dataset>,
    slots: HashMap<u32, (String, usize)>,
    shells: HashMap<(String, usize), Arc<Vec<u8>>>,
    order: VecDeque<(String, usize)>,
    clock: u64,
}

/// What the routes read for a slot: the mzML of a file of the disk, or the `guscio` of a sample with its reader (`ReadAt::lettore`).
pub(crate) enum Src<'a> {
    Map(MapAt),
    Nat {
        g: Arc<Vec<u8>>,
        l: &'a mut dyn Lettore,
        c: usize,
    },
}

impl ReadAt for Src<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        match self {
            Src::Map(m) => m.read_at(off, buf),
            Src::Nat { g, .. } => {
                let end = (off as usize).checked_add(buf.len());
                let s = end
                    .and_then(|e| g.get(off as usize..e))
                    .ok_or_else(|| Error::new("error.badIndex"))?;
                buf.copy_from_slice(s);
                Ok(())
            }
        }
    }

    fn lettore(&mut self) -> Option<(&mut dyn Lettore, usize)> {
        match self {
            Src::Map(_) => None,
            Src::Nat { l, c, .. } => Some((&mut **l, *c)),
        }
    }
}

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
