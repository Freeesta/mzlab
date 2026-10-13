use super::*;

fn run(xml: &str) -> String {
    let mut out = Vec::new();
    {
        let mut f = MzmlFilter::new(&mut out);
        for part in xml.as_bytes().chunks(7) {
            f.write_all(part).unwrap(); // pieces cut inside the lines
        }
        f.flush().unwrap();
    }
    String::from_utf8(out).unwrap()
}

fn spectrum(scan: u32, level: u32, parent: Option<u32>) -> String {
    let p = parent.map_or(String::new(), |p| format!("<precursor spectrumRef=\"controllerType=0 controllerNumber=1 scan={p}\">\n<selectedIon/>\n</precursor>\n"));
    format!("<spectrum id=\"controllerType=0 controllerNumber=1 scan={scan}\" index=\"0\">\n<cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>\n{p}</spectrum>\n")
}

#[test]
fn a_precursor_that_points_to_scan_1_points_to_the_survey_scan_before_it() {
    let xml = [
        spectrum(1, 1, None),
        spectrum(2, 2, Some(1)),
        spectrum(3, 2, Some(1)),
        spectrum(4, 1, None),
        spectrum(5, 2, Some(1)),
        spectrum(6, 3, Some(1)),
    ]
    .concat();
    let got = run(&xml);
    let refs: Vec<u32> = got
        .lines()
        .filter(|l| l.contains("spectrumRef"))
        .map(|l| number_after(l, "scan=").unwrap())
        .collect();
    assert_eq!(refs, [1, 1, 4, 5]); // the MS2 of scan 5 follows the survey scan 4, the MS3 its MS2 (scan 5)
}

#[test]
fn a_real_reference_is_kept() {
    let xml = [
        spectrum(1, 1, None),
        spectrum(2, 1, None),
        spectrum(3, 1, None),
        spectrum(4, 2, Some(2)),
    ]
    .concat();
    let got = run(&xml);
    assert!(got.contains("scan=2\">"), "a reference other than 1 stays");
    assert_eq!(got.lines().count(), xml.lines().count());
}

#[test]
fn the_extra_lines_go_right_after_the_run_tag_once() {
    let xml = "<mzML>\n<run id=\"r\">\n<spectrumList count=\"0\"/>\n</run>\n";
    let mut out = Vec::new();
    {
        let mut f = MzmlFilter::new(&mut out).with_run_extra("<userParam name=\"x\"/>".into());
        f.write_all(xml.as_bytes()).unwrap();
        f.flush().unwrap();
    }
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "<mzML>\n<run id=\"r\">\n<userParam name=\"x\"/>\n<spectrumList count=\"0\"/>\n</run>\n"
    );
}

#[test]
fn the_intensity_array_of_a_pressure_chromatogram_gets_the_pascal_unit() {
    let array =
        "<cvParam cvRef=\"MS\" accession=\"MS:1000515\" name=\"intensity array\" value=\"\"/>\n";
    let tic = format!("<chromatogram id=\"TIC\">\n<cvParam cvRef=\"MS\" accession=\"MS:1000235\" name=\"tic\" value=\"\"/>\n{array}</chromatogram>\n");
    let pressure = format!("<chromatogram id=\"Pump Pressure 1\">\n<cvParam cvRef=\"MS\" accession=\"MS:1003019\" name=\"pressure chromatogram\" value=\"\"/>\n{array}</chromatogram>\n");
    let got = run(&format!("{tic}{pressure}"));
    assert_eq!(got.matches("unitName=\"pascal\"").count(), 1);
    assert!(got.find("unitName=\"pascal\"").unwrap() > got.find("Pump Pressure 1").unwrap());
    assert!(got.contains("unitAccession=\"UO:0000110\""));
}
