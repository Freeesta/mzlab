//! Read-only memory map of a file picked by the user. This is the only place with `unsafe` in the crate.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;

pub struct Mapped {
    map: Option<Mmap>,
}

impl Mapped {
    /// Maps the file. An empty file has no map (mapping zero bytes fails on some systems).
    pub fn open(path: &Path) -> std::io::Result<Mapped> {
        let file = File::open(path)?;
        if file.metadata()?.len() == 0 {
            return Ok(Mapped { map: None });
        }
        // SAFETY: the map is read-only and lives as long as the parse of one file; if another program truncates the file meanwhile
        // the worst case is an I/O fault of this process, as for any program reading a file that changes under it.
        let map = unsafe { Mmap::map(&file)? };
        Ok(Mapped { map: Some(map) })
    }

    pub fn bytes(&self) -> &[u8] {
        self.map.as_deref().unwrap_or(&[])
    }
}

/// A reader of a maker's file (`Lettore`) kept between two requests of the page. The engine is behind one `Mutex`: the reader is only ever used by the
/// thread that holds it.
pub struct Held(pub Box<dyn mzlab_core::lettori::Lettore>);

// SAFETY: a `Lettore` is not `Send` only because the trait does not say so (the site's readers read through JavaScript). Here every member is a
// `MapAt` (an `Arc<Mapped>`, which is `Send + Sync`), the reader is created in this crate and never cloned or shared, and it is used only under the
// engine's `Mutex`: moving the whole value to another thread leaves no other handle to anything inside it.
unsafe impl Send for Held {}
