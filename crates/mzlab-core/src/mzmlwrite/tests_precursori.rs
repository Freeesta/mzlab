use super::*;

fn ms3() -> (Spectrum, SpectrumExtra) {
    let sp = Spectrum {
        index: 2,
        id: "scan=3".into(),
        level: 3,
        rt: 1.5,
        polarity: 1,
        profile: false,
        precursor: Some(300.25),
        mz: vec![100.5, 200.25],
        intensity: vec![10.0, 40.0],
        filter: "ITMS + c ESI d Full ms3".into(),
        ..Default::default()
    };
    let extra = SpectrumExtra {
        precursors: vec![Precursore {
            selected_mz: Some(300.25),
            charge: Some(2),
            intensity: Some(1234.5),
            iso_target: Some(300.5),
            iso_lower: Some(1.5),
            iso_upper: Some(0.5),
            activation: vec![(
                "MS:1000422".into(),
                "beam-type collision-induced dissociation".into(),
            )],
            energy: Some(("MS:1000138".into(), 35.0)),
            spectrum_ref: Some("scan=2".into()),
        }],
        tic: Some(999.0),
        base_peak: Some((200.25, 77.0)),
        scan_params: vec![CvPar::new("MS:1000927", "ion injection time", 12.5)
            .with_unit("UO:0000028", "millisecond")],
        window: Some((50.0, 600.0)),
        ..Default::default()
    };
    (sp, extra)
}

#[test]
fn the_chain_of_an_ms3_is_written_with_its_parent_window_charge_and_activation() {
    let (sp, extra) = ms3();
    let head = spectrum_head(2, &sp, &extra);
    let expected = concat!(
        "<spectrum index=\"2\" id=\"scan=3\" defaultArrayLength=\"2\">",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"3\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000580\" name=\"MSn spectrum\" value=\"\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000130\" name=\"positive scan\" value=\"\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000127\" name=\"centroid spectrum\" value=\"\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000504\" name=\"base peak m/z\" value=\"200.25\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000505\" name=\"base peak intensity\" value=\"77\" unitCvRef=\"MS\" unitAccession=\"MS:1000131\" unitName=\"number of detector counts\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000285\" name=\"total ion current\" value=\"999\"/>",
        "<scanList count=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000795\" name=\"no combination\" value=\"\"/><scan>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"1.5\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000512\" name=\"filter string\" value=\"ITMS + c ESI d Full ms3\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000927\" name=\"ion injection time\" value=\"12.5\" unitCvRef=\"UO\" unitAccession=\"UO:0000028\" unitName=\"millisecond\"/>",
        "<scanWindowList count=\"1\"><scanWindow>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000501\" name=\"scan window lower limit\" value=\"50\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000500\" name=\"scan window upper limit\" value=\"600\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "</scanWindow></scanWindowList></scan></scanList>",
        "<precursorList count=\"1\"><precursor spectrumRef=\"scan=2\"><isolationWindow>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000827\" name=\"isolation window target m/z\" value=\"300.5\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000828\" name=\"isolation window lower offset\" value=\"1.5\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000829\" name=\"isolation window upper offset\" value=\"0.5\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "</isolationWindow><selectedIonList count=\"1\"><selectedIon>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000744\" name=\"selected ion m/z\" value=\"300.25\" unitCvRef=\"MS\" unitAccession=\"MS:1000040\" unitName=\"m/z\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000041\" name=\"charge state\" value=\"2\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000042\" name=\"peak intensity\" value=\"1234.5\"/>",
        "</selectedIon></selectedIonList><activation>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000422\" name=\"beam-type collision-induced dissociation\" value=\"\"/>",
        "<cvParam cvRef=\"MS\" accession=\"MS:1000138\" name=\"normalized collision energy\" value=\"35\" unitCvRef=\"UO\" unitAccession=\"UO:0000187\" unitName=\"percent\"/>",
        "</activation></precursor></precursorList>",
    );
    assert_eq!(head, expected);
}

#[test]
fn a_spectrum_without_precursors_keeps_the_plain_precursor_and_the_totals_from_its_points() {
    let sp = Spectrum {
        index: 0,
        id: "scan=2".into(),
        level: 2,
        rt: 1.0,
        polarity: 1,
        precursor: Some(229.1),
        mz: vec![100.0, 101.0],
        intensity: vec![3.0, 5.0],
        ..Default::default()
    };
    let head = spectrum_head(0, &sp, &SpectrumExtra::default());
    assert!(head.contains("<precursorList count=\"1\"><precursor><isolationWindow><cvParam cvRef=\"MS\" accession=\"MS:1000827\" name=\"isolation window target m/z\" value=\"229.1\""));
    assert!(head.contains("name=\"selected ion m/z\" value=\"229.1\""));
    assert!(head.contains("name=\"total ion current\" value=\"8\"/>"));
    assert!(head.contains("name=\"base peak m/z\" value=\"101\""));
    assert!(!head.contains("activation"));
}

#[test]
fn the_written_file_with_a_chain_validates_and_reads_back() {
    let (sp, extra) = ms3();
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "x".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(1).unwrap();
    w.spectrum_with(&sp, &extra).unwrap();
    w.close_spectra().unwrap();
    let bytes = w.finish().unwrap();
    let run = crate::mzml::read(&bytes[..]).unwrap();
    assert_eq!(
        (run.spectra[0].level, run.spectra[0].precursor),
        (3, Some(300.25))
    );
    let xml = String::from_utf8(bytes).unwrap();
    assert!(xml.contains("version=\"4.2.6\"") && !xml.contains("4.1.163"));
}

#[test]
fn the_indexed_file_with_a_chain_validates_against_the_psi_schema() {
    let (ms3, extra3) = ms3();
    let ms1 = Spectrum {
        index: 0,
        id: "scan=1".into(),
        level: 1,
        rt: 0.5,
        polarity: 1,
        mz: vec![300.25],
        intensity: vec![5.0],
        ..Default::default()
    };
    let ms2 = Spectrum {
        index: 1,
        id: "scan=2".into(),
        level: 2,
        rt: 1.0,
        polarity: 1,
        precursor: Some(400.0),
        mz: vec![300.25],
        intensity: vec![5.0],
        ..Default::default()
    };
    let extra2 = SpectrumExtra {
        precursors: vec![Precursore {
            selected_mz: Some(400.0),
            spectrum_ref: Some("scan=1".into()),
            activation: vec![("MS:1000133".into(), "collision-induced dissociation".into())],
            energy: Some(("MS:1000045".into(), 25.0)),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(
        Vec::new(),
        &Header {
            run: "x".into(),
            ..Default::default()
        },
        Opzioni::default(),
    )
    .unwrap();
    w.open_spectra(3).unwrap();
    w.spectrum(&ms1).unwrap();
    w.spectrum_with(&ms2, &extra2).unwrap();
    w.spectrum_with(&ms3, &extra3).unwrap();
    w.close_spectra().unwrap();
    super::tests_xsd::valida(&w.finish().unwrap());
}
