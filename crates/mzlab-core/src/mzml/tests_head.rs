//! The engine's mzML reader on synthetic files of known truth: groups, RT units, the immediate precursor, SRM windows, namespace prefixes and
//! CDATA, and the refusals (no number type, arrays of two lengths, a file that declares items and shows none).
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use crate::mzml::read;
use base64::Engine;

fn b64(v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(raw)
}

fn b64_32(v: &[f32]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    base64::engine::general_purpose::STANDARD.encode(raw)
}

/// One array: its kind accession, its precision terms (or a group ref) and its text.
fn arr(kind: &str, prec: &str, text: &str) -> String {
    format!(
        r#"<binaryDataArray encodedLength="0">{prec}<cvParam cvRef="MS" accession="{kind}" name="a"/><binary>{text}</binary></binaryDataArray>"#
    )
}

const F64: &str = r#"<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>"#;

fn spec(id: &str, head: &str, n: usize, arrays: &str) -> String {
    format!(
        r#"<spectrum index="0" id="{id}" defaultArrayLength="{n}">{head}<binaryDataArrayList count="2">{arrays}</binaryDataArrayList></spectrum>"#
    )
}

fn file(groups: &str, spectra: &[String], chroms: &[String]) -> String {
    let g = if groups.is_empty() {
        String::new()
    } else {
        format!(r#"<referenceableParamGroupList count="1">{groups}</referenceableParamGroupList>"#)
    };
    format!(
        r#"<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml">{g}<run id="r"><spectrumList count="{}">{}</spectrumList><chromatogramList count="{}">{}</chromatogramList></run></mzML>"#,
        spectra.len(),
        spectra.concat(),
        chroms.len(),
        chroms.concat()
    )
}

fn plain(head: &str) -> String {
    spec(
        "s",
        head,
        2,
        &(arr("MS:1000514", F64, &b64(&[100.0, 200.0]))
            + &arr("MS:1000515", F64, &b64(&[1.0, 2.0]))),
    )
}

fn scan(rt: &str) -> String {
    format!(r#"<scanList count="1"><scan>{rt}</scan></scanList>"#)
}

#[test]
fn groups_give_the_spectrum_terms_and_the_array_precision() {
    let groups = r#"<referenceableParamGroup id="ms2"><cvParam accession="MS:1000511" name="ms level" value="2"/><cvParam accession="MS:1000129" name="negative scan"/></referenceableParamGroup><referenceableParamGroup id="f32"><cvParam accession="MS:1000521" name="32-bit float"/></referenceableParamGroup>"#;
    let gref = |r: &str| format!(r#"<referenceableParamGroupRef ref="{r}"/>"#);
    let arrays = arr("MS:1000514", F64, &b64(&[100.0, 200.0]))
        + &arr("MS:1000515", &gref("f32"), &b64_32(&[5.0, 6.0]));
    let s = spec("s", &gref("ms2"), 2, &arrays);
    let run = read(file(groups, &[s], &[]).as_bytes()).unwrap();
    let sp = &run.spectra[0];
    assert_eq!((sp.level, sp.polarity), (2, -1));
    assert_eq!(sp.intensity, vec![5.0, 6.0]);
}

#[test]
fn every_retention_time_unit_is_minutes() {
    let cases = [
        (r#"unitAccession="UO:0000032" unitName="hour""#, "0.5", 30.0),
        (
            r#"unitAccession="UO:0000028" unitName="millisecond""#,
            "120000",
            2.0,
        ),
        (r#"unitAccession="UO:0000010" unitName="second""#, "90", 1.5),
        (r#"unitAccession="UO:0000031" unitName="minute""#, "4", 4.0),
        ("", "7", 7.0),
    ];
    for (unit, v, want) in cases {
        let rt = format!(
            r#"<cvParam accession="MS:1000016" name="scan start time" value="{v}" {unit}/>"#
        );
        let run = read(file("", &[plain(&scan(&rt))], &[]).as_bytes()).unwrap();
        assert!(
            (run.spectra[0].rt - want).abs() < 1e-9,
            "{unit}: {}",
            run.spectra[0].rt
        );
    }
    // two scans: the time of the first
    let two = r#"<scanList count="2"><scan><cvParam accession="MS:1000016" value="1" unitName="minute"/></scan><scan><cvParam accession="MS:1000016" value="9" unitName="minute"/></scan></scanList>"#;
    let run = read(file("", &[plain(two)], &[]).as_bytes()).unwrap();
    assert_eq!(run.spectra[0].rt, 1.0);
}

fn precursor(refer: &str, target: f64, selected: Option<f64>) -> String {
    let r = if refer.is_empty() {
        String::new()
    } else {
        format!(r#" spectrumRef="{refer}""#)
    };
    let ion = selected.map_or(String::new(), |s| {
        format!(r#"<selectedIonList count="1"><selectedIon><cvParam accession="MS:1000744" value="{s}"/></selectedIon></selectedIonList>"#)
    });
    format!(
        r#"<precursor{r}><isolationWindow><cvParam accession="MS:1000827" value="{target}"/></isolationWindow>{ion}</precursor>"#
    )
}

#[test]
fn the_immediate_precursor_is_the_one_of_the_level_before() {
    let lv = |n: u8| format!(r#"<cvParam accession="MS:1000511" value="{n}"/>"#);
    let s1 = spec(
        "ms1",
        &lv(1),
        2,
        &(arr("MS:1000514", F64, &b64(&[1.0, 2.0])) + &arr("MS:1000515", F64, &b64(&[1.0, 2.0]))),
    );
    let mut s2 = s1
        .replace("ms1", "ms2")
        .replace(r#"value="1""#, r#"value="2""#);
    s2 = s2.replace(
        "<binaryDataArrayList",
        &format!(
            r#"<precursorList count="1">{}</precursorList><binaryDataArrayList"#,
            precursor("ms1", 500.0, Some(500.1))
        ),
    );
    // MS3 lists the immediate precursor (from the MS2) first, then the first stage
    let pl = precursor("ms2", 250.0, Some(250.2)) + &precursor("ms1", 500.0, Some(500.1));
    let s3 = s1
        .replace("ms1", "ms3")
        .replace(r#"value="1""#, r#"value="3""#)
        .replace(
            "<binaryDataArrayList",
            &format!(r#"<precursorList count="2">{pl}</precursorList><binaryDataArrayList"#),
        );
    let run = read(file("", &[s1, s2, s3.clone()], &[]).as_bytes()).unwrap();
    assert_eq!(run.spectra[1].precursor, Some(500.1));
    assert_eq!(run.spectra[2].precursor, Some(250.2));
    // without refs the last listed is the immediate one
    let plain3 = s3
        .replace(r#" spectrumRef="ms2""#, "")
        .replace(r#" spectrumRef="ms1""#, "");
    let run = read(file("", &[plain3], &[]).as_bytes()).unwrap();
    assert_eq!(run.spectra[0].precursor, Some(500.1));
    // a product window is never a precursor
    let prod = r#"<productList count="1"><product><isolationWindow><cvParam accession="MS:1000827" value="150"/></isolationWindow></product></productList>"#;
    let only = plain(prod);
    let run = read(file("", &[only], &[]).as_bytes()).unwrap();
    assert_eq!(run.spectra[0].precursor, None);
}

#[test]
fn srm_q1_and_q3_come_from_the_isolation_windows() {
    let pr = |t: f64, tag: &str| {
        format!(
            r#"<{tag}><isolationWindow><cvParam accession="MS:1000827" value="{t}"/></isolationWindow></{tag}>"#
        )
    };
    let arrays = arr(
        "MS:1000595",
        &format!(
            r#"{F64}<cvParam accession="MS:1000595" unitName="second" unitAccession="UO:0000010"/>"#
        ),
        &b64(&[60.0, 120.0]),
    ) + &arr("MS:1000515", F64, &b64(&[3.0, 4.0]));
    let c = format!(
        r#"<chromatogram index="0" id="transition 7" defaultArrayLength="2"><cvParam accession="MS:1001473" name="selected reaction monitoring chromatogram"/>{}{}<binaryDataArrayList count="2">{arrays}</binaryDataArrayList></chromatogram>"#,
        pr(400.25, "precursor"),
        pr(150.5, "product")
    );
    let run = read(file("", &[], &[c]).as_bytes()).unwrap();
    let ch = &run.chromatograms[0];
    assert_eq!(ch.kind, crate::mzml::ChromKind::Srm);
    assert_eq!((ch.q1, ch.q3), (Some(400.25), Some(150.5)));
    assert_eq!(ch.time, vec![1.0, 2.0]);
}

#[test]
fn prefixed_names_and_cdata_read_like_plain_ones() {
    let plainf = file("", &[plain("")], &[]);
    let want = read(plainf.as_bytes()).unwrap().spectra;
    let pre = plainf
        .replace(
            r#"xmlns="http://psi.hupo.org/ms/mzml""#,
            r#"xmlns:m="http://psi.hupo.org/ms/mzml""#,
        )
        .replace("<binary>", "<binary><![CDATA[")
        .replace("</binary>", "]]></binary>")
        .replace("</", "</m:")
        .replace("<", "<m:")
        .replace("<m:/m:", "</m:")
        .replace("<m:?", "<?")
        .replace("<m:![CDATA[", "<![CDATA[");
    assert!(pre.contains("<m:spectrum ") && pre.contains("CDATA"));
    let got = read(pre.as_bytes()).unwrap().spectra;
    assert_eq!(got, want);
    assert_eq!(got[0].mz, vec![100.0, 200.0]);
}

#[test]
fn refusals_have_their_keys() {
    let key = |x: String| read(x.as_bytes()).unwrap_err().key.to_string();
    // no number type
    let s = spec(
        "s",
        "",
        2,
        &(arr("MS:1000514", "", &b64(&[1.0, 2.0])) + &arr("MS:1000515", F64, &b64(&[1.0, 2.0]))),
    );
    assert_eq!(key(file("", &[s], &[])), "err.file.mzmlPrecision");
    // arrays of two lengths
    let s = spec(
        "s",
        "",
        2,
        &(arr("MS:1000514", F64, &b64(&[1.0, 2.0])) + &arr("MS:1000515", F64, &b64(&[1.0]))),
    );
    assert_eq!(key(file("", &[s], &[])), "err.file.mzmlLengths");
    // a list that declares spectra the reader cannot see
    let hidden = file("", &[plain("")], &[])
        .replace("<spectrum ", "<spectra ")
        .replace("</spectrum>", "</spectra>");
    assert_eq!(key(hidden), "err.file.mzmlNoItems");
}

#[test]
fn the_spectra_declared_and_read_are_counted() {
    use crate::readers::{Losses, TRUNCATED};
    let whole = file(
        "",
        &[plain(""), plain("").replace(r#"id="s""#, r#"id="t""#)],
        &[],
    );
    assert_eq!(read(whole.as_bytes()).unwrap().losses, Losses::complete(2));
    // the list says three, the file holds two and closes
    let fewer = whole.replace(r#"<spectrumList count="2">"#, r#"<spectrumList count="3">"#);
    assert_eq!(
        read(fewer.as_bytes()).unwrap().losses,
        Losses::of(3, 2, crate::mzml::FEWER)
    );
    // cut after the first spectrum
    let cut = &whole[..whole.find("</spectrum>").unwrap() + "</spectrum>".len()];
    let run = read(cut.as_bytes()).unwrap();
    assert_eq!(
        (run.spectra.len(), run.losses),
        (1, Losses::of(2, 1, TRUNCATED))
    );
}
