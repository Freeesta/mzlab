use super::*;

fn run(xml: &str, anonymous: bool) -> String {
    let mut out = Vec::new();
    {
        let mut f = MzmlFilter::new(&mut out, anonymous);
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
    let got = run(&xml, false);
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
    let got = run(&xml, false);
    assert!(got.contains("scan=2\">"), "a reference other than 1 stays");
    assert_eq!(got.lines().count(), xml.lines().count());
}

#[test]
fn the_extra_lines_go_right_after_the_run_tag_once() {
    let xml = "<mzML>\n<run id=\"r\">\n<spectrumList count=\"0\"/>\n</run>\n";
    let mut out = Vec::new();
    {
        let mut f =
            MzmlFilter::new(&mut out, false).with_run_extra("<userParam name=\"x\"/>".into());
        f.write_all(xml.as_bytes()).unwrap();
        f.flush().unwrap();
    }
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "<mzML>\n<run id=\"r\">\n<userParam name=\"x\"/>\n<spectrumList count=\"0\"/>\n</run>\n"
    );
}

#[test]
fn personal_data_are_left_out_only_when_asked() {
    let xml = "<run>\n    <userParam name=\"opentfraw.computer_name\" value=\"X\"/>\n    <userParam name=\"opentfraw.raw_version\" value=\"66\"/>\n    <userParam name=\"opentfraw.original_file_path\" value=\"C:\\a\"/>\n    <userParam name=\"opentfraw.user_label.0.heading\" value=\"Study\"/>\n    <userParam name=\"opentfraw.user_label.0.value\" value=\"P\"/>\n</run>\n";
    assert_eq!(run(xml, false), xml);
    let got = run(xml, true);
    assert!(
        !got.contains("computer_name")
            && !got.contains("original_file_path")
            && !got.contains("label.0.value")
    );
    assert!(got.contains("raw_version") && got.contains("label.0.heading"));
}
