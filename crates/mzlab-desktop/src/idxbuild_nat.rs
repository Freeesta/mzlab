//! The index of a native sample (a maker's file read as it is): built straight from its reader, version 2 (see `natives.rs`).
use mzlab_core::mzidx::Stats;
use mzlab_core::readers::Limits;
use mzlab_core::{readers_centroids, upload, Error};

use crate::idxbuild::IdxBuilder;
use crate::natives;
use crate::store::Nat;

impl IdxBuilder {
    /// Builds the index of sample `nat.c` of a maker's file straight from its reader (version 2: the rows say where each scan lies in the members) as `<name>.mzidx`.
    pub fn build_nat(&self, nat: &Nat, name: &str) -> Result<Stats, Error> {
        self.write(name, |out, spill, _| {
            // the fingerprints are taken by a second thread (its own mapping of the members) while the scans are decoded; the members this one opens say
            // they have a SHA-1 already (empty): the index never reads it
            let mut fs = natives::fileset(&nat.members)?;
            upload::without_sha1(&mut fs);
            let l = mzlab_lettori::apri(fs, &Limits::desktop())?;
            let mut l = if nat.centroid {
                readers_centroids::centroidato(l)
            } else {
                l
            };
            // an index that would not fit the budget of this machine is refused in order before a scan is decoded (`error.memoryBudget`)
            let cap = mzlab_core::budget::MemBudget::mb(crate::ram::native_budget_mb());
            let peaks = mzlab_core::budget_auto::native_index_budget();
            mzlab_core::estimate::check_reader(&cap, &mut *l, nat.c, peaks, 1)?;
            let members = nat.members.clone();
            upload::with_parallel_prints(
                move || natives::fileset(&members),
                |impronte| {
                    let (mut st, _) = upload::index_reader_with(
                        &mut *l,
                        nat.c,
                        impronte,
                        name,
                        out,
                        peaks,
                        Some(spill),
                        &mut |_| {},
                    )?;
                    // the spectra the file declares beyond the ones read (`Reader::losses`)
                    st.missing = l.losses(nat.c)?.lost();
                    Ok(st)
                },
            )
        })
    }
}
