use super::*;

const GROUPS: &str = r#"<mzML><referenceableParamGroupList count="2">
<referenceableParamGroup id="ms2neg"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/><cvParam cvRef="MS" accession="MS:1000129" name="negative scan"/>
<cvParam cvRef="MS" accession="MS:1000128" name="profile spectrum"/></referenceableParamGroup>
<referenceableParamGroup id="f32z"><cvParam cvRef="MS" accession="MS:1000521" name="32-bit float"/><cvParam cvRef="MS" accession="MS:1000574" name="zlib compression"/></referenceableParamGroup>
</referenceableParamGroupList><run id="r">"#;

#[test]
fn quotes_order_entities_and_prefixes_do_not_matter() {
    let h = parse(
        r#"<m:spectrum xmlns:m="http://psi.hupo.org/ms/mzml" id='scan=1 &amp; b' index = "0" defaultArrayLength="4">
<m:cvParam value='2' name="ms level" accession='MS:1000511' cvRef="MS"/><!-- a comment <cvParam accession="MS:1000130"/> -->
<m:scanList><m:scan><m:cvParam unitName='minute' value="5.5" accession="MS:1000016" unitAccession='UO:0000031'/></m:scan></m:scanList>"#,
        &Groups::new(),
    )
    .unwrap();
    assert_eq!(h.id, "scan=1 & b");
    assert_eq!(
        (h.index, h.length, h.level(), h.polarity()),
        (Some(0), Some(4), 2, 0)
    );
    assert_eq!(h.rt(), Some((5.5, RtUnit::Minute)));
}

#[test]
fn groups_resolve_for_the_spectrum_and_its_arrays() {
    let g = groups(GROUPS);
    let h = parse(
        r#"<spectrum id="s" defaultArrayLength="2"><referenceableParamGroupRef ref="ms2neg"/>
<binaryDataArrayList count="1"><binaryDataArray encodedLength="4"><referenceableParamGroupRef ref="f32z"/><cvParam accession="MS:1000514" name="m/z array"/>
<binary><![CDATA[AAAA]]></binary></binaryDataArray></binaryDataArrayList></spectrum>"#,
        &g,
    )
    .unwrap();
    assert_eq!((h.level(), h.polarity(), h.profile()), (2, -1, true));
    let a = &h.arrays[0].terms;
    assert!(a.has("MS:1000521") && a.has("MS:1000574") && a.has("MS:1000514"));
}

#[test]
fn binary_text_and_cdata_are_kept_when_asked() {
    let g = Groups::new();
    let mut r = Reader::new(&g);
    r.keep_binary = true;
    let mut got = Vec::new();
    r.feed(
        r#"<spectrum id="a"><binaryDataArray><binary><![CDATA[AAAA]]></binary></binaryDataArray></spectrum><spectrum id="b"><binaryDataArray><binary>BB&#x41;A</binary></binaryDataArray></spectrum>"#,
        |h| got.push(h),
    );
    let texts: Vec<_> = got.iter().map(|h| h.arrays[0].text.as_str()).collect();
    assert_eq!(texts, ["AAAA", "BBAA"]);
}

#[test]
fn every_unit_of_a_retention_time_becomes_minutes() {
    for (unit, v, min) in [
        (r#"unitAccession="UO:0000031" unitName="minute""#, 1.5, 1.5),
        (r#"unitAccession="UO:0000010" unitName="second""#, 90.0, 1.5),
        (
            r#"unitAccession="UO:0000028" unitName="millisecond""#,
            120000.0,
            2.0,
        ),
        (r#"unitAccession="UO:0000032" unitName="hour""#, 0.5, 30.0),
        ("", 3.0, 3.0),
    ] {
        let h = parse(
            &format!(r#"<spectrum id="s"><scanList><scan><cvParam accession="MS:1000016" value="{v}" {unit}/></scan></scanList>"#),
            &Groups::new(),
        )
        .unwrap();
        assert!(
            (h.rt_minutes() - min).abs() < 1e-12,
            "{unit}: {}",
            h.rt_minutes()
        );
        assert_eq!(h.rt().unwrap().1 == RtUnit::Missing, unit.is_empty());
    }
}

#[test]
fn the_first_scan_gives_the_time_and_the_configuration() {
    let h = parse(
        r#"<spectrum id="s"><scanList count="2"><scan instrumentConfigurationRef="IC2"><cvParam accession="MS:1000016" value="1" unitName="minute"/></scan>
<scan instrumentConfigurationRef="IC3"><cvParam accession="MS:1000016" value="9" unitName="minute"/></scan></scanList></spectrum>"#,
        &Groups::new(),
    )
    .unwrap();
    assert_eq!(
        (h.rt_minutes(), h.scans, h.ic_ref.as_deref()),
        (1.0, 2, Some("IC2"))
    );
}

#[test]
fn precursors_keep_their_order_and_a_product_is_never_one() {
    let h = parse(
        r#"<spectrum id="s"><cvParam accession="MS:1000511" value="3"/><precursorList count="2">
<precursor spectrumRef="scan=1"><isolationWindow><cvParam accession="MS:1000827" value="600"/><cvParam accession="MS:1000828" value="1"/><cvParam accession="MS:1000829" value="2"/></isolationWindow>
<selectedIonList><selectedIon><cvParam accession="MS:1000744" value="600.3"/><cvParam accession="MS:1000041" value="2"/></selectedIon><selectedIon><cvParam accession="MS:1000744" value="1"/></selectedIon></selectedIonList>
<activation><cvParam accession="MS:1000133" name="collision-induced dissociation"/><cvParam accession="MS:1000045" value="35"/></activation></precursor>
<precursor spectrumRef="scan=2"><selectedIonList><selectedIon><cvParam accession="MS:1000744" value="400"/></selectedIon></selectedIonList></precursor></precursorList>
<productList><product><isolationWindow><cvParam accession="MS:1000827" value="250"/></isolationWindow></product></productList>"#,
        &Groups::new(),
    )
    .unwrap();
    let p = &h.precursors;
    assert_eq!(p.len(), 2);
    assert_eq!(
        (p[0].target(), p[0].edges(), p[0].selected(), p[0].charge()),
        (
            Some(600.0),
            (Some(599.0), Some(602.0)),
            Some(600.3),
            Some(2.0)
        )
    );
    assert_eq!(
        (p[0].energy(), p[0].method(), p[0].refer.as_deref()),
        (Some(35.0), Some("CID"), Some("scan=1"))
    );
    assert_eq!(
        (p[1].selected(), p[1].refer.as_deref()),
        (Some(400.0), Some("scan=2"))
    );
    assert_eq!(h.products[0].target(), Some(250.0));
    assert!(!h.terms.has("MS:1000827") && !h.terms.has("MS:1000744"));
}

#[test]
fn cut_or_broken_text_ends_the_scan_without_a_panic() {
    for t in [
        r#"<spectrum id="s"><cvParam accession="MS:1000511" value="2"/><cvPar"#,
        r#"<spectrum id="s"><cvParam accession=MS:1/>"#,
        "<",
        "<spectrum",
        "",
    ] {
        let _ = parse(t, &Groups::new());
    }
    let h = parse(
        r#"<spectrum id="s"><cvParam accession="MS:1000511" value="2"/><cvPar"#,
        &Groups::new(),
    )
    .unwrap();
    assert_eq!(h.level(), 2);
}

#[test]
fn unescape_keeps_a_broken_reference_and_normalizes_attribute_white_space() {
    assert_eq!(
        tags::unescape("a&lt;b&#65;&#x42;&bogus;&", false),
        "a<bAB&bogus;&"
    );
    assert_eq!(tags::unescape("a\tb\nc", true), "a b c");
    assert_eq!(tags::unescape("a&#9;b", true), "a\tb");
}
