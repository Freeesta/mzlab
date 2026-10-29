//! SCIEX scan modes as msconvert writes them in mzML: the SIM trace id, and the precursor of an MS3 spectrum with two `<precursor>`.

use mzlab_core::mzml::{self, ChromKind};

fn chrom(id: &str, term: &str) -> String {
    format!("<chromatogram index=\"0\" id=\"{id}\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"{term}\" name=\"x\"/></chromatogram>")
}

fn precursor(mz: f64) -> String {
    format!("<precursor><selectedIonList count=\"1\"><selectedIon><cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"{mz}\"/></selectedIon></selectedIonList></precursor>")
}

fn read(body: &str) -> mzml::Run {
    let xml = format!("<?xml version=\"1.0\"?><mzML xmlns=\"http://psi.hupo.org/ms/mzml\"><run id=\"r\">{body}</run></mzML>");
    mzml::read(xml.as_bytes()).unwrap()
}

#[test]
fn sim_sic_id_is_a_sic_with_its_q1() {
    let run = read(&format!(
        "<chromatogramList count=\"2\">{}{}</chromatogramList>",
        chrom(
            "SIM SIC Q1=228.1 sample=1 period=1 experiment=2",
            "MS:1000627"
        ),
        chrom("SIC 301.5", "MS:1000627")
    ));
    assert_eq!(run.chromatograms[0].kind, ChromKind::Sic);
    assert_eq!(run.chromatograms[0].q1, Some(228.1));
    assert_eq!(run.chromatograms[1].kind, ChromKind::Sic);
}

#[test]
fn ms3_with_two_precursors_is_level_3_with_the_last_stage() {
    let run = read(&format!(
        "<spectrumList count=\"1\"><spectrum index=\"0\" id=\"scan=1\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"3\"/><precursorList count=\"2\">{}{}</precursorList></spectrum></spectrumList>",
        precursor(300.0),
        precursor(150.0)
    ));
    assert_eq!(run.spectra[0].level, 3);
    assert_eq!(run.spectra[0].precursor, Some(150.0));
}
