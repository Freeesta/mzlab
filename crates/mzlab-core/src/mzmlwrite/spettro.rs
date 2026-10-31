//! What a spectrum header carries beyond the points: the precursor chain (`<precursorList>`) and the cvParams of `<spectrum>` and `<scan>`.
//! Used by `spectrum_head` and filled by the readers through `SpectrumExtra` (`Lettore::extra_mzml`, `Lettore::extra_scansione`).

use super::{cv_into, esc_into, mz_cv_into, unit_cv_str_into};

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
        CvPar {
            accession: accession.into(),
            name: name.into(),
            value: value.to_string(),
            unit: None,
        }
    }

    pub fn with_unit(mut self, accession: &str, name: &str) -> Self {
        self.unit = Some((accession.into(), name.into()));
        self
    }

    /// Appends the cvParam; the name and the value come from files and are escaped.
    pub(super) fn xml_into(&self, s: &mut String) {
        s.push_str("<cvParam cvRef=\"MS\" accession=\"");
        s.push_str(&self.accession);
        s.push_str("\" name=\"");
        esc_into(s, &self.name);
        s.push_str("\" value=\"");
        esc_into(s, &self.value);
        match &self.unit {
            Some((ua, un)) => {
                s.push_str("\" unitCvRef=\"");
                s.push_str(if ua.starts_with("UO:") { "UO" } else { "MS" });
                s.push_str("\" unitAccession=\"");
                s.push_str(ua);
                s.push_str("\" unitName=\"");
                s.push_str(un);
                s.push_str("\"/>");
            }
            None => s.push_str("\"/>"),
        }
    }
}

/// Appends the cvParams of `list` in order.
pub(super) fn params_into(s: &mut String, list: &[CvPar]) {
    for p in list {
        p.xml_into(s);
    }
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
    crate::attivazioni::da_nome(nome)
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

/// Appends the `<precursorList>` of `list` (nothing for none).
pub(super) fn precursor_list_into(s: &mut String, list: &[Precursore]) {
    if list.is_empty() {
        return;
    }
    s.reserve(768 * list.len() + 64);
    s.push_str("<precursorList count=\"");
    s.push_str(&list.len().to_string());
    s.push_str("\">");
    for p in list {
        s.push_str("<precursor");
        if let Some(r) = &p.spectrum_ref {
            s.push_str(" spectrumRef=\"");
            esc_into(s, r);
            s.push('"');
        }
        s.push('>');
        if p.iso_target.is_some() || p.iso_lower.is_some() || p.iso_upper.is_some() {
            s.push_str("<isolationWindow>");
            if let Some(v) = p.iso_target {
                mz_cv_into(s, "MS:1000827", "isolation window target m/z", v);
            }
            if let Some(v) = p.iso_lower {
                mz_cv_into(s, "MS:1000828", "isolation window lower offset", v);
            }
            if let Some(v) = p.iso_upper {
                mz_cv_into(s, "MS:1000829", "isolation window upper offset", v);
            }
            s.push_str("</isolationWindow>");
        }
        if let Some(mz) = p.selected_mz {
            s.push_str("<selectedIonList count=\"1\"><selectedIon>");
            mz_cv_into(s, "MS:1000744", "selected ion m/z", mz);
            if let Some(z) = p.charge {
                cv_into(s, "MS:1000041", "charge state", z);
            }
            if let Some(i) = p.intensity {
                cv_into(s, "MS:1000042", "peak intensity", i);
            }
            s.push_str("</selectedIon></selectedIonList>");
        }
        s.push_str("<activation>");
        for (a, n) in &p.activation {
            cv_into(s, a, n, "");
        }
        if let Some((a, v)) = &p.energy
            && let Some((n, ua, un)) = energy_term(a)
        {
            unit_cv_str_into(s, a, n, &v.to_string(), "UO", ua, un);
        }
        s.push_str("</activation></precursor>");
    }
    s.push_str("</precursorList>");
}
