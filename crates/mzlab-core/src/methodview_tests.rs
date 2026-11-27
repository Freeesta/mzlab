use super::*;
use crate::json::parse;

fn view(doc: &str) -> Value {
    let v = parse(doc.as_bytes()).unwrap();
    let mut o = Out::default();
    o.raw("{\"lc\":");
    write_lc(&mut o, Some(&v));
    o.raw(",\"dad\":");
    write_dad(&mut o, Some(&v));
    o.raw("}");
    parse(o.0.as_bytes()).unwrap()
}

#[test]
fn display_rounding_follows_the_shared_corpus() {
    let corpus = parse(include_str!("../../../tests/data/shown_numbers.json").as_bytes()).unwrap();
    for row in corpus.arr() {
        let (v, want) = (
            row.at(0).unwrap().f64().unwrap(),
            row.at(1).unwrap().str().unwrap(),
        );
        assert_eq!(shown(v), want, "shown({v})");
    }
}

// a program whose last event is at 20 min and whose run ends at 27.4998 min (the stop event of a Shimadzu method): the last %B is held to the end
const LC: &str = r#"{"schema":"method/1","lc":{"gradient":[{"t_min":0,"B_pct":5,"flow_mL_min":0.4},{"t_min":20,"B_pct":95,"flow_mL_min":0.4}],"flow_mL_min":0.4,"oven_C":40,"duration_min":27.499780654907227,
"sources":{"gradient":{"kind":"read","at":"stream A/field B"},"run_time":{"kind":"read","at":"stream A/Stop"}}}}"#;

#[test]
fn the_last_percent_b_is_held_to_the_end_of_the_run_and_marked_as_inferred() {
    let v = view(LC);
    let lc = v.get("lc").unwrap();
    let h = lc.get("hold").unwrap();
    assert_eq!(h.get("from_min").unwrap().f64(), Some(20.0));
    assert_eq!(h.get("to_min").unwrap().f64(), Some(27.499780654907227));
    assert_eq!(h.get("B_pct").unwrap().f64(), Some(95.0));
    assert_eq!(
        h.get("shown").unwrap().get("to").unwrap().str(),
        Some("27.50")
    );
    assert_eq!(lc.get("run_time_shown").unwrap().str(), Some("27.50"));
    assert_eq!(
        lc.get("run_time_min").unwrap().f64(),
        Some(27.499780654907227),
        "the full value stays"
    );
    assert_eq!(
        lc.get("gradient").unwrap().arr().len(),
        2,
        "the hold is not a point of the file"
    );
    assert_eq!(
        lc.get("sources")
            .unwrap()
            .get("hold")
            .unwrap()
            .get("kind")
            .unwrap()
            .str(),
        Some("inferred")
    );
    assert_eq!(
        lc.get("sources")
            .unwrap()
            .get("a_pct")
            .unwrap()
            .get("kind")
            .unwrap()
            .str(),
        Some("inferred")
    );
    let p = lc
        .get("gradient")
        .unwrap()
        .at(1)
        .unwrap()
        .get("shown")
        .unwrap();
    assert_eq!(
        (
            p.get("t").unwrap().str(),
            p.get("a").unwrap().str(),
            p.get("b").unwrap().str(),
            p.get("flow").unwrap().str()
        ),
        (Some("20.00"), Some("5.00"), Some("95.00"), Some("0.40"))
    );
}

#[test]
fn a_program_that_ends_at_the_run_time_has_no_hold_and_the_eluents_are_never_invented() {
    let v = view(
        r#"{"lc":{"gradient":[{"t_min":0,"B_pct":5},{"t_min":10,"B_pct":95}],"duration_min":10}}"#,
    );
    let lc = v.get("lc").unwrap();
    assert!(lc.get("hold").unwrap().is_null());
    assert_eq!(
        lc.get("eluents").unwrap().arr().len(),
        0,
        "no composition unless the file names it"
    );
    assert!(
        lc.get("sources")
            .unwrap()
            .get("gradient")
            .unwrap()
            .is_null(),
        "a reader that names no stream gives no source"
    );
    let t = view(
        r#"{"lc":{"gradient":[{"t_min":0,"B_pct":5},{"t_min":10,"B_pct":95}],"solvents":[{"channel":"A","name":"named in the file"}]}}"#,
    );
    assert_eq!(
        t.get("lc")
            .unwrap()
            .get("eluents")
            .unwrap()
            .at(0)
            .unwrap()
            .get("name")
            .unwrap()
            .str(),
        Some("named in the file")
    );
}

#[test]
fn no_lc_and_no_dad_are_null() {
    let v = view(r#"{"schema":"method/1"}"#);
    assert!(v.get("lc").unwrap().is_null() && v.get("dad").unwrap().is_null());
}

#[test]
fn the_dad_header_becomes_labelled_rows_with_its_stream_as_the_source() {
    const P: &str = "SampleSubtree/Sample1/Devices/Device_1/DAD/DADSampleHeader";
    let doc = format!(
        r#"{{"fields":[{{"origin":"{P}","key":"points","value":1651}},{{"origin":"{P}","key":"channels","value":351}},{{"origin":"{P}","key":"duration","value":27.499780654907227,"unit":"min"}},{{"origin":"{P}","key":"wavelength_min","value":250,"unit":"nm"}},{{"origin":"{P}","key":"wavelength_max","value":600,"unit":"nm"}},{{"origin":"{P}","key":"step","value":1,"unit":"nm"}}]}}"#
    );
    let d = view(&doc);
    let rows = d.get("dad").unwrap().get("rows").unwrap().arr();
    let by = |id: &str| {
        rows.iter()
            .find(|r| r.get("id").unwrap().str() == Some(id))
            .unwrap()
    };
    assert_eq!(by("points").get("shown").unwrap().str(), Some("1651"));
    assert_eq!(by("duration").get("shown").unwrap().str(), Some("27.50"));
    assert_eq!(
        by("range").get("shown").unwrap().str(),
        Some("250.00\u{2013}600.00")
    );
    assert_eq!(by("step").get("unit").unwrap().str(), Some("nm"));
    assert_eq!(
        by("step").get("source").unwrap().get("at").unwrap().str(),
        Some(format!("{P} / step").as_str())
    );
    assert_eq!(
        by("duration")
            .get("source")
            .unwrap()
            .get("kind")
            .unwrap()
            .str(),
        Some("read")
    );
}

#[test]
fn a_method_detector_adds_what_the_header_lacks_and_keeps_its_own_source() {
    let v = view(
        r#"{"detectors":[{"type":"PDA","nm":[190,800],"step_nm":2,"frequency_Hz":10,"lamp":"D2","source":{"kind":"read","at":"VendorAppMethod/PDA Detector"}}]}"#,
    );
    let rows = v.get("dad").unwrap().get("rows").unwrap().arr();
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r.get("id").unwrap().str().unwrap())
        .collect();
    assert_eq!(ids, ["range", "step", "frequency", "lamp"]);
    assert_eq!(
        rows[0].get("source").unwrap().get("at").unwrap().str(),
        Some("VendorAppMethod/PDA Detector")
    );
}

#[test]
fn the_view_of_a_dam_program_carries_the_source_the_reader_named() {
    use crate::methodinfo::{GPoint, Lc, Pda, Src};
    let lc = Lc {
        gradient: vec![
            GPoint {
                t: 0.0,
                b: Some(5.0),
                flow: Some(0.4),
            },
            GPoint {
                t: 12.5,
                b: Some(80.0),
                flow: Some(0.4),
            },
        ],
        flow: Some(0.4),
        b0: Some(5.0),
        run_time: Some(15.0),
        pda: Pda::default(),
        channels: vec![],
        oven: None,
        src: vec![
            (
                "gradient",
                Src::read("DeviceMethod1/LCPumpStepK/LCPumpStep"),
            ),
            ("run_time", Src::inferred("time of the last pump step")),
        ],
    };
    let mut o = Out::default();
    write_view_of(&mut o, &lc);
    let v = parse(o.0.as_bytes()).unwrap();
    let l = v.get("lc").unwrap();
    assert_eq!(
        l.get("sources")
            .unwrap()
            .get("gradient")
            .unwrap()
            .get("at")
            .unwrap()
            .str(),
        Some("DeviceMethod1/LCPumpStepK/LCPumpStep")
    );
    assert_eq!(
        l.get("hold")
            .unwrap()
            .get("shown")
            .unwrap()
            .get("from")
            .unwrap()
            .str(),
        Some("12.50")
    );
    assert!(v.get("dad").unwrap().is_null());
}
