//! Detection of the features from the raw data after the algorithms of mzmine 3.9.0 (feature `untargeted`). One module per stage; each opens with the
//! attribution of the Java file it was rewritten from (MIT, (c) 2004-2022 The mzmine Development Team; source read at the tag `v3.9.0`; licence line in
//! `LICENZE-TERZI.md`). Here: `join` (join aligner), `gapfill` (peak finder that fills the empty cells) and `table` (the result as the `Aligned` table).

pub mod adap;
pub mod gapfill;
pub mod join;
pub mod resolve;
pub mod table;

/// One feature of one file in the units of mzmine: RT and its range in minutes, `area` in intensity x minutes (`table::to_aligned` converts it to
/// counts x seconds). `filled` marks a feature estimated by the gap filler (mzmine `ESTIMATED`) rather than detected.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Feature {
    pub mz: f64,
    pub rt: f64,
    pub rt_start: f64,
    pub rt_end: f64,
    pub height: f64,
    pub area: f64,
    pub filled: bool,
}
