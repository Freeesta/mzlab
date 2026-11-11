//! The samples of a maker's file (`.raw`, `.wiff` + `.wiff.scan`, a `.D` folder) read as they are, no mzML written: the native twin of
//! `mzlab-wasm/src/natives.rs`. A dataset is a `FileSet` of memory-mapped members opened with the readers of the registry (`readers::apri`); a slot is
//! sample `c` of a dataset, and the routes that read the file besides its index read the `guscio` of the sample (the mzML without its spectra) and the
//! reader. The registered file of such a sample is an entry of the store with `nat` (`store.rs`).

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{FileSet, Limits, Member, Reader};
use mzlab_core::{readers_centroids, Error, Result};

use crate::mapped::{Held, Mapped};
use crate::rpc::MapAt;

/// Readers kept open at once (a `.wiff` holds its container in memory).
pub(super) const OPEN: usize = 4;
/// Bytes of `guscio` kept at once.
pub(super) const SHELLS: usize = 32 << 20;

/// The members of a dataset mapped from the disk.
pub(crate) fn fileset(members: &[(String, PathBuf)]) -> Result<FileSet> {
    let mut out = Vec::new();
    for (name, path) in members {
        let map = Mapped::open(path)
            .map_err(|e| Error::new("err.file.raw").with("detail", e.to_string()))?;
        let dim = map.bytes().len() as u64;
        out.push(Member {
            nome: name.clone(),
            dim,
            dati: Box::new(MapAt(Arc::new(map))),
            sha1: None,
        });
    }
    Ok(FileSet { membri: out })
}

/// The reader of a dataset; with `centroid` its profile scans become centroids.
pub(crate) fn open(members: &[(String, PathBuf)], centroid: bool) -> Result<Box<dyn Reader>> {
    let l = mzlab_lettori::apri(fileset(members)?, &Limits::desktop())?;
    Ok(if centroid {
        readers_centroids::centroidato(l)
    } else {
        l
    })
}

/// The samples of a dataset: the JSON of `readers::samples_json`, the listing the site gives too.
pub(crate) fn samples(members: &[(String, PathBuf)]) -> Result<String> {
    let mut l = open(members, false)?;
    Ok(mzlab_core::readers::samples_json(&mut *l))
}

pub(super) struct Dataset {
    pub(super) members: Vec<(String, PathBuf)>,
    pub(super) centroid: bool,
    pub(super) l: Option<Held>,
    pub(super) used: u64,
}

/// The datasets of the session and which sample of which dataset each slot is.
#[derive(Default)]
pub struct Nativi {
    pub(super) sets: HashMap<String, Dataset>,
    pub(super) slots: HashMap<u32, (String, usize)>,
    pub(super) shells: HashMap<(String, usize), Arc<Vec<u8>>>,
    pub(super) order: VecDeque<(String, usize)>,
    pub(super) clock: u64,
}

/// What the routes read for a slot: the mzML of a file of the disk, or the `guscio` of a sample with its reader (`ReadAt::lettore`).
pub(crate) enum Src<'a> {
    Map(MapAt),
    Nat {
        g: Arc<Vec<u8>>,
        l: &'a mut dyn Reader,
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

    fn reader(&mut self) -> Option<(&mut dyn Reader, usize)> {
        match self {
            Src::Map(_) => None,
            Src::Nat { l, c, .. } => Some((&mut **l, *c)),
        }
    }
}
