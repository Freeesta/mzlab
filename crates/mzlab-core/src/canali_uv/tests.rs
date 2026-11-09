use super::*;
use crate::canali::Channels;
use crate::iteminfo::facts_from;
use base64::Engine;

fn b64(vals: &[f64], f64s: bool, zlib: bool) -> String {
    let raw: Vec<u8> = vals
        .iter()
        .flat_map(|&v| {
            if f64s {
                v.to_le_bytes().to_vec()
            } else {
                (v as f32).to_le_bytes().to_vec()
            }
        })
        .collect();
    let raw = if zlib {
        use std::io::Write;
        let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
        z.write_all(&raw).unwrap();
        z.finish().unwrap()
    } else {
        raw
    };
    base64::engine::general_purpose::STANDARD.encode(raw)
}

fn arr(acc: &str, unit: &str, v: &[f64]) -> String {
    format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"{unit}/><binary>{}</binary></binaryDataArray>", b64(v, true, false))
}

/// An mzML with no scans: the TWC (with a 4 × 3 `dad_matrix`) and, when `analog`, a pump pressure trace in bar.
fn file(pda: bool, analog: bool) -> Mzml<Vec<u8>> {
    let matrix: Vec<f64> = (0..12).map(f64::from).collect();
    let matrix = format!("<binaryDataArray encodedLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000521\" name=\"32-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000786\" name=\"non-standard data array\" value=\"dad_matrix\"/><userParam name=\"wl_start\" value=\"200\"/><userParam name=\"wl_stop\" value=\"202\"/><userParam name=\"wl_step\" value=\"1\"/><userParam name=\"n_rt\" value=\"4\"/><userParam name=\"n_wl\" value=\"3\"/><userParam name=\"rt_start\" value=\"0\"/><userParam name=\"rt_stop\" value=\"0.03\"/><binary>{}</binary></binaryDataArray>", b64(&matrix, false, true));
    let chrom = |i: usize, id: &str, acc: &str, extra: &str, y_unit: &str| {
        format!("<chromatogram index=\"{i}\" id=\"{id}\" defaultArrayLength=\"2\"><cvParam accession=\"{acc}\" name=\"x\"/><binaryDataArrayList count=\"3\">{}{}{extra}</binaryDataArrayList></chromatogram>", arr("MS:1000595", "", &[0.0, 0.03]), arr("MS:1000515", y_unit, &[1.0, 2.0]))
    };
    let mut l = String::new();
    if pda {
        l += &chrom(0, "TWC", "MS:1000813", &matrix, "");
    }
    if analog {
        l += &chrom(
            usize::from(pda),
            "Pump Pressure",
            "MS:1003019",
            "",
            " unitName=\"bar\"",
        );
    }
    let s = format!("<mzML><run><chromatogramList count=\"2\">{l}</chromatogramList></run></mzML>")
        .into_bytes();
    let n = s.len() as u64;
    Mzml::new(s, n)
}

fn channels(m: &mut Mzml<Vec<u8>>) -> (Channels, Vec<Chrom>) {
    let chroms = m.chromatograms(0).unwrap();
    let facts = facts_from(m, 0, chroms.len() as i64, false).unwrap();
    (Channels::da(&[], &chroms, &facts, "a.mzML"), chroms)
}

#[test]
fn ids_are_read_back() {
    assert_eq!(read_id("uv:twc"), Some(Reading::Twc));
    assert_eq!(read_id("uv:254±4"), Some(Reading::Nm(254.0, 4.0)));
    assert_eq!(read_id("uv:254+-4.5"), Some(Reading::Nm(254.0, 4.5)));
    assert_eq!(read_id("uv:280"), Some(Reading::Nm(280.0, 0.0)));
    assert_eq!(read_id("an:3"), Some(Reading::Analog(3)));
    for no in ["tic", "uv:", "uv:x±1", "uv:254±-1", "an:x", "srm:1>2"] {
        assert_eq!(read_id(no), None, "{no}");
    }
}

#[test]
fn a_file_with_uv_lists_roots_and_one_without_has_none() {
    let (c, _) = channels(&mut file(true, true));
    let uv = &c.v[c.find("uv:twc").unwrap()];
    assert_eq!(
        (uv.parent, uv.famiglia, uv.spettri, uv.n),
        (None, Famiglia::Uv, true, 2)
    );
    let an = &c.v[c.find("an:1").unwrap()];
    assert_eq!(
        (an.parent, an.famiglia, an.spettri),
        (None, Famiglia::Analog, false)
    );
    assert_eq!(an.label, "Pump Pressure (bar)");
    assert_eq!(c.tipo.proposed, "solo-uv");
    // the derived channel is never listed
    assert!(c.v.iter().all(|x| !x.id.starts_with("uv:2")));
    let (empty, _) = channels(&mut file(false, false));
    assert!(empty
        .v
        .iter()
        .all(|x| !x.id.starts_with("uv:") && !x.id.starts_with("an:")));
}

#[test]
fn channels_answer_what_the_old_routes_answer() {
    let mut m = file(true, true);
    let (_, chroms) = channels(&mut m);
    assert_eq!(
        trace(&mut m, &chroms, "uv:twc", 2000).unwrap(),
        pda_trace(&mut m, &chroms).unwrap()
    );
    assert_eq!(
        trace(&mut m, &chroms, "uv:201±1", 2000).unwrap(),
        pda_wavelength(&mut m, 200.0, 202.0).unwrap()
    );
    assert_eq!(
        trace(&mut m, &chroms, "an:1", 50).unwrap(),
        analog_route(&mut m, &chroms, 1, 50).unwrap()
    );
    assert_eq!(
        spettro(&mut m, "uv:twc", 0.0, 0.01).unwrap(),
        pda_spectrum(&mut m, 0.0, 0.01).unwrap()
    );
    assert_eq!(
        trace(&mut m, &chroms, "tic", 10).unwrap_err().key,
        "err.canali.unknown"
    );
    assert_eq!(
        spettro(&mut m, "an:1", 0.0, 0.0).unwrap_err().key,
        "err.canali.unknown"
    );
}

#[test]
fn a_file_without_uv_answers_no_pda() {
    let mut m = file(false, true);
    let (_, chroms) = channels(&mut m);
    assert_eq!(
        trace(&mut m, &chroms, "uv:twc", 10).unwrap(),
        "{\"rt\":[],\"y\":[]}"
    );
    assert_eq!(
        trace(&mut m, &chroms, "uv:254±4", 10).unwrap_err().key,
        "err.pda.none"
    );
}
