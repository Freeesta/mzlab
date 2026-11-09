use super::*;

const SCRIPT: &str = "---- Overview ----\nName: M\nRun time: 15.000 [min]\n---- Script ----\ninitial     Instrument Setup\n            ColumnComp.CC.Temperature.Nominal: 40.00 [°C]\n            PumpModule.Pump.Pressure.UpperLimit: 14504 [psi]\n            PumpModule.Pump.%A1_Equate: \"Water\"\n0.000 [min] Run\n            PumpModule.Pump.Flow.Nominal: 0.200 [ml/min]\n            PumpModule.Pump.%B.Value: 10.0 [%]\n4.000 [min]\n            PumpModule.Pump.%B.Value: 60.0 [%]\n";

#[test]
fn blocks_are_told_apart() {
    assert_eq!(classify(SCRIPT), Block::Script);
    assert_eq!(
        classify("<?xml version=\"1.0\"?><InstrumentSetupMethod/>"),
        Block::Xml
    );
    assert_eq!(
        classify("Method of X\nOVERALL METHOD SETTINGS"),
        Block::Text
    );
    assert_eq!(
        xml_of("text <?xml version=\"1.0\"?><a/>"),
        "<?xml version=\"1.0\"?><a/>"
    );
}

#[test]
fn script_lines_become_key_value_unit_with_the_time_of_their_row() {
    let p = script_pairs(SCRIPT);
    let find = |k: &str, t: Option<f64>| p.iter().find(|x| x.1 == k && x.0 == t).cloned();
    assert_eq!(
        find("ColumnComp.CC.Temperature.Nominal", None).map(|x| (x.2, x.3)),
        Some(("40.00".into(), Some("°C".into())))
    );
    assert_eq!(
        find("PumpModule.Pump.%A1_Equate", None).map(|x| x.2),
        Some("Water".into())
    );
    assert_eq!(
        find("PumpModule.Pump.%B.Value", Some(4.0)).map(|x| x.2),
        Some("60.0".into())
    );
    assert_eq!(
        setting(&p, "PumpModule.Pump.Pressure.UpperLimit"),
        Some((14504.0, Some("psi".into())))
    );
    assert_eq!(setting(&p, "PumpModule.Pump.Flow.Nominal"), None); // only the settings before the first timed row
}

#[test]
fn the_scan_events_of_the_xml_keep_their_numbers_and_units() {
    let xml = "<?xml version=\"1.0\" encoding=\"utf-16\"?><M><ScanEvent id=\"1\"><First_mass unit=\"m/z\">100</First_mass><Last_mass unit=\"m/z\">1000</Last_mass><Resolution>35000</Resolution></ScanEvent><ScanEvent id=\"2\"><Resolution>17500</Resolution><Name>x</Name></ScanEvent></M>";
    let e = experiments(&xml_items(xml));
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].get("indice"), Some(&V::Num(1.0)));
    assert_eq!(
        e[0].get("mz"),
        Some(&V::Arr(vec![V::Num(100.0), V::Num(1000.0)]))
    );
    assert!(e[1].get("mz").is_none());
    let n = |e: &V| match e.get("parametri") {
        Some(V::Arr(a)) => a.len(),
        _ => 0,
    };
    assert_eq!((n(&e[0]), n(&e[1])), (1, 1)); // the range is `mz`; the text of `Name` is no parameter
}

#[test]
fn the_lc_of_a_script_is_the_model_of_the_dam() {
    let lc = lc_of(SCRIPT).unwrap();
    assert_eq!(
        (lc.oven, lc.flow, lc.b0),
        (Some(40.0), Some(0.2), Some(10.0))
    );
    assert_eq!(lc.gradient.len(), 2);
}
