//! The dissociation methods the writer can write are the ones the scan table reads back.

use super::precursor_element;
use crate::attivazioni::ATTIVAZIONI;

#[cfg(feature = "precursori")]
#[test]
fn every_activation_the_writer_writes_is_named_back_by_the_reader() {
    use crate::mzml::Spectrum;
    use crate::mzmlwrite::{attivazione_da_nome, spectrum_head, Precursore, SpectrumExtra};
    for (name, acc, term) in ATTIVAZIONI {
        let p = Precursore {
            selected_mz: Some(300.25),
            activation: vec![(acc.into(), term.into())],
            ..Default::default()
        };
        assert_eq!(attivazione_da_nome(name), Some((acc, term)));
        assert_eq!(attivazione_da_nome(&name.to_lowercase()), Some((acc, term)));
        let sp = Spectrum {
            level: 2,
            mz: vec![100.0],
            intensity: vec![1.0],
            ..Default::default()
        };
        let xml = spectrum_head(
            0,
            &sp,
            &SpectrumExtra {
                precursors: vec![p],
                ..Default::default()
            },
        );
        assert_eq!(precursor_element(&xml, None).act, Some(name), "{name}");
    }
}

#[test]
fn an_accession_that_only_starts_like_a_term_is_not_that_term() {
    let e = "<activation><cvParam cvRef=\"MS\" accession=\"MS:10001360\" name=\"x\" value=\"\"/><cvParam cvRef=\"MS\" accession=\"MS:1000598\" name=\"electron transfer dissociation\" value=\"\"/></activation>";
    assert_eq!(precursor_element(e, None).act, Some("ETD"));
    assert_eq!(
        precursor_element(
            "<activation><cvParam accession=\"MS:1000136\" value=\"\"/></activation>",
            None
        )
        .act,
        Some("SID")
    );
    assert_eq!(
        precursor_element("<activation></activation>", None).act,
        None
    );
}
