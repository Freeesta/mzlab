//! A member of a dataset of the engine, read through the page's `read(i, offset, length) -> Uint8Array` (an OPFS sync-handle read in the index builder)
//! and a `BlockCache` that reads ahead along reads in order: the readers of the `.D` and `.wiff` ask a few hundred bytes at a time.
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use wasm_bindgen::prelude::*;

/// Bytes a member is read in when a reader asks less: every `read` of the page is an OPFS sync-handle read with a fixed cost per call (about 0.25 ms),
/// and the `.D` reader asks a few hundred bytes per scan (6000 calls for 6000 scans).
const READ_AHEAD: usize = 256 << 10;
/// One block per member: the `.wiff` and `.D` readers go along one member at a time, a second block saved no read.
const BLOCKS: usize = 1;
/// The longest block a run of reads in order grows to (`BlockCache::read_ahead`): at most this many bytes held per member.
const READ_AHEAD_MAX: usize = 4 << 20;
/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`, or `read.into(i, offset, view)` when the page has it (`rust-nativo.js`, over OPFS handles):
/// the handle reads straight into the engine's memory. The view (as `lend` of `natives.rs`, the other way) is valid during the call only: JS fills it at once, never keeps it,
/// makes no call into the engine meanwhile, and answers the bytes it put.
struct JsMember {
    read: Function,
    into: Option<Function>,
    i: u32,
}

impl ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let (i, o) = (JsValue::from(self.i), JsValue::from(off as f64));
        if let Some(into) = &self.into {
            let mem: js_sys::WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
            let (at, n) = u32::try_from(buf.as_mut_ptr() as usize)
                .ok()
                .zip(u32::try_from(buf.len()).ok())
                .ok_or_else(bad)?;
            let v = js_sys::Uint8Array::new_with_byte_offset_and_length(&mem.buffer(), at, n);
            let k = into.call3(&JsValue::NULL, &i, &o, &v).map_err(|_| bad())?;
            return (k.as_f64() == Some(f64::from(n)))
                .then_some(())
                .ok_or_else(bad);
        }
        let v = self
            .read
            .call3(&JsValue::NULL, &i, &o, &(buf.len() as f64).into())
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
pub(super) fn member(read: &Function, i: u32, dim: u64) -> Box<dyn ReadAt> {
    let mut m = JsMember {
        read: read.clone(),
        into: js_sys::Reflect::get(read, &JsValue::from_str("into"))
            .ok()
            .and_then(|f| f.dyn_into().ok()),
        i,
    };
    Box::new(
        mzlab_core::blocks::BlockCache::new(
            move |off: u64, buf: &mut [u8]| m.read_at(off, buf),
            dim,
            READ_AHEAD,
            BLOCKS,
        )
        .read_ahead(READ_AHEAD_MAX),
    )
}
