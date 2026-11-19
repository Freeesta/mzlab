//! Tests of the mzML optical walk on mixed wavelength grids and of the f32 absorbance decode.

use super::tests::{b64, mz};
use crate::itemroutes::{pda_map, pda_wavelength};

/// One optical row with its own wavelength and absorbance arrays (64-bit, plain).
fn row(i: usize, wl: &[f64], ab: &[f64]) -> String {
    let arr = |acc: &str, v: &[f64]| {
        format!("<binaryDataArray encodedLength=\"1\"><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"{acc}\" name=\"x\"/><binary>{}</binary></binaryDataArray>", b64(v, true, false))
    };
    format!(
        "<spectrum index=\"{i}\" id=\"uv={i}\" defaultArrayLength=\"{}\"><cvParam accession=\"MS:1000804\" name=\"electromagnetic radiation spectrum\"/><cvParam accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"second\"/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></spectrum>",
        wl.len(),
        i as f64 * 1.8,
        arr("MS:1000617", wl),
        arr("MS:1000515", ab)
    )
}

#[test]
fn rows_on_other_grids_or_lengths_do_not_join_the_matrix() {
    let g = [200.0, 201.0, 202.0];
    let rows = [
        row(0, &g, &[1.0, 2.0, 3.0]),
        // same text as the first row: the grid is not decoded again
        row(1, &g, &[4.0, 5.0, 6.0]),
        // another grid of the same length: decoded, and it joins
        row(2, &[200.0, 201.0, 203.0], &[7.0, 8.0, 9.0]),
        // another length: left out
        row(3, &[200.0, 201.0, 202.0, 203.0], &[1.0, 1.0, 1.0, 1.0]),
        // absorbance shorter than the wavelengths: not a row
        row(4, &g, &[1.0, 2.0]),
        row(5, &g, &[10.0, 11.0, 12.0]),
    ];
    let xml = format!(
        "<mzML><run><spectrumList count=\"6\">{}</spectrumList></run></mzML>",
        rows.concat()
    );
    let mut m = mz(xml.into_bytes());
    let map = pda_map(&mut m).unwrap();
    assert!(
        map.contains("\"nrt\":4") && map.contains("\"nmz\":3"),
        "{map}"
    );
    let wl = pda_wavelength(&mut m, 200.0, 201.0).unwrap();
    assert!(wl.contains("\"y\":[1.5000,4.5000,7.5000,10.5000]"), "{wl}");
}

#[test]
fn absorbance_decoded_to_f32_equals_the_f64_path() {
    use crate::mzmlarray::{Inflate, Want};
    let vals = [0.0, 1.5, -2.25, 1234.5678, 1e-7, 3.4e10];
    let acc = |bits: &str, z: bool| {
        format!(
            "<cvParam accession=\"{bits}\"/>{}",
            if z {
                "<cvParam accession=\"MS:1000574\"/>"
            } else {
                ""
            }
        )
    };
    let mut inf = Inflate::new();
    for (bits, f64s) in [("MS:1000523", true), ("MS:1000521", false)] {
        for z in [false, true] {
            let head = acc(bits, z);
            let data = b64(&vals, f64s, z);
            let (a, _) =
                super::decode_with(&head, data.as_bytes(), 0, &mut inf, Want::F64).unwrap();
            let (_, b) =
                super::decode_with(&head, data.as_bytes(), 0, &mut inf, Want::F32).unwrap();
            let old: Vec<u8> = a.iter().flat_map(|&v| (v as f32).to_le_bytes()).collect();
            let new: Vec<u8> = b.iter().flat_map(|v| v.to_le_bytes()).collect();
            assert_eq!(old, new, "{bits} zlib={z}");
        }
    }
}
