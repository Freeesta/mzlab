//! One cvParam of an mzML, from the element itself or replayed from a `referenceableParamGroup`: what it says about the spectrum,
//! chromatogram or binary array being read (`mzml::parse_loop`).

use super::{help::Decode, mobility_of, Array, ArrayKind, Chrom, Item, Num, Scope, Spec};
use crate::error::{Error, Result};

/// What `parse_loop` is reading now: the element being filled and the flags of the lists it is inside.
pub(super) struct Cur {
    pub(super) scope: Scope,
    pub(super) spec: Option<Spec>,
    pub(super) chrom: Option<Chrom>,
    pub(super) array: Option<Array>,
    pub(super) in_precursor_list: bool,
    pub(super) in_analyzer: bool,
}

impl Default for Cur {
    fn default() -> Self {
        Cur {
            scope: Scope::None,
            spec: None,
            chrom: None,
            array: None,
            in_precursor_list: false,
            in_analyzer: false,
        }
    }
}

impl Cur {
    /// Applies `[name, accession, value, unitName, unitAccession]` to the element being read.
    pub(super) fn apply<D: Decode + ?Sized>(
        &mut self,
        [name, acc, value, unit, unit_acc]: [&str; 5],
        out: &mut D,
        f: &mut dyn FnMut(Item),
    ) -> Result<()> {
        if self.in_analyzer {
            out.item(Item::Analyzer(name.to_string()), f)?;
        }
        let seconds = super::is_seconds(Some(unit), Some(unit_acc));
        if let (Scope::Chromatogram, Some(c)) = (self.scope, self.chrom.as_mut()) {
            match acc {
                "MS:1000812" | "MS:1000813" => c.pda = true,
                "MS:1003019" => c.pressure = true,
                "MS:1000235" | "MS:1000627" | "MS:1000628" | "MS:1001473" | "MS:1001472" => {
                    c.ms_term = true
                }
                _ => {}
            }
        }
        if let Some(arr) = self.array.as_mut() {
            match acc {
                "MS:1000514" => arr.kind = ArrayKind::Mz,
                "MS:1000515" => {
                    arr.kind = ArrayKind::Intensity;
                    if let (false, Some(c)) = (unit.is_empty(), self.chrom.as_mut()) {
                        c.unit = unit.to_string();
                    }
                }
                "MS:1000595" => {
                    arr.kind = ArrayKind::Time;
                    arr.seconds = seconds;
                }
                "MS:1000574" => arr.zlib = true,
                "MS:1002312" | "MS:1002313" | "MS:1002314" => {
                    return Err(Error::new("err.file.numpress"))
                }
                _ => {
                    if let Some(n) = Num::of_accession(acc) {
                        arr.num = n;
                    } else if let Some(u) = mobility_of(acc) {
                        (arr.kind, arr.mobility) = (ArrayKind::Mobility, u);
                    }
                }
            }
        } else if let (Scope::Spectrum, Some(s)) = (self.scope, self.spec.as_mut()) {
            spectrum_cv(s, acc, value, seconds, self.in_precursor_list);
        }
        Ok(())
    }
}

/// A cvParam of a `<spectrum>` (level, polarity, retention time, selected ion, ...).
fn spectrum_cv(s: &mut Spec, acc: &str, value: &str, seconds: bool, in_precursor_list: bool) {
    match acc {
        "MS:1000804" => s.uv = true,
        "MS:1000511" if !in_precursor_list => {
            // A level is a small integer; `as` saturates and maps NaN to 0.
            #[allow(clippy::cast_possible_truncation)]
            let level = value.parse::<f64>().map(|v| v as u8).unwrap_or(1);
            s.level = level
        }
        "MS:1000130" => s.polarity = 1,
        "MS:1000129" => s.polarity = -1,
        "MS:1000128" => s.profile = true,
        "MS:1000512" if !in_precursor_list => s.filter = value.to_string(),
        "MS:1000016" => {
            s.rt = value.parse().unwrap_or(0.0);
            s.rt_seconds = seconds;
        }
        "MS:1000744" => s.selected = value.parse().ok(),
        "MS:1000827" => s.target = value.parse().ok(),
        _ => {
            s.mobility_scan = s.mobility_scan.or_else(|| {
                mobility_of(acc)
                    .zip(value.parse().ok())
                    .map(|(u, v)| (v, u))
            })
        }
    }
}
