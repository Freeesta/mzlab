//! The encoder of the binary arrays of `MzmlWriter` (little-endian floats or numpress, zlib, base64) with the scratch buffers it keeps from one array
//! to the next, and the text of a whole spectrum built with it. The writer owns one; each thread of the encoding pool (`pool.rs`) owns its own.

use std::borrow::Cow;
use std::fmt::Write as _;

use base64::Engine;
use flate2::{Compress, Compression, FlushCompress};

use super::{io, spectrum_head, Opzioni, SpectrumExtra};
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::numpress::{self, Np};

/// The values of one binary array: 64-bit or 32-bit floats.
pub(super) enum Data<'a> {
    F64(&'a [f64]),
    F32(&'a [f32]),
    /// 64-bit values written with a numpress codec (`Np::Linear` or `Np::Slof`).
    Np(Np, Cow<'a, [f64]>),
}

pub(super) struct Codifica {
    opz: Opzioni,
    /// The zlib state and the two scratch buffers are kept from one array to the next (same bytes as a new encoder each time, without the allocations).
    z: Compress,
    raw: Vec<u8>,
    packed: Vec<u8>,
    /// The numpress bytes of the array being written.
    np: Vec<u8>,
    /// The fixed-size output chunk of zlib.
    blocco: Vec<u8>,
}

impl Codifica {
    pub fn new(opz: Opzioni) -> Self {
        Codifica { opz, z: Compress::new(Compression::new(opz.livello.min(9)), true), raw: Vec::new(), packed: Vec::new(), np: Vec::new(), blocco: vec![0; 32 * 1024] }
    }

    /// Appends to `s` the whole `<spectrum>` number `index`: head, the m/z and intensity arrays, closing tags.
    pub fn spettro(&mut self, index: usize, sp: &Spectrum, extra: &SpectrumExtra, s: &mut String) -> Result<()> {
        s.push_str(&spectrum_head(index, sp, extra));
        *s += "<binaryDataArrayList count=\"2\">\n";
        let res = self.arrays(s, sp);
        *s += "</binaryDataArrayList></spectrum>\n";
        res
    }

    fn arrays(&mut self, s: &mut String, sp: &Spectrum) -> Result<()> {
        let np = self.opz.numpress && sp.mz.len() >= 3;
        if np {
            self.array(s, "mz", "mz", Data::Np(Np::Linear, Cow::Borrowed(&sp.mz)))?;
        } else {
            self.array(s, "mz", "mz", Data::F64(&sp.mz))?;
        }
        match (&sp.wide, np) {
            (Some(w), false) => self.array(s, "intensity", "counts", Data::F64(w)),
            (None, false) => self.array(s, "intensity", "counts", Data::F32(&sp.intensity)),
            (Some(w), true) => self.array(s, "intensity", "counts", Data::Np(Np::Slof, Cow::Borrowed(w))),
            (None, true) => {
                let v: Vec<f64> = sp.intensity.iter().map(|&x| f64::from(x)).collect();
                self.array(s, "intensity", "counts", Data::Np(Np::Slof, Cow::Owned(v)))
            }
        }
    }

    /// Appends `<binaryDataArray>` to `s`: the values as little-endian floats (or a numpress codec), zlib (level `Opzioni::livello`) and base64.
    pub fn array(&mut self, s: &mut String, kind: &str, unit: &str, data: Data) -> Result<()> {
        self.raw.clear();
        // (accession, name) of the number type or of the numpress codec, and whether zlib follows
        let (acc, name, zlib): (&str, &str, bool) = match &data {
            Data::F64(v) => {
                self.raw.reserve(v.len() * 8);
                v.iter().for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                ("MS:1000523", "64-bit float", true)
            }
            Data::F32(v) => {
                self.raw.reserve(v.len() * 4);
                v.iter().for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                ("MS:1000521", "32-bit float", true)
            }
            Data::Np(np, v) => {
                self.np = match np {
                    Np::Slof => numpress::encode_slof(v, numpress::slof_fixed_point(v)),
                    _ => numpress::encode_linear(v, numpress::linear_fixed_point(v)),
                };
                self.raw.extend_from_slice(&self.np);
                (if *np == Np::Slof { "MS:1002314" } else { "MS:1002312" }, "", false)
            }
        };
        let is_np = matches!(data, Data::Np(..));
        let mut zlib = zlib;
        if is_np {
            // zlib after the codec only when it makes the array smaller
            self.deflate()?;
            zlib = self.packed.len() < self.raw.len();
            if !zlib {
                std::mem::swap(&mut self.packed, &mut self.raw);
            }
        } else {
            self.deflate()?;
        }
        let len = self.packed.len().div_ceil(3) * 4;
        let (unit_acc, unit_name) = match unit {
            "time" => ("UO:0000031", "minute"),
            "mz" => ("MS:1000040", "m/z"),
            _ => ("MS:1000131", "number of detector counts"),
        };
        let (cv, label) = match kind {
            "time" => ("MS:1000595", "time array"),
            "mz" => ("MS:1000514", "m/z array"),
            _ => ("MS:1000515", "intensity array"),
        };
        let unit_cv = if unit == "time" { "UO" } else { "MS" };
        let _ = write!(s, "<binaryDataArray encodedLength=\"{len}\">");
        if is_np {
            // the values are 64-bit floats once decoded; the codec (and zlib with it) replaces the compression term
            let (acc, name) = match (acc, zlib) {
                ("MS:1002314", false) => ("MS:1002314", "MS-Numpress short logged float compression"),
                ("MS:1002314", true) => ("MS:1002748", "MS-Numpress short logged float compression followed by zlib compression"),
                (_, false) => ("MS:1002312", "MS-Numpress linear prediction compression"),
                (_, true) => ("MS:1002746", "MS-Numpress linear prediction compression followed by zlib compression"),
            };
            let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/>");
        } else {
            let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\" value=\"\"/>");
        }
        let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{cv}\" name=\"{label}\" value=\"\" unitCvRef=\"{unit_cv}\" unitAccession=\"{unit_acc}\" unitName=\"{unit_name}\"/><binary>");
        base64::engine::general_purpose::STANDARD.encode_string(&self.packed, s);
        s.push_str("</binary></binaryDataArray>\n");
        Ok(())
    }

    /// `raw` compressed into `packed` (a zlib stream at the default level, as `ZlibEncoder` writes it). The output goes through a chunk of fixed size: with a
    /// buffer whose room depends on what was compressed before (as `compress_vec` uses it) the same array can come out as different bytes.
    fn deflate(&mut self) -> Result<()> {
        self.z.reset();
        self.packed.clear();
        let mut fatto = 0;
        loop {
            let (dentro, fuori) = (self.z.total_in(), self.z.total_out());
            let stato = self.z.compress(&self.raw[fatto..], &mut self.blocco, FlushCompress::Finish).map_err(|_| io(std::io::ErrorKind::Other.into()))?;
            let (letti, scritti) = ((self.z.total_in() - dentro) as usize, (self.z.total_out() - fuori) as usize);
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

/// The cores minus one (the reader and the writer share the calling thread), at least 1.
#[cfg(not(target_family = "wasm"))]
pub fn thread_predefinito() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).max(1))
}

/// Always 1 in the browser, which has no thread group.
#[cfg(target_family = "wasm")]
pub fn thread_predefinito() -> usize {
    1
}
