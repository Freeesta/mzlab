//! The index of a native sample (a maker's file read as it is): built straight from its reader, version 2 (see `nativi.rs`).
use mzlab_core::lettori::Limiti;
use mzlab_core::mzidx::{Stats, DEFAULT_BUDGET};
use mzlab_core::{lettori_centroidi, upload, Error};

use crate::idxbuild::IdxBuilder;
use crate::nativi;
use crate::store::Nat;

impl IdxBuilder {
    /// Builds the index of sample `nat.c` of a maker's file straight from its reader (version 2: the rows say where each scan lies in the members) as `<name>.mzidx`.
    pub fn build_nat(&self, nat: &Nat, name: &str) -> Result<Stats, Error> {
        self.scrivi(name, |out, spill| {
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
            let members = nat.members.clone();
            upload::con_impronte_parallele(
                move || nativi::fileset(&members),
                |impronte| {
                    upload::indicizza_lettore_con(
                        &mut *l,
                        nat.c,
                        impronte,
                        name,
                        out,
                        DEFAULT_BUDGET,
                        Some(spill),
                        &mut |_| {},
                    )
                    .map(|(st, _)| st)
                },
            )
        })
    }
}
