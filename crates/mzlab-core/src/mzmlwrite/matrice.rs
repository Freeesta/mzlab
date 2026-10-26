//! `Codifica::array`: one `<binaryDataArray>` of a spectrum or a chromatogram (floats or numpress, zlib, base64).

use std::borrow::Cow;
use std::fmt::Write as _;

use base64::Engine;

use super::codifica::{Codifica, Data};
use super::{spectrum_head, SpectrumExtra};
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
            (Some(w), true) => self.array(
                s,
                "intensity",
                "counts",
                Data::Np(Np::Slof, Cow::Borrowed(w)),
            ),
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
                v.iter()
                    .for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                ("MS:1000523", "64-bit float", true)
            }
            Data::F32(v) => {
                self.raw.reserve(v.len() * 4);
                v.iter()
                    .for_each(|x| self.raw.extend_from_slice(&x.to_le_bytes()));
                ("MS:1000521", "32-bit float", true)
            }
            Data::Np(np, v) => {
                self.np = match np {
                    Np::Slof => numpress::encode_slof(v, numpress::slof_fixed_point(v)),
                    _ => numpress::encode_linear(v, numpress::linear_fixed_point(v)),
                };
                self.raw.extend_from_slice(&self.np);
                (
                    if *np == Np::Slof {
                        "MS:1002314"
                    } else {
                        "MS:1002312"
                    },
                    "",
                    false,
                )
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
