//! What a library says about itself (`SEC_LIBINFO` of a v2 `.mzlib`): experimental or in silico, LC-MS/MS, GC-EI or MSⁿ, counters per polarity and analyzer, and where it
//! comes from. The builder detects everything from the records (`Detect`); the reader hands it back as `LibInfo`. Also the InChIKey hash used to group duplicates.

use crate::error::{Error, Result};
use crate::spec::Spectrum;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Experimental,
    InSilico,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Experimental => "experimental",
            Kind::InSilico => "in_silico",
        }
    }

    fn parse(s: &str) -> Option<Kind> {
        match s {
            "experimental" => Some(Kind::Experimental),
            "in_silico" => Some(Kind::InSilico),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tech {
    LcMsms,
    GcEi,
    Msn,
}

impl Tech {
    pub fn as_str(self) -> &'static str {
        match self {
            Tech::LcMsms => "lc_msms",
            Tech::GcEi => "gc_ei",
            Tech::Msn => "msn",
        }
    }

    fn parse(s: &str) -> Option<Tech> {
        match s {
            "lc_msms" => Some(Tech::LcMsms),
            "gc_ei" => Some(Tech::GcEi),
            "msn" => Some(Tech::Msn),
            _ => None,
        }
    }
}

/// Analyzer classes counted in the info, in the order of `Detect::analyzers`.
pub const ANALYZERS: [&str; 5] = ["orbitrap", "qtof", "qqq", "it", "ei"];

/// Content of `SEC_LIBINFO`. A v1 file has none: every field is empty (`kind` and `tech` unknown).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibInfo {
    pub kind: Option<Kind>,
    pub tech: Option<Tech>,
    pub pos: u32,
    pub neg: u32,
    pub unknown_pol: u32,
    /// Spectra per analyzer class (`ANALYZERS`); classes with no spectrum are absent.
    pub analyzers: BTreeMap<String, u32>,
    pub source: String,
    pub version: String,
    /// SHA-256 (hex) of the source file the library was built from, as the caller states it.
    pub sha256: String,
}

impl LibInfo {
    pub fn to_json(&self) -> Vec<u8> {
        let v = json!({
            "kind": self.kind.map(Kind::as_str),
            "tech": self.tech.map(Tech::as_str),
            "pol": { "pos": self.pos, "neg": self.neg, "unknown": self.unknown_pol },
            "analyzers": self.analyzers,
            "source": self.source,
            "version": self.version,
            "sha256": self.sha256,
        });
        v.to_string().into_bytes()
    }

    pub fn from_json(b: &[u8]) -> Result<LibInfo> {
        let v: Value = serde_json::from_slice(b).map_err(|_| Error::corrupt("libinfo"))?;
        let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let num = |n: Option<&Value>| {
            n.and_then(Value::as_u64)
                .map_or(0, |n| u32::try_from(n).unwrap_or(u32::MAX))
        };
        let pol = v.get("pol").cloned().unwrap_or(Value::Null);
        let analyzers = v
            .get("analyzers")
            .and_then(Value::as_object)
            .map(|o| o.iter().map(|(k, n)| (k.clone(), num(Some(n)))).collect())
            .unwrap_or_default();
        Ok(LibInfo {
            kind: v.get("kind").and_then(Value::as_str).and_then(Kind::parse),
            tech: v.get("tech").and_then(Value::as_str).and_then(Tech::parse),
            pos: num(pol.get("pos")),
            neg: num(pol.get("neg")),
            unknown_pol: num(pol.get("unknown")),
            analyzers,
            source: text("source"),
            version: text("version"),
            sha256: text("sha256"),
        })
    }
}

/// Words that mark a spectrum as predicted rather than measured (lower case, searched in name, instrument, accession, authors and licence).
const IN_SILICO_WORDS: [&str; 6] = [
    "lipidblast",
    "in-silico",
    "in silico",
    "insilico",
    "predicted",
    "cfm-id",
];

fn analyzer_of(instrument: &str, ei: bool) -> Option<usize> {
    if ei {
        return Some(4);
    }
    let t = instrument.to_ascii_lowercase();
    if t.contains("orbitrap") || t.contains("exactive") {
        Some(0)
    } else if t.contains("tof") {
        Some(1)
    } else if t.contains("qqq")
        || t.contains("triple")
        || t.contains("qtrap")
        || t.contains("quattro")
    {
        Some(2)
    } else if t.contains("ion trap")
        || t.contains("iontrap")
        || t.contains("ltq")
        || t == "it"
        || t.starts_with("it-")
    {
        Some(3)
    } else {
        None
    }
}

/// What the builder may be told instead of detecting it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Declared {
    pub kind: Option<Kind>,
    pub tech: Option<Tech>,
}

/// Counters over the spectra added to a builder.
#[derive(Debug, Clone, Default)]
pub struct Detect {
    n: u32,
    pos: u32,
    neg: u32,
    unknown: u32,
    analyzers: [u32; 5],
    in_silico: u32,
    gc: u32,
}

impl Detect {
    pub fn observe(&mut self, s: &Spectrum) {
        self.n += 1;
        match s.pol {
            p if p > 0 => self.pos += 1,
            p if p < 0 => self.neg += 1,
            _ => self.unknown += 1,
        }
        let ei = s.ei || s.ri.is_some();
        if ei {
            self.gc += 1;
        }
        if let Some(a) = analyzer_of(&s.meta.instrument, ei) {
            self.analyzers[a] += 1;
        }
        let m = &s.meta;
        let marked = [&m.name, &m.instrument, &m.accession, &m.authors, &m.license]
            .iter()
            .any(|f| {
                let t = f.to_ascii_lowercase();
                IN_SILICO_WORDS.iter().any(|w| t.contains(w))
            });
        if marked {
            self.in_silico += 1;
        }
    }

    /// The library is judged by the majority of its spectra; `declared` wins over the detection.
    pub fn finish(&self, declared: Declared) -> LibInfo {
        let half = |x: u32| x as u64 * 2 > self.n as u64;
        LibInfo {
            kind: Some(declared.kind.unwrap_or(if half(self.in_silico) {
                Kind::InSilico
            } else {
                Kind::Experimental
            })),
            tech: Some(declared.tech.unwrap_or(if half(self.gc) {
                Tech::GcEi
            } else {
                Tech::LcMsms
            })),
            pos: self.pos,
            neg: self.neg,
            unknown_pol: self.unknown,
            analyzers: ANALYZERS
                .iter()
                .zip(self.analyzers)
                .filter(|(_, n)| *n > 0)
                .map(|(k, n)| (k.to_string(), n))
                .collect(),
            ..LibInfo::default()
        }
    }
}

/// FNV-1a of the first 14 characters of an InChIKey (the connectivity block, case-insensitive); 0 when the key is missing.
pub fn inchikey14(key: &str) -> u64 {
    let k = key.trim();
    if k.len() < 14 {
        return 0;
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in k.bytes().take(14) {
        h ^= b.to_ascii_uppercase() as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Meta;

    #[test]
    fn json_round_trip() {
        let mut i = LibInfo {
            kind: Some(Kind::InSilico),
            tech: Some(Tech::Msn),
            pos: 3,
            neg: 2,
            unknown_pol: 1,
            source: "mb".into(),
            version: "2024.1".into(),
            sha256: "ab".into(),
            ..LibInfo::default()
        };
        i.analyzers.insert("qtof".into(), 4);
        assert_eq!(LibInfo::from_json(&i.to_json()).unwrap(), i);
        assert!(LibInfo::from_json(b"not json").is_err());
        assert_eq!(LibInfo::from_json(b"{}").unwrap(), LibInfo::default());
    }

    #[test]
    fn inchikey_hash_ignores_case_and_the_tail() {
        let a = inchikey14("BSYNRYMUTXBXSQ-UHFFFAOYSA-N");
        assert_ne!(a, 0);
        assert_eq!(a, inchikey14("bsynrymutxbxsq-ZZZZZZZZZZ-N"));
        assert_ne!(a, inchikey14("BSYNRYMUTXBXSR-UHFFFAOYSA-N"));
        assert_eq!(inchikey14(""), 0);
        assert_eq!(inchikey14("short"), 0);
    }

    #[test]
    fn detection_follows_the_majority() {
        let mut d = Detect::default();
        for i in 0..3 {
            d.observe(&Spectrum {
                pol: 1,
                ri: (i < 2).then_some(1200.0),
                meta: Meta {
                    instrument: "Thermo Q Exactive Orbitrap".into(),
                    name: if i == 0 {
                        "LipidBlast x".into()
                    } else {
                        String::new()
                    },
                    ..Meta::default()
                },
                ..Spectrum::default()
            });
        }
        let i = d.finish(Declared::default());
        assert_eq!(i.tech, Some(Tech::GcEi));
        assert_eq!(i.kind, Some(Kind::Experimental));
        assert_eq!(i.pos, 3);
        assert_eq!(i.analyzers.get("ei"), Some(&2));
        assert_eq!(i.analyzers.get("orbitrap"), Some(&1));
        let i = d.finish(Declared {
            kind: Some(Kind::InSilico),
            tech: Some(Tech::LcMsms),
        });
        assert_eq!((i.kind, i.tech), (Some(Kind::InSilico), Some(Tech::LcMsms)));
    }
}
