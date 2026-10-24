use super::*;
use crate::itemroutes::{pda_map, pda_spectrum, pda_wavelength};
use base64::Engine;
use std::io::Write;

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
        let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
        z.write_all(&raw).unwrap();
        z.finish().unwrap()
    } else {
        raw
    };
    base64::engine::general_purpose::STANDARD.encode(raw)
}

const NT: usize = 4;
const NL: usize = 3;

fn matrix() -> Vec<f64> {
    (0..NT * NL).map(|k| k as f64).collect()
}

/// A tiny mzML: one MS1 scan, then the chromatograms; `twc` is the PDA trace and `extra` an optional `dad_matrix` array.
fn with_array(extra: bool) -> Vec<u8> {
    let arr = if extra {
        format!(
            "<binaryDataArray encodedLength=\"1\"><cvParam cvRef=\"MS\" accession=\"MS:1000521\" name=\"32-bit float\"/><cvParam cvRef=\"MS\" accession=\"MS:1000574\" name=\"zlib compression\"/><cvParam cvRef=\"MS\" accession=\"MS:1000786\" name=\"non-standard data array\" value=\"dad_matrix\"/><userParam name=\"wl_start\" value=\"200\"/><userParam name=\"wl_stop\" value=\"202\"/><userParam name=\"wl_step\" value=\"1\"/><userParam name=\"n_rt\" value=\"{NT}\"/><userParam name=\"n_wl\" value=\"{NL}\"/><userParam name=\"rt_start\" value=\"0\"/><userParam name=\"rt_stop\" value=\"0.03\"/><binary>{}</binary></binaryDataArray>",
            b64(&matrix(), false, true)
        )
    } else {
        String::new()
    };
    let one = |kind: &str, v: &[f64], unit: &str| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{kind}\" name=\"x\"{unit}/><binary>{}</binary></binaryDataArray>", b64(v, true, false))
    };
    format!(
        "<mzML><run><spectrumList count=\"1\"><spectrum index=\"0\" id=\"scan=1\" defaultArrayLength=\"1\"><cvParam accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/><cvParam accession=\"MS:1000016\" name=\"scan start time\" value=\"0.5\" unitName=\"minute\"/><binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum></spectrumList><chromatogramList count=\"1\"><chromatogram index=\"0\" id=\"TWC\" defaultArrayLength=\"2\"><cvParam accession=\"MS:1000813\" name=\"absorption chromatogram\"/><binaryDataArrayList count=\"3\">{}{}{arr}</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>",
        one("MS:1000595", &[0.0, 0.03], ""),
        one("MS:1000515", &[1.0, 2.0], ""),
    )
    .into_bytes()
}

fn spectra() -> Vec<u8> {
    let row = |i: usize| {
        let wl = b64(&[200.0, 201.0, 202.0], true, false);
        let ab = b64(&matrix()[i * NL..(i + 1) * NL], true, false);
        format!("<spectrum index=\"{}\" id=\"uv={i}\" defaultArrayLength=\"3\"><cvParam accession=\"MS:1000804\" name=\"electromagnetic radiation spectrum\"/><cvParam accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"second\"/><binaryDataArrayList count=\"2\"><binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000617\" name=\"wavelength array\"/><binary>{wl}</binary></binaryDataArray><binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000515\" name=\"intensity array\"/><binary>{ab}</binary></binaryDataArray></binaryDataArrayList></spectrum>", i + 1, i as f64 * 1.8)
    };
    let ms1 = "<spectrum index=\"0\" id=\"scan=1\" defaultArrayLength=\"0\"><cvParam accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/><binaryDataArrayList count=\"0\"></binaryDataArrayList></spectrum>";
    format!(
        "<mzML><run><spectrumList count=\"5\">{ms1}{}</spectrumList></run></mzML>",
        (0..NT).map(row).collect::<String>()
    )
    .into_bytes()
}

fn mz(bytes: Vec<u8>) -> Mzml<Vec<u8>> {
    let n = bytes.len() as u64;
    Mzml::new(bytes, n)
}

fn all_routes(m: &mut Mzml<Vec<u8>>) -> [Result<String>; 3] {
    [
        pda_map(m),
        pda_spectrum(m, 0.0, 0.01),
        pda_wavelength(m, 200.0, 201.0),
    ]
}

#[test]
fn matrix_array_answers_the_three_routes() {
    let mut m = mz(with_array(true));
    let [map, spec, wl] = all_routes(&mut m).map(|r| r.unwrap());
    assert!(
        map.contains("\"nrt\":4") && map.contains("\"nmz\":3") && map.contains("\"kind\":\"pda\""),
        "{map}"
    );
    // rows 0 and 1 (0, 0.01): means of [0,1,2] and [3,4,5]
    assert!(
        spec.contains("\"mau\":[1.5000,2.5000,3.5000]") && spec.contains("\"n\":2"),
        "{spec}"
    );
    // channels 200, 201: rows [0,1,2] [3,4,5] ... -> 0.5, 3.5, 6.5, 9.5
    assert!(
        wl.contains("\"y\":[0.5000,3.5000,6.5000,9.5000]") && wl.contains("\"n\":2"),
        "{wl}"
    );
}

#[test]
fn optical_spectra_are_not_scans_and_answer_the_routes() {
    let mut m = mz(spectra());
    let [map, spec, wl] = all_routes(&mut m).map(|r| r.unwrap());
    assert!(
        map.contains("\"nrt\":4") && map.contains("\"nmz\":3"),
        "{map}"
    );
    // 0 s and 1.8 s are 0 and 0.03 min: the window 0..0.01 holds row 0
    assert!(spec.contains("\"mau\":[0.0000,1.0000,2.0000]"), "{spec}");
    assert!(
        wl.contains("\"nm0\":200") && wl.contains("\"nm1\":201"),
        "{wl}"
    );
    let run = crate::mzml::read(&spectra()[..]).unwrap();
    assert_eq!(run.spectra.len(), 1, "only the MS1 scan counts");
    let mut bytes = spectra();
    let size = bytes.len() as u64;
    let table = crate::scantable::read(&mut bytes, size).unwrap();
    assert_eq!(
        table.rows.len(),
        1,
        "the optical spectra are not in the scan table"
    );
}

#[test]
fn trace_only_file_has_no_matrix() {
    let mut m = mz(with_array(false));
    for r in all_routes(&mut m) {
        assert_eq!(r.unwrap_err().key, "err.pda.none");
    }
}
