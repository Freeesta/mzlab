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

/// A reader of one sample with no scans and the optical matrix of a `.wiff` (200..210 nm every 2 nm, 5 instants of 0.01 min).
struct Ottico(Option<DadData>);

impl Ottico {
    fn matrice() -> DadData {
        let wl: Vec<f64> = (0..6).map(|j| 200.0 + 2.0 * f64::from(j)).collect();
        let time: Vec<f64> = (0..5).map(|i| f64::from(i as f32 * 0.01)).collect();
        let mat: Vec<f32> = (0..30).map(|k| k as f32 * 0.37 - 3.0).collect();
        DadData {
            twc_mau: time.iter().map(|t| t * 100.0).collect(),
            duration_min: 0.04,
            time_min: time,
            wl_start_nm: 200.0,
            wl_stop_nm: 210.0,
            wl_step_nm: 2.0,
            wavelengths_nm: wl,
            matrix_mau: mat,
        }
    }
}

impl crate::lettori::Lettore for Ottico {
    fn formato(&self) -> crate::lettori::Formato {
        crate::lettori::Formato::Altro("prova")
    }
    fn campioni(&self) -> &[crate::lettori::Campione] {
        static C: std::sync::OnceLock<Vec<crate::lettori::Campione>> = std::sync::OnceLock::new();
        C.get_or_init(|| {
            vec![crate::lettori::Campione {
                id: 1,
                nome: "uv".into(),
                scansioni: 0,
                dimensione: None,
            }]
        })
    }
    fn indice(&mut self, _: usize) -> Result<&[crate::lettori::VoceScansione]> {
        Ok(&[])
    }
    fn scansione(&mut self, _: usize, _: usize) -> Result<crate::mzml::Spectrum> {
        Err(crate::error::Error::new("error.badIndex"))
    }
    fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<crate::lettori::Campo>> {
        Ok(vec![])
    }
    fn cromatogrammi(&mut self, _: usize) -> Result<Vec<crate::lettori::VoceCromatogramma>> {
        Ok(vec![])
    }
    fn cromatogramma(&mut self, _: usize, _: &str) -> Result<crate::mzml::Chromatogram> {
        Err(crate::error::Error::new("error.badIndex"))
    }
    fn metodo(&mut self, _: usize) -> Result<String> {
        Ok(String::new())
    }
    fn dad_dati(&mut self, _: usize) -> Result<Option<DadData>> {
        Ok(self.0.clone())
    }
}

fn convert(dad: bool, matrix: Option<DadData>) -> Vec<u8> {
    let opz = crate::mzmlwrite::Opzioni {
        dad,
        ..Default::default()
    };
    crate::lettori_mzml::scrivi_con(&mut Ottico(matrix), 0, Vec::new(), &mut |_, _| {}, opz)
        .unwrap()
}

#[test]
fn conversion_round_trip_gives_the_json_of_the_wiff() {
    let d = Ottico::matrice();
    let mut m = mz(convert(true, Some(d.clone())));
    assert_eq!(
        pda_map(&mut m).unwrap(),
        crate::wiff::dad_map::dad_to_map_json(&d)
    );
    let spec = crate::wiff::dad_uv::dad_spectrum_json(&d, 0.0, 0.025).unwrap();
    assert_eq!(pda_spectrum(&mut m, 0.0, 0.025).unwrap(), spec);
    let xic = crate::wiff::dad_uv::dad_wavelength_json(&d, 202.0, 206.0).unwrap();
    assert_eq!(pda_wavelength(&mut m, 202.0, 206.0).unwrap(), xic);
    let run = crate::mzml::read(&convert(true, Some(d))[..]).unwrap();
    assert!(run.spectra.is_empty(), "the optical spectra are not scans");
}

#[test]
fn option_off_leaves_the_bytes_as_they_are() {
    let without = convert(false, None);
    assert_eq!(convert(false, Some(Ottico::matrice())), without);
    assert!(!String::from_utf8_lossy(&without).contains("MS:1000804"));
    assert_eq!(convert(true, None), without, "no matrix, nothing to add");
    assert!(
        String::from_utf8_lossy(&convert(true, Some(Ottico::matrice()))).contains("count=\"5\"")
    );
}

/// The matrix of the 3200 PDA (210..400 nm every 1 nm, 0.24 s for 20 min) with a slow baseline and noise: the size the option adds to a file.
#[test]
fn size_of_a_real_sized_matrix() {
    let (nl, nt) = (191usize, 5000usize);
    let mut x = 12345u32;
    let mut noise = move || {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        (x >> 8) as f32 / 16_777_216.0 - 0.5
    };
    let time: Vec<f64> = (0..nt).map(|i| i as f64 * 0.004).collect();
    let mat: Vec<f32> = (0..nt * nl)
        .map(|k| {
            20.0 + 10.0 * ((k / nl) as f32 * 0.01).sin() + (k % nl) as f32 * 0.05 + noise() * 0.2
        })
        .collect();
    let d = DadData {
        twc_mau: vec![0.0; nt],
        duration_min: 20.0,
        time_min: time,
        wl_start_nm: 210.0,
        wl_stop_nm: 400.0,
        wl_step_nm: 1.0,
        wavelengths_nm: (0..nl).map(|j| 210.0 + j as f64).collect(),
        matrix_mau: mat,
    };
    let raw = d.matrix_mau.len() * 4;
    let bytes = convert(true, Some(d)).len();
    eprintln!("raw {raw} mzML {bytes}");
    assert!(bytes < 4 * raw, "{bytes} bytes for {raw} raw");
}

/// A `Vec<u8>` that counts the bytes asked of it.
struct Counted(Vec<u8>, std::rc::Rc<std::cell::Cell<u64>>);

impl crate::idxread::ReadAt for Counted {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + buf.len() as u64);
        self.0.read_at(off, buf)
    }
}

/// The optical routes answer from the matrix alone: the chromatogram list the page-side wrapper used to read first (to throw it away) costs
/// a full read of the tail of the file, and the answers do not change without it.
#[test]
fn optical_routes_do_not_need_the_chromatogram_list() {
    let bytes = with_array(true);
    let size = bytes.len() as u64;
    let run = |with_list: bool| {
        let seen = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut m = Mzml::new(Counted(bytes.clone(), seen.clone()), size);
        if with_list {
            assert_eq!(m.chromatograms(0).unwrap().len(), 1);
        }
        let answers = (
            pda_map(&mut m).unwrap(),
            pda_spectrum(&mut m, 0.0, 0.01).unwrap(),
            pda_wavelength(&mut m, 200.0, 201.0).unwrap(),
        );
        (answers, seen.get())
    };
    let ((with_a, with_bytes), (without_a, without_bytes)) = (run(true), run(false));
    assert_eq!(with_a, without_a);
    assert_eq!(
        with_bytes - without_bytes,
        size,
        "the list read the whole tail once more"
    );
}

#[test]
fn uv_row_time_in_seconds_by_unit_name_or_accession() {
    let full = |u: &str| {
        let arr = |acc: &str, v: &[f64]| format!("<binaryDataArray encodedLength=\"0\"><cvParam accession=\"MS:1000523\"/><cvParam accession=\"{acc}\"/><binary>{}</binary></binaryDataArray>", b64(v, true, false));
        let b = format!("<spectrum><cvParam accession=\"MS:1000804\"/><cvParam accession=\"MS:1000016\" name=\"scan start time\" value=\"30\" {u}/><binaryDataArrayList count=\"2\">{}{}</binaryDataArrayList></spectrum>", arr("MS:1000617", &[200.0, 210.0]), arr("MS:1000515", &[1.0, 2.0]));
        super::uv_row(b.as_bytes()).map(|r| r.0)
    };
    assert_eq!(full("unitName=\"second\""), Some(0.5));
    assert_eq!(full("unitAccession=\"UO:0000010\""), Some(0.5));
    assert_eq!(full("unitAccession=\"UO:0000010\" unitName=\"second\""), Some(0.5));
    assert_eq!(full("unitAccession=\"UO:0000031\" unitName=\"minute\""), Some(30.0));
}
