use super::*;

fn method() -> LcMethod {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let row = |module: &str,
               kind: &str,
               table: Option<(&str, usize)>,
               name: &str,
               v: &str,
               unit: &str| {
        let t = table
            .map(|(t, r)| format!("<TableName>{t}</TableName><RowIndex>{r}</RowIndex>"))
            .unwrap_or_default();
        format!("<Section><ModuleDisplayName>{module}</ModuleDisplayName><DeviceType>{kind}</DeviceType>{t}<ParameterName>{name}</ParameterName><ParameterValue>{v}</ParameterValue><ParameterUnit>{unit}</ParameterUnit></Section>")
    };
    let mut rc = String::new();
    for (r, (t, b, f)) in [("0.00", "5", "0.4"), ("10.00", "95", "0.4")]
        .iter()
        .enumerate()
    {
        let tt = Some(("Timetable", r));
        rc += &row("Binary Pump", "Binary Pump", tt, "Time", t, "min");
        rc += &row("Binary Pump", "Binary Pump", tt, "B", b, "%");
        rc += &row("Binary Pump", "Binary Pump", tt, "Flow", f, "mL/min");
    }
    rc += &row(
        "Binary Pump",
        "Binary Pump",
        None,
        "Flow",
        "0.400",
        "mL/min",
    );
    rc += &row(
        "Column Comp.",
        "Column Comp.",
        None,
        "Temperature",
        "40.00",
        "°C",
    );
    rc += &row(
        "DAD",
        "DAD",
        None,
        "Peakwidth",
        ">0.05 min (1.0 s response time) (5 Hz)",
        "",
    );
    rc += &row("DAD", "DAD", None, "UV Lamp Required", "Yes", "");
    for (name, v, unit) in [
        ("Use Sig.", "Yes", ""),
        ("Wavelength", "254.0", "nm"),
        ("Use Ref.", "Yes", ""),
        ("Ref Wavel.", "360.0", "nm"),
    ] {
        rc += &row("DAD", "DAD", Some(("Signal table", 0)), name, v, unit);
    }
    let cell = |r: usize, c: usize, col: &str, v: &str| {
        format!("<SectionInfo><SectionType>Column</SectionType><Name>{col}</Name><RowIndex>{r}</RowIndex><ColumnIndex>{c}</ColumnIndex><Value>{v}</Value></SectionInfo>")
    };
    let mut ms = String::from("<SectionInfo><SectionType>Repeater</SectionType><Name>Time Segment 1</Name></SectionInfo><SectionInfo><SectionType>Table</SectionType><Name>Scan Segments</Name></SectionInfo>");
    for (r, (q1, q3)) in [("399.1", "250.2"), ("399.1", "180.0")].iter().enumerate() {
        ms += &cell(r, 0, "Prec Ion", q1);
        ms += &cell(r, 1, "Prod Ion", q3);
        ms += &cell(r, 2, "Dwell", "50");
        ms += &cell(r, 3, "CE (V)", "20");
        ms += &cell(r, 4, "Ret Time (min)", "2.5");
    }
    let text = format!(
        "<R><RCDevicesXml>{}</RCDevicesXml><SCICDevicesXml>{}</SCICDevicesXml></R>",
        esc(&format!("<D>{rc}</D>")),
        esc(&format!("<N>{ms}</N>"))
    );
    LcMethod::parse(&text).unwrap()
}

#[test]
fn gradient_oven_and_detector_come_from_the_rows() {
    let m = method();
    let doc = lc(&m).unwrap().json();
    assert!(doc.contains("\"forno_C\":40"), "{doc}");
    assert!(doc.contains("\"flusso_mL_min\":0.4"), "{doc}");
    assert!(
        doc.contains("{\"t_min\":10,\"B_pct\":95,\"flusso_mL_min\":0.4}"),
        "{doc}"
    );
    let det = V::Arr(detectors(&m)).json();
    assert!(det.contains("\"frequenza_Hz\":5"), "{det}");
    assert!(det.contains("\"lampada\":\"UV\""), "{det}");
    assert!(det.contains("\"riferimento_nm\":360"), "{det}");
}

#[test]
fn the_scan_segments_become_transitions() {
    let m = method();
    let t = transitions(m.table("Scan Segments", Some(1)).unwrap());
    assert_eq!(t.len(), 2);
    let j = V::Arr(t).json();
    assert!(j.contains("\"q1\":399.1,\"q3\":180"), "{j}");
    assert!(j.contains("\"rt_min\":2.5"), "{j}");
    // the cells are carried by the transitions: not repeated in campi
    assert!(items(&m).iter().all(|i| !i.name.contains("Scan Segments[")));
}
