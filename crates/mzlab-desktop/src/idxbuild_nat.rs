//! The index of a native sample (a maker's file read as it is): built straight from its reader, version 2 (see `nativi.rs`).
use std::fs::File;

use mzlab_core::lettori::Limiti;
use mzlab_core::mzidx::{Stats, DEFAULT_BUDGET};
use mzlab_core::{lettori_centroidi, upload, Error};

use crate::idxbuild::{clean, io_error, IdxBuilder};
use crate::nativi;
use crate::store::Nat;

impl IdxBuilder {
    /// Builds the index of sample `nat.c` of a maker's file straight from its reader (version 2: the rows say where each scan lies in the members) as `<name>.mzidx`.
    pub fn build_nat(&self, nat: &Nat, name: &str) -> Result<Stats, Error> {
        let dest = self.path(name)?;
        let part = self.dir.join(format!("{}.mzidx.part", clean(name)?));
        let spill_path = self.dir.join(format!("{}.mzidx.sp.part", clean(name)?));
        let r = self.pool.install(|| -> Result<Stats, Error> {
            // the fingerprints are taken by a second thread (its own mapping of the members) while the scans are decoded; the members this one opens say
            // they have a SHA-1 already (empty): the index never reads it
            let mut fs = nativi::fileset(&nat.members)?;
            upload::senza_sha1(&mut fs);
            let l = mzlab_lettori::apri(fs, &Limiti::desktop())?;
            let mut l = if nat.centroid {
                lettori_centroidi::centroidato(l)
            } else {
                l
            };
            let mut out = File::create(&part).map_err(|e| io_error(&e))?;
            // the peaks that do not fit in the budget wait in a temporary file: the scans are decoded once
            let mut spill = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&spill_path)
                .map_err(|e| io_error(&e))?;
            let members = nat.members.clone();
            let r = upload::con_impronte_parallele(
                move || nativi::fileset(&members),
                |impronte| {
                    upload::indicizza_lettore_con(
                        &mut *l,
                        nat.c,
                        impronte,
                        name,
                        &mut out,
                        DEFAULT_BUDGET,
                        Some(&mut spill),
                        &mut |_| {},
                    )
                    .map(|(st, _)| st)
                },
            );
            drop(spill);
            let _ = std::fs::remove_file(&spill_path);
            r
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
