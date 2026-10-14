//! The index of a native sample (a maker's file read as it is): built straight from its reader, version 2 (see `nativi.rs`).
use std::fs::File;

use mzlab_core::lettori::{self, Limiti};
use mzlab_core::mzidx::{self, Stats, DEFAULT_BUDGET};
use mzlab_core::{lettori_centroidi, upload, Error};

use crate::idxbuild::{clean, io_error, IdxBuilder};
use crate::nativi;
use crate::store::Nat;

impl IdxBuilder {
    /// Builds the index of sample `nat.c` of a maker's file straight from its reader (version 2: the rows say where each scan lies in the members) as `<name>.mzidx`.
    pub fn build_nat(&self, nat: &Nat, name: &str) -> Result<Stats, Error> {
        let dest = self.path(name)?;
        let part = self.dir.join(format!("{}.mzidx.part", clean(name)?));
        let r = self.pool.install(|| -> Result<Stats, Error> {
            let mut fs = nativi::fileset(&nat.members)?;
            let impronte = upload::impronte(&mut fs)?;
            let l = lettori::apri(fs, &Limiti::desktop())?;
            let mut l = if nat.centroid {
                lettori_centroidi::centroidato(l)
            } else {
                l
            };
            let p = upload::prepare_lettore(&mut *l, nat.c, &impronte, name)?;
            let mut out = File::create(&part).map_err(|e| io_error(&e))?;
            mzidx::build_lettore(&p.table, &mut *l, nat.c, &mut out, DEFAULT_BUDGET)
        });
        match r {
            Ok(st) => {
                std::fs::rename(&part, &dest).map_err(|e| io_error(&e))?;
                Ok(st)
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                Err(e)
            }
        }
    }
}
