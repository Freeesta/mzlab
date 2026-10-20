//! The `.wasm` build of `mzmlwrite::indice`: no index. The indexed mzML is written by the native `convert.mzml` step only (`RunsIn::native`), and the
//! SHA-1 and the index text would push the reader modules over their size ceilings (`tools/build_lettori_wasm.sh`).
use std::io::Write;

use super::{Header, MzmlWriter, Opzioni, Result};
use crate::error::Error;

pub(super) struct Indice;

impl Indice {
    pub fn new() -> Self {
        Indice
    }
}

impl<W: Write> MzmlWriter<W> {
    /// Not available in the browser build.
    pub fn indicizzato(_: W, _: &Header, _: Opzioni) -> Result<Self> {
        Err(Error::new("err.route.unavailable"))
    }

    pub(super) fn put(&mut self, b: &[u8]) -> Result<()> {
        self.out.write_all(b).map_err(super::io)
    }

    pub(super) fn mark(&mut self, _: bool, _: &str) {}

    pub(super) fn close_index(&mut self) -> Result<()> {
        Ok(())
    }
}
