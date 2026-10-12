//! The recipe of a feature table: every parameter that changes the result, the version of the crate and of its rules, as text and as a fingerprint.
//! The fingerprint (FNV-1a over the text, two 64-bit lanes) names the per-file cache and tells whether a stored table is still the answer to the
//! current parameters; changing only the alignment does not change the fingerprint of the per-file part.

use crate::famalign::FamAlignParams;
use crate::family::evidence::PROOF_VERSION;
use crate::family::{FamParams, FAMILY_VERSION};
use crate::gapfill::FillParams;
use crate::pipeline::Steps;
use crate::rtmap::RtParams;
use crate::trace::Params;
use crate::FEAT_VERSION;

/// How the groups of the table are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One row per ion (`align::align`).
    Ion,
    /// One row per family of ions (`famalign::align_families`).
    Family,
}

/// All the parameters of a table. `detect.budget` only sizes the slabs and is not part of the text.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub detect: Params,
    pub steps: Steps,
    pub fam: FamParams,
    pub mode: Mode,
    pub align: FamAlignParams,
    pub rt: RtParams,
    pub fill: FillParams,
    /// Fill the gaps of the aligned table with the integration of the raw traces (L6).
    pub fill_gaps: bool,
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe {
            detect: Params::default(),
            steps: Steps::default(),
            fam: FamParams::default(),
            mode: Mode::Family,
            align: FamAlignParams::default(),
            rt: RtParams::default(),
            fill: FillParams::default(),
            fill_gaps: true,
        }
    }
}

/// Version of the crate and of the rules that make the tables.
fn versions() -> String {
    format!(
        "crate={} feat={} family={} proof={}",
        env!("CARGO_PKG_VERSION"),
        FEAT_VERSION,
        FAMILY_VERSION,
        PROOF_VERSION
    )
}

impl Recipe {
    /// Text of what the table of one file depends on: the versions and the parameters of detection, quality, isotopes, MS2 and families.
    pub fn file_text(&self) -> String {
        let detect = Params {
            budget: Params::default().budget,
            ..self.detect.clone()
        };
        format!(
            "{}\ndetect={:?}\nsteps={:?}\nfamilies={:?}",
            versions(),
            detect,
            self.steps,
            self.fam
        )
    }

    /// Text of what the aligned table depends on: the text of the files and the alignment, the RT map and the filling.
    pub fn table_text(&self) -> String {
        format!(
            "{}\nmode={:?}\nalign={:?}\nrt={:?}\nfill={:?}{}",
            self.file_text(),
            self.mode,
            self.align,
            self.rt,
            self.fill,
            if self.fill_gaps { "" } else { "\nfill=off" }
        )
    }

    /// Fingerprint (16 hex digits) of `file_text`: the name of the per-file cache.
    pub fn file_hash(&self) -> String {
        fingerprint(&self.file_text())[..16].to_string()
    }

    /// Fingerprint (32 hex digits) of `table_text`.
    pub fn table_hash(&self) -> String {
        fingerprint(&self.table_text())
    }
}

/// FNV-1a over `text` in two 64-bit lanes with different seeds: 32 hex digits.
pub fn fingerprint(text: &str) -> String {
    let (mut a, mut b) = (0xcbf29ce484222325u64, 0x84222325cbf29ce4u64);
    for &c in text.as_bytes() {
        a = (a ^ c as u64).wrapping_mul(0x100000001b3);
        b = (b ^ c as u64).wrapping_mul(0x100000001b3).rotate_left(5);
    }
    format!("{a:016x}{b:016x}")
}
