//! What a spectrum header carries beyond the points: the precursor chain (`<precursorList>`) and the cvParams of `<spectrum>` and `<scan>`.
//! Used by `spectrum_head` and filled by the readers through `SpectrumExtra` (`Lettore::extra_mzml`, `Lettore::extra_scansione`).

use super::{cv, esc, mz_cv, unit_cv_str};

/// A cvParam with an optional unit: accession, name, value and (accession, name) of the unit (`UO:` units get `unitCvRef="UO"`, the others `MS`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CvPar {
    pub accession: String,
    pub name: String,
    pub value: String,
    pub unit: Option<(String, String)>,
}

impl CvPar {
    pub fn new(accession: &str, name: &str, value: impl ToString) -> Self {
        CvPar { accession: accession.into(), name: name.into(), value: value.to_string(), unit: None }
    }

    pub fn with_unit(mut self, accession: &str, name: &str) -> Self {
        self.unit = Some((accession.into(), name.into()));
        self
    }

    pub(super) fn xml(&self) -> String {
        let value = esc(&self.value);
        match &self.unit {
            Some((ua, un)) => unit_cv_str(&self.accession, &esc(&self.name), &value, if ua.starts_with("UO:") { "UO" } else { "MS" }, ua, un),
            None => cv(&self.accession, &esc(&self.name), value),
        }
    }
}

/// The cvParams of `list` in order (nothing without the `precursori` feature).
#[cfg(feature = "precursori")]
pub(super) fn params_xml(list: &[CvPar]) -> String {
    let mut s = String::new();
    for p in list {
        s += &p.xml();
    }
    s
}

/// One precursor of a spectrum: where the ion was isolated, which ion it was, and how it was fragmented.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Precursore {
    /// `MS:1000744` selected ion m/z.
    pub selected_mz: Option<f64>,
    /// `MS:1000041` charge state.
    pub charge: Option<i32>,
    /// `MS:1000042` peak intensity.
    pub intensity: Option<f64>,
    /// `MS:1000827` isolation window target m/z.
    pub iso_target: Option<f64>,
    /// `MS:1000828` / `MS:1000829` offsets of the isolation window (positive numbers, in m/z).
    pub iso_lower: Option<f64>,
    pub iso_upper: Option<f64>,
    /// Dissociation method terms (accession, name): CID `MS:1000133`, beam-type CID `MS:1000422`, ETD `MS:1000598`, ...
    pub activation: Vec<(String, String)>,
    /// Energy of the activation as (accession, value): `MS:1000045` collision energy (eV), `MS:1000138` normalized collision energy (%), `MS:1000509` activation energy (eV).
    pub energy: Option<(String, f64)>,
    /// `id` of the spectrum the ion was selected from (the parent in an MSn chain).
    pub spectrum_ref: Option<String>,
}

/// The dissociation term of a method named as the file formats name it (`CID`, `HCD`, ...); `None` for a name without a PSI-MS term.
pub fn attivazione_da_nome(nome: &str) -> Option<(&'static str, &'static str)> {
    Some(match nome.trim().to_ascii_uppercase().as_str() {
        "CID" => ("MS:1000133", "collision-induced dissociation"),
        "HCD" => ("MS:1000422", "beam-type collision-induced dissociation"),
        "ETD" => ("MS:1000598", "electron transfer dissociation"),
        "ECD" => ("MS:1000250", "electron capture dissociation"),
        "PQD" => ("MS:1000599", "pulsed q dissociation"),
        "IRMPD" => ("MS:1000262", "infrared multiphoton dissociation"),
        _ => return None,
    })
}

/// (name, unit accession, unit name) of the energy terms a precursor can carry; an accession not listed here is not written.
fn energy_term(accession: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match accession {
        "MS:1000045" => Some(("collision energy", "UO:0000266", "electronvolt")),
        "MS:1000138" => Some(("normalized collision energy", "UO:0000187", "percent")),
        "MS:1000509" => Some(("activation energy", "UO:0000266", "electronvolt")),
        _ => None,
    }
}

/// The `<precursorList>` of `list` (empty for none).
#[cfg(feature = "precursori")]
pub(super) fn precursor_list(list: &[Precursore]) -> String {
    if list.is_empty() {
        return String::new();
    }
    let mut s = String::from("<precursorList count=\"");
    s += &list.len().to_string();
    s += "\">";
    for p in list {
        s += "<precursor";
        if let Some(r) = &p.spectrum_ref {
            s += " spectrumRef=\"";
            s += &esc(r);
            s += "\"";
        }
        s += ">";
        if p.iso_target.is_some() || p.iso_lower.is_some() || p.iso_upper.is_some() {
            s += "<isolationWindow>";
            if let Some(v) = p.iso_target {
                s += &mz_cv("MS:1000827", "isolation window target m/z", v);
            }
            if let Some(v) = p.iso_lower {
                s += &mz_cv("MS:1000828", "isolation window lower offset", v);
            }
            if let Some(v) = p.iso_upper {
                s += &mz_cv("MS:1000829", "isolation window upper offset", v);
            }
            s += "</isolationWindow>";
        }
        if let Some(mz) = p.selected_mz {
            s += "<selectedIonList count=\"1\"><selectedIon>";
            s += &mz_cv("MS:1000744", "selected ion m/z", mz);
            if let Some(z) = p.charge {
                s += &cv("MS:1000041", "charge state", z.to_string());
            }
            if let Some(i) = p.intensity {
                s += &cv("MS:1000042", "peak intensity", i);
            }
            s += "</selectedIon></selectedIonList>";
        }
        s += "<activation>";
        for (a, n) in &p.activation {
            s += &cv(a, n, "");
        }
        if let Some((a, v)) = &p.energy {
            if let Some((n, ua, un)) = energy_term(a) {
                s += &unit_cv_str(a, n, &v.to_string(), "UO", ua, un);
            }
        }
        s += "</activation></precursor>";
    }
    s + "</precursorList>"
}
