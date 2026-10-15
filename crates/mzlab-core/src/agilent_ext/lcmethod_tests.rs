use super::*;

#[test]
fn unescape_entities() {
    assert_eq!(
        unescape("a &lt;b&gt; &amp;&#228;&#xE4; &bogus; &"),
        "a <b> &ää &bogus; &"
    );
}

#[test]
fn the_two_documents_become_rows() {
    let rc = "<DSRdlReport><Section><ModuleDisplayName>Binary Pump</ModuleDisplayName><DeviceType>Binary Pump</DeviceType><ParentSectionName>Stop Time</ParentSectionName><ParameterName>Stoptime</ParameterName><ParameterValue>10.00</ParameterValue><ParameterUnit>min</ParameterUnit></Section></DSRdlReport>";
    let ms = "<NewDataSet><SectionInfo><SectionType>Repeater</SectionType><Name>Time Segment 1</Name></SectionInfo><SectionInfo><SectionType>Table</SectionType><Name>Scan Segments</Name></SectionInfo><SectionInfo><SectionType>Column</SectionType><Name>Prec Ion</Name><RowIndex>0</RowIndex><ColumnIndex>0</ColumnIndex><Value>399.1</Value></SectionInfo><SectionInfo><SectionType>Column</SectionType><Name>Prod Ion</Name><RowIndex>1</RowIndex><ColumnIndex>1</ColumnIndex><Value>250.2</Value></SectionInfo><SectionInfo><SectionType>Parameter</SectionType><Name>Ion Source</Name><Value>ESI</Value></SectionInfo></NewDataSet>";
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let text = format!(
        "<DataFileMethodReport><MethodReport><MethodName>a.m</MethodName><RCDevicesXml>{}</RCDevicesXml><SCICDevicesXml>{}</SCICDevicesXml></MethodReport></DataFileMethodReport>",
        esc(rc),
        esc(ms)
    );
    let m = LcMethod::parse(&text).unwrap();
    assert_eq!(m.name.as_deref(), Some("a.m"));
    assert_eq!(m.rc[0].name, "Stoptime");
    assert_eq!(m.rc[0].section, "Stop Time");
    let t = m.table("Scan Segments", Some(1)).unwrap();
    assert_eq!(t.rows.len(), 2);
    assert_eq!(t.cell(0, "Prec Ion"), Some("399.1"));
    assert_eq!(t.cell(1, "Prod Ion"), Some("250.2"));
    assert_eq!(t.cell(0, "Prod Ion"), None);
    assert_eq!(m.param("Ion Source"), Some("ESI"));
    assert!(LcMethod::parse("<x/>").is_none());
}
