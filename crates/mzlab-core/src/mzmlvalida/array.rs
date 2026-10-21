//! One `<binaryDataArray>` of the file under check: its terms, its text, and the values it decodes to.

use crate::error::Result;
use crate::mzmlarray::{decode_array, Inflate, Want};
use crate::numarray::Num;
use crate::numpress;

/// What the pass has seen of an array before `</binaryDataArray>`.
#[derive(Default)]
pub(super) struct Arr {
    /// Accessions of its cvParams.
    pub terms: Vec<String>,
    pub text: String,
    pub encoded_length: Option<usize>,
    pub array_length: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Mz,
    Intensity,
    Time,
    Other,
}

impl Arr {
    pub fn kind(&self) -> Kind {
        for t in &self.terms {
            match t.as_str() {
                "MS:1000514" => return Kind::Mz,
                "MS:1000515" => return Kind::Intensity,
                "MS:1000595" => return Kind::Time,
                _ => {}
            }
        }
        Kind::Other
    }

    /// The values: base64, then zlib or a numpress codec as the terms say, then the number type (64-bit float when the file does not say).
    pub fn decode(&self, inf: &mut Inflate) -> Result<Vec<f64>> {
        let zlib = self.terms.iter().any(|t| t == "MS:1000574");
        let text = self.text.trim();
        if let Some((np, z)) = self.terms.iter().find_map(|t| numpress::of_accession(t)) {
            return numpress::decode_text(np, z, text, 0);
        }
        let num = self
            .terms
            .iter()
            .find_map(|t| Num::of_accession(t))
            .unwrap_or(Num::F64);
        // 8 M values is the reader's limit for an array whose length the file does not state
        decode_array(num, zlib, text, 8 << 20, inf, Want::F64).map(|(v, _)| v)
    }
}
