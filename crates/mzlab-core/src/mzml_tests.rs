#![allow(clippy::unwrap_used, clippy::indexing_slicing)]
use crate::mzml::read;
use base64::Engine;
use std::io::Write;

fn mzml(zlib: &[u8]) -> String {
    let b = base64::engine::general_purpose::STANDARD.encode(zlib);
    let a = |acc: &str| {
        format!(
            r#"<binaryDataArray><cvParam accession="MS:1000523"/><cvParam accession="MS:1000574"/><cvParam accession="{acc}"/><binary>{b}</binary></binaryDataArray>"#
        )
    };
    format!(
        r#"<mzML><run><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="3"><binaryDataArrayList>{}{}</binaryDataArrayList></spectrum></spectrumList></run></mzML>"#,
        a("MS:1000514"),
        a("MS:1000515")
    )
}

fn deflate(raw: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(raw).unwrap();
    e.finish().unwrap()
}

fn key(z: &[u8]) -> (String, String) {
    let e = read(mzml(z).as_bytes()).unwrap_err();
    let detail = e
        .params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    (e.key.to_string(), detail)
}

#[test]
fn zlib_errors_keep_their_keys_and_details() {
    let raw: Vec<u8> = [1.0f64, 2.0, 3.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let good = deflate(&raw);
    let ok = read(mzml(&good).as_bytes()).unwrap();
    assert_eq!(ok.spectra[0].mz, vec![1.0, 2.0, 3.0]);
    let trailing = [good.clone(), b"xyz".to_vec()].concat();
    assert_eq!(
        read(mzml(&trailing).as_bytes()).unwrap().spectra[0]
            .mz
            .len(),
        3
    );
    let cut = &good[..good.len() - 6];
    assert_eq!(
        key(cut),
        ("err.file.zlib".into(), "incomplete deflate stream".into())
    );
    assert_eq!(key(&good[..5]).1, "incomplete deflate stream");
    assert_eq!(key(b"hello world 12345").1, "corrupt deflate stream");
    // 3 values declared, far more inflated: a bomb is a memory-budget error, not a zlib one
    let bomb = deflate(&vec![0u8; 8 << 20]);
    assert_eq!(key(&bomb), ("error.memoryBudget".into(), String::new()));
}

/// An mzML of `n` spectra (their size varies with the index) with a chromatogram and an analyzer among them; spectrum `bad` has a corrupt array.
fn series(n: usize, bad: Option<usize>) -> String {
    let array = |acc: &str, vals: &[f64], zlib: bool, corrupt: bool| {
        let raw: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut bytes = if zlib { deflate(&raw) } else { raw };
        if corrupt {
            bytes.truncate(bytes.len() / 2);
        }
        let z = if zlib {
            r#"<cvParam accession="MS:1000574"/>"#
        } else {
            ""
        };
        format!(
            r#"<binaryDataArray><cvParam accession="MS:1000523"/>{z}<cvParam accession="{acc}"/><binary>{}</binary></binaryDataArray>"#,
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    };
    let mut s = String::from(
        r#"<mzML><instrumentConfigurationList><instrumentConfiguration><componentList><analyzer><cvParam name="orbitrap" accession="MS:1000484"/></analyzer></componentList></instrumentConfiguration></instrumentConfigurationList><run><spectrumList>"#,
    );
    for i in 0..n {
        let len = 3 + i % 17;
        let mz: Vec<f64> = (0..len)
            .map(|k| 100.0 + i as f64 + k as f64 * 0.5)
            .collect();
        let it: Vec<f64> = (0..len).map(|k| 1e3 + (i * k) as f64).collect();
        let c = bad == Some(i);
        s += &format!(
            r#"<spectrum index="{i}" id="scan={i}" defaultArrayLength="{len}"><cvParam accession="MS:1000511" value="1"/><binaryDataArrayList>{}{}</binaryDataArrayList></spectrum>"#,
            array("MS:1000514", &mz, i % 3 != 0, c),
            array("MS:1000515", &it, i % 3 != 0, false)
        );
        if i == n / 2 {
            s += r#"<chromatogram index="0" id="TIC" defaultArrayLength="0"></chromatogram>"#;
        }
    }
    s + "</spectrumList></run></mzML>"
}

fn items(xml: &str, helpers: usize) -> (Vec<crate::mzml::Item>, Option<String>) {
    let mut out = Vec::new();
    let r = crate::mzml::help::pool::parse_helped(xml.as_bytes(), &mut |it| out.push(it), helpers);
    (out, r.err().map(|e| e.key.to_string()))
}

#[test]
fn helpers_give_the_items_of_one_thread_in_the_same_order() {
    for bad in [None, Some(1), Some(151), Some(298)] {
        let xml = series(300, bad);
        let want = items(&xml, 0);
        assert_eq!(want.1.is_some(), bad.is_some(), "{bad:?}");
        for helpers in [1, 3, 8] {
            assert_eq!(items(&xml, helpers), want, "{helpers} helpers, bad {bad:?}");
        }
    }
}

/// The same file with its data type, compression and polarity once as inline cvParams and once in shared `referenceableParamGroup`s.
fn shared(groups: bool) -> String {
    let enc = |raw: Vec<u8>| base64::engine::general_purpose::STANDARD.encode(deflate(&raw));
    let mz = enc([100.5f32, 200.25, 300.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect());
    let it = enc([10.0f32, 20.0, 30.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect());
    let (head, pol, ty, zl) = if groups {
        (
            r#"<referenceableParamGroupList count="3"><referenceableParamGroup id="neg"><cvParam accession="MS:1000129" name="negative scan"/></referenceableParamGroup><referenceableParamGroup id="f32z"><cvParam accession="MS:1000521" name="32-bit float"/><cvParam accession="MS:1000574" name="zlib compression"/></referenceableParamGroup><referenceableParamGroup id="prof"><cvParam accession="MS:1000128" name="profile spectrum"/></referenceableParamGroup></referenceableParamGroupList>"#,
            r#"<referenceableParamGroupRef ref="neg"/><referenceableParamGroupRef ref="prof"/>"#,
            r#"<referenceableParamGroupRef ref="f32z"/>"#,
            "",
        )
    } else {
        (
            "",
            r#"<cvParam accession="MS:1000129" name="negative scan"/><cvParam accession="MS:1000128" name="profile spectrum"/>"#,
            r#"<cvParam accession="MS:1000521" name="32-bit float"/>"#,
            r#"<cvParam accession="MS:1000574" name="zlib compression"/>"#,
        )
    };
    format!(
        r#"<mzML>{head}<run><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="3"><cvParam accession="MS:1000511" name="ms level" value="1"/>{pol}<binaryDataArrayList><binaryDataArray>{ty}{zl}<cvParam accession="MS:1000514" name="m/z array"/><binary>{mz}</binary></binaryDataArray><binaryDataArray>{ty}{zl}<cvParam accession="MS:1000515" name="intensity array"/><binary>{it}</binary></binaryDataArray></binaryDataArrayList></spectrum></spectrumList></run></mzML>"#
    )
}

#[test]
fn shared_param_groups_read_like_inline_cvparams() {
    let inline = read(shared(false).as_bytes()).unwrap();
    let grouped = read(shared(true).as_bytes()).unwrap();
    assert_eq!(inline.spectra, grouped.spectra);
    let s = &grouped.spectra[0];
    assert_eq!((s.polarity, s.profile), (-1, true));
    assert_eq!(s.mz, vec![100.5, 200.25, 300.0]);
    assert_eq!(s.intensity, vec![10.0, 20.0, 30.0]);
}
