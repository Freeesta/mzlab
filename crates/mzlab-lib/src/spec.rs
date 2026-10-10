//! A library spectrum as the parsers produce it and the builder consumes it.

/// Text fields kept per spectrum (the `.mzlib` metadata section stores them in this order).
pub const META_FIELDS: usize = 10;
/// A text field longer than this is cut (at a character boundary) when it is read from a file.
pub const MAX_FIELD_BYTES: usize = 2048;
/// More peaks than this in one record are ignored (the rest is counted by the parser, not stored).
pub const MAX_RAW_PEAKS: usize = 200_000;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub name: String,
    pub adduct: String,
    pub ce: String,
    pub instrument: String,
    pub formula: String,
    pub inchikey: String,
    pub smiles: String,
    pub accession: String,
    pub authors: String,
    pub license: String,
}

impl Meta {
    pub fn fields(&self) -> [&str; META_FIELDS] {
        [
            &self.name,
            &self.adduct,
            &self.ce,
            &self.instrument,
            &self.formula,
            &self.inchikey,
            &self.smiles,
            &self.accession,
            &self.authors,
            &self.license,
        ]
    }

    pub fn from_fields(f: [String; META_FIELDS]) -> Meta {
        let [name, adduct, ce, instrument, formula, inchikey, smiles, accession, authors, license] =
            f;
        Meta {
            name,
            adduct,
            ce,
            instrument,
            formula,
            inchikey,
            smiles,
            accession,
            authors,
            license,
        }
    }
}

/// Peaks as read (not cleaned): `mz` and `it` have the same length, every value positive and finite.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spectrum {
    pub prec: f64,
    /// +1, -1 or 0 (not known).
    pub pol: i8,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub meta: Meta,
}

/// Licence classes of the `u8` in the spectra table (the exact text stays in the metadata).
pub mod licence {
    pub const UNKNOWN: u8 = 0;
    pub const CC0: u8 = 1;
    pub const CC_BY: u8 = 2;
    pub const CC_BY_SA: u8 = 3;
    pub const CC_BY_NC: u8 = 4;
    pub const CC_BY_NC_SA: u8 = 5;
    pub const OTHER: u8 = 6;

    /// Class of a licence text («CC BY-SA 4.0», «CC0», a creativecommons.org URL…).
    pub fn class_of(text: &str) -> u8 {
        let t = text.to_ascii_lowercase();
        let t = t.trim();
        if t.is_empty() {
            return UNKNOWN;
        }
        let compact: String = t.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if compact.contains("cc0") || compact.contains("publicdomain") || compact.contains("zero") {
            CC0
        } else if compact.contains("bsyncsa") || compact.contains("byncsa") {
            CC_BY_NC_SA
        } else if compact.contains("bync") {
            CC_BY_NC
        } else if compact.contains("bysa") {
            CC_BY_SA
        } else if compact.contains("ccby")
            || compact.starts_with("by")
            || compact.contains("licensesby")
        {
            CC_BY
        } else {
            OTHER
        }
    }
}

#[cfg(test)]
mod tests {
    use super::licence::*;

    #[test]
    fn licence_classes() {
        assert_eq!(class_of(""), UNKNOWN);
        assert_eq!(class_of("CC0"), CC0);
        assert_eq!(class_of("CC BY"), CC_BY);
        assert_eq!(class_of("CC BY 4.0"), CC_BY);
        assert_eq!(class_of("CC BY-SA"), CC_BY_SA);
        assert_eq!(class_of("CC BY-NC"), CC_BY_NC);
        assert_eq!(class_of("CC BY-NC-SA 4.0"), CC_BY_NC_SA);
        assert_eq!(
            class_of("https://creativecommons.org/licenses/by/4.0/"),
            CC_BY
        );
        assert_eq!(
            class_of("https://creativecommons.org/publicdomain/zero/1.0/"),
            CC0
        );
        assert_eq!(class_of("all rights reserved"), OTHER);
    }
}
