// Ported from Attributio ms2_engine/compiler.py.
//! Library compiler: the spectra of a GNPS JSON / JSON-lines or MSP text become engine entries (polarity, precursor, adduct, peaks as float32 like the original),
//! with the library name cleaned of the file names, collision-energy tags and energy suffixes that the public libraries put in the name field.

use crate::attributio::names::clean_raw;
use mzlab_lib::parse::{parse_all, Format, Stats};

/// One library spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineEntry {
    pub id: String,
    pub name: String,
    pub precursor_mz: f64,
    /// +1 positive, -1 negative.
    pub pol: i8,
    pub adduct: String,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

/// The name of a library spectrum without what is not the compound: a file name (`….mzML`) gives no name at all, then the `CollisionEnergy…` tag, the energy
/// suffix (` - 30.0 eV`, ` 30 eV`) and the usual qualifiers (`clean_raw`).
pub fn clean_library_name(raw: &str) -> String {
    let t = raw.trim();
    let low = t.to_lowercase();
    if [".mzml", ".mzxml", ".raw", ".d", ".wiff", ".cdf"]
        .iter()
        .any(|e| low.ends_with(e))
    {
        return String::new();
    }
    let mut s = t;
    if let Some(i) = s.to_lowercase().find("collisionenergy") {
        s = &s[..i];
    }
    let s = clean_raw(s);
    let b = s.trim_end();
    // trailing "- 30.0 eV" / "30 eV"
    let lower = b.to_lowercase();
    if let Some(body) = lower.strip_suffix("ev") {
        let body = body.trim_end();
        let digits = body
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .count();
        if digits > 0 {
            let cut = body.len() - digits;
            return b[..cut]
                .trim_end_matches([' ', '-', '_'])
                .to_string();
        }
    }
    b.to_string()
}

/// Spectra of a library text; `name` decides the format by its extension (`.json`/`.jsonl` → JSON, `.msp` → MSP). Spectra without a polarity or peaks are left
/// out. The statistics tell how many records the parser dropped.
pub fn compile_library(name: &str, data: &[u8]) -> Result<(Vec<EngineEntry>, Stats), String> {
    let low = name.to_lowercase();
    let fmt = if low.ends_with(".json") || low.ends_with(".jsonl") || low.ends_with(".ndjson") {
        Format::Json
    } else if low.ends_with(".msp") {
        Format::Msp
    } else {
        return Err(format!("unsupported library format: {name}"));
    };
    let (spectra, stats) = parse_all(fmt, data);
    let mut out = Vec::with_capacity(spectra.len());
    for s in spectra {
        if s.pol == 0 || s.mz.is_empty() {
            continue;
        }
        let id = if s.meta.accession.is_empty() {
            s.meta.name.clone()
        } else {
            s.meta.accession.clone()
        };
        if id.is_empty() {
            continue;
        }
        out.push(EngineEntry {
            id,
            name: clean_library_name(&s.meta.name),
            precursor_mz: s.prec,
            pol: s.pol,
            adduct: s.meta.adduct.clone(),
            mz: s.mz,
            it: s.it,
        });
    }
    Ok((out, stats))
}

#[cfg(test)]
mod tests {
    use super::clean_library_name as c;

    #[test]
    fn names() {
        assert_eq!(c("Atrazine - 30.0 eV"), "Atrazine");
        assert_eq!(c("Atrazine CollisionEnergy:30"), "Atrazine");
        assert_eq!(c("20190101_sample_01.mzML"), "");
        assert_eq!(c("Massbank:Caffeine (9CI)"), "Caffeine");
        assert_eq!(c("Imidacloprid 40 eV"), "Imidacloprid");
    }
}
