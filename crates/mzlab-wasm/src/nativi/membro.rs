//! A member of a dataset of the engine, read through the page's `read(i, offset, length) -> Uint8Array` (an OPFS sync-handle read in the index builder)
//! and a `BlockCache` that reads ahead along reads in order: the readers of the `.D` and `.wiff` ask a few hundred bytes at a time.
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use wasm_bindgen::prelude::*;

/// Bytes a member is read in when a reader asks less: every `read` of the page is an OPFS sync-handle read with a fixed cost per call (about 0.25 ms),
/// and the `.D` reader asks a few hundred bytes per scan (6000 calls for 6000 scans).
const LETTURA: usize = 256 << 10;
/// One block per member: the `.wiff` and `.D` readers go along one member at a time, a second block saved no read.
const BLOCCHI: usize = 1;
/// The longest block a run of reads in order grows to (`BlockCache::read_ahead`): at most this many bytes held per member.
const LETTURA_MAX: usize = 4 << 20;
/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`.
struct JsMember {
    read: Function,
    i: u32,
}

impl ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let v = self
            .read
            .call3(
                &JsValue::NULL,
                &self.i.into(),
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| bad())?;
        let a: js_sys::Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != buf.len() {
            return Err(bad());
        }
        a.copy_to(buf);
        Ok(())
    }
}

/// Member `i` (of `dim` bytes) as the readers of the registry read it.
pub(super) fn membro(read: &Function, i: u32, dim: u64) -> Box<dyn ReadAt> {
    let mut m = JsMember {
        read: read.clone(),
        i,
    };
    Box::new(
        mzlab_core::blocchi::BlockCache::new(
            move |off: u64, buf: &mut [u8]| m.read_at(off, buf),
            dim,
            LETTURA,
            BLOCCHI,
        )
        .read_ahead(LETTURA_MAX),
    )
}
