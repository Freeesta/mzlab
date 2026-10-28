//! `Codifica::array`: one `<binaryDataArray>` of a spectrum or a chromatogram (floats or numpress, zlib, base64).

use std::fmt::Write as _;

use base64::Engine;

use super::codifica::{Codifica, Data};
use super::{spectrum_head_into, SpectrumExtra};
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::numpress::{self, Np};

impl Codifica {
    /// Appends to `s` the whole `<spectrum>` number `index`: head, the m/z and intensity arrays, closing tags.
    pub fn spettro(
        &mut self,
        index: usize,
        sp: &Spectrum,
        extra: &SpectrumExtra,
        s: &mut String,
    ) -> Result<()> {
        spectrum_head_into(s, index, sp, extra);
        *s += "<binaryDataArrayList count=\"2\">\n";
        let res = self.arrays(s, sp);
        *s += "</binaryDataArrayList></spectrum>\n";
        res
    }

    fn arrays(&mut self, s: &mut String, sp: &Spectrum) -> Result<()> {
        let np = self.opz.numpress && sp.mz.len() >= 3;
        if np {
            self.array(s, "mz", "mz", Data::Np(Np::Linear, &sp.mz))?;
        } else {
            self.array(s, "mz", "mz", Data::F64(&sp.mz))?;
        }
        let data = match (&sp.wide, np) {
            (Some(w), false) => Data::F64(w),
            (None, false) => Data::F32(&sp.intensity),
            (Some(w), true) => Data::Np(Np::Slof, w),
            (None, true) => Data::NpF32(&sp.intensity),
        };
        self.array(s, "intensity", "counts", data)
    }

    /// Appends `<binaryDataArray>` to `s`: the values as little-endian floats (or a numpress codec), zlib (level `Opzioni::livello`) and base64.
    pub fn array(&mut self, s: &mut String, kind: &str, unit: &str, data: Data) -> Result<()> {
        self.raw.clear();
        // (accession, name) of the number type or of the numpress codec, and whether zlib follows
        let (acc, name, zlib): (&str, &str, bool) = match &data {
            Data::F64(v) => {
                le_bytes(&mut self.raw, v, |x| x.to_le_bytes());
                ("MS:1000523", "64-bit float", true)
            }
            Data::F32(v) => {
                le_bytes(&mut self.raw, v, |x| x.to_le_bytes());
                ("MS:1000521", "32-bit float", true)
            }
            Data::Np(Np::Slof, v) => {
                numpress::encode_slof_into(&mut self.raw, v, numpress::slof_fixed_point(v));
                ("MS:1002314", "", false)
            }
            Data::NpF32(v) => {
                numpress::encode_slof_f32_into(&mut self.raw, v, numpress::slof_fixed_point_f32(v));
                ("MS:1002314", "", false)
            }
            Data::Np(_, v) => {
                numpress::encode_linear_into(&mut self.raw, v, numpress::linear_fixed_point(v));
                ("MS:1002312", "", false)
            }
        };
        let is_np = matches!(data, Data::Np(..) | Data::NpF32(..));
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
        let ((unit_acc, unit_name), (cv, label)) = (unit_term(unit), kind_term(kind));
        let unit_cv = if matches!(unit, "time" | "nm" | "au") {
            "UO"
        } else {
            "MS"
        };
        let _ = write!(s, "<binaryDataArray encodedLength=\"{len}\">");
        if is_np {
            // the values are 64-bit floats once decoded; the codec (and zlib with it) replaces the compression term
            let (acc, name) = np_term(acc, zlib);
            let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"MS:1000523\" name=\"64-bit float\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/>");
        } else {
            let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\" value=\"\"/>");
        }
        let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"{cv}\" name=\"{label}\" value=\"\" unitCvRef=\"{unit_cv}\" unitAccession=\"{unit_acc}\" unitName=\"{unit_name}\"/><binary>");
        base64::engine::general_purpose::STANDARD.encode_string(&self.packed, s);
        s.push_str("</binary></binaryDataArray>\n");
        Ok(())
    }
}

/// `out` = the little-endian bytes of `v` (`N` bytes each): sized once and filled in chunks, which the compiler turns into one copy on a little-endian
/// machine (the crate forbids `unsafe`, so no cast of the slice to bytes).
fn le_bytes<T: Copy, const N: usize>(out: &mut Vec<u8>, v: &[T], to_le: impl Fn(T) -> [u8; N]) {
    out.resize(v.len() * N, 0);
    for (c, x) in out.chunks_exact_mut(N).zip(v) {
        c.copy_from_slice(&to_le(*x));
    }
}

/// (accession, name) of the unit of an array.
fn unit_term(unit: &str) -> (&'static str, &'static str) {
    match unit {
        "time" => ("UO:0000031", "minute"),
        "mz" => ("MS:1000040", "m/z"),
        "nm" => ("UO:0000018", "nanometer"),
        "au" => ("UO:0000269", "absorbance unit"),
        _ => ("MS:1000131", "number of detector counts"),
    }
}

/// (accession, name) of the kind of an array.
fn kind_term(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "time" => ("MS:1000595", "time array"),
        "mz" => ("MS:1000514", "m/z array"),
        "wavelength" => ("MS:1000617", "wavelength array"),
        _ => ("MS:1000515", "intensity array"),
    }
}

/// The numpress term of a codec (`acc`: slof or linear) with or without zlib after it.
fn np_term(acc: &str, zlib: bool) -> (&'static str, &'static str) {
    match (acc, zlib) {
        ("MS:1002314", false) => ("MS:1002314", "MS-Numpress short logged float compression"),
        ("MS:1002314", true) => (
            "MS:1002748",
            "MS-Numpress short logged float compression followed by zlib compression",
        ),
        (_, false) => ("MS:1002312", "MS-Numpress linear prediction compression"),
        (_, true) => (
            "MS:1002746",
            "MS-Numpress linear prediction compression followed by zlib compression",
        ),
    }
}
