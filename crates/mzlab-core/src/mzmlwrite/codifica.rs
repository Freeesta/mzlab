//! The encoder of the binary arrays of `MzmlWriter` (little-endian floats or numpress, zlib, base64) with the scratch buffers it keeps from one array
//! to the next, and the text of a whole spectrum built with it. The writer owns one; each thread of the encoding pool (`pool.rs`) owns its own.

use std::borrow::Cow;

use flate2::{Compress, Compression, FlushCompress};

use super::{io, Opzioni};
use crate::error::Result;
use crate::numpress::Np;

/// The values of one binary array: 64-bit or 32-bit floats.
pub(super) enum Data<'a> {
    F64(&'a [f64]),
    F32(&'a [f32]),
    /// 64-bit values written with a numpress codec (`Np::Linear` or `Np::Slof`).
    Np(Np, Cow<'a, [f64]>),
}

pub(super) struct Codifica {
    pub(super) opz: Opzioni,
    /// The zlib state and the two scratch buffers are kept from one array to the next (same bytes as a new encoder each time, without the allocations).
    pub(super) z: Compress,
    pub(super) raw: Vec<u8>,
    pub(super) packed: Vec<u8>,
    /// The numpress bytes of the array being written.
    pub(super) np: Vec<u8>,
    /// The fixed-size output chunk of zlib.
    pub(super) blocco: Vec<u8>,
}

impl Codifica {
    pub fn new(opz: Opzioni) -> Self {
        Codifica {
            opz,
            z: Compress::new(Compression::new(opz.livello.min(9)), true),
            raw: Vec::new(),
            packed: Vec::new(),
            np: Vec::new(),
            blocco: vec![0; 32 * 1024],
        }
    }

    /// `raw` compressed into `packed` (a zlib stream at the default level, as `ZlibEncoder` writes it). The output goes through a chunk of fixed size: with a
    /// buffer whose room depends on what was compressed before (as `compress_vec` uses it) the same array can come out as different bytes.
    pub(super) fn deflate(&mut self) -> Result<()> {
        self.z.reset();
        self.packed.clear();
        let mut fatto = 0;
        loop {
            let (dentro, fuori) = (self.z.total_in(), self.z.total_out());
            let stato = self
                .z
                .compress(&self.raw[fatto..], &mut self.blocco, FlushCompress::Finish)
                .map_err(|_| io(std::io::ErrorKind::Other.into()))?;
            let (letti, scritti) = (
                (self.z.total_in() - dentro) as usize,
                (self.z.total_out() - fuori) as usize,
            );
            fatto += letti;
            self.packed.extend_from_slice(&self.blocco[..scritti]);
            if stato == flate2::Status::StreamEnd {
                return Ok(());
            }
            if letti == 0 && scritti == 0 {
                return Err(io(std::io::ErrorKind::Other.into()));
            }
        }
    }
}
