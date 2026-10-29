//! A synthetic centroid LC-MS run as an `.mzidx`, for the tests and the bench of the detection pipeline: `ions` traces on a gradient shared by the persistent
//! ones, each with a narrow peak, every `BAND_EVERY`-th with a broad band as well. Deterministic.
#![allow(dead_code)]

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;

pub const BAND_EVERY: usize = 20;
/// Seconds between two scans.
pub const DT_S: f64 = 0.5;

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            s.push(if i <= c.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

fn array(acc: &str, v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name=""/><binary>{}</binary></binaryDataArray>"#,
        b64(&raw)
    )
}

struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The mzML text of `scans` scans of `ions` ions (positive MS1).
pub fn mzml(scans: usize, ions: usize) -> Vec<u8> {
    let mut rng = Lcg(42);
    // per ion: persistent?, baseline height, narrow peak (centre scan, height), band (centre scan, height)
    let spec: Vec<(bool, f64, f64, f64, f64, f64)> = (0..ions)
        .map(|i| {
            let n_c = 30.0 + (scans as f64 - 60.0) * rng.unit();
            let b_c = 0.2 * scans as f64 + 0.6 * scans as f64 * rng.unit();
            (
                i % 5 == 0,
                3e4 + 3e4 * rng.unit(),
                n_c,
                2e5 + 8e5 * rng.unit(),
                b_c,
                if i % BAND_EVERY == 0 { 1.5e5 } else { 0.0 },
            )
        })
        .collect();
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{scans}">"#
    );
    for k in 0..scans {
        let t = k as f64 / scans as f64;
        // the gradient shared by the persistent traces
        let g = 1.0 + 2.0 * t + 0.5 * libm::sin(6.0 * t);
        let (mut mz, mut y) = (Vec::new(), Vec::new());
        for (i, &(persistent, base, n_c, n_h, b_c, b_h)) in spec.iter().enumerate() {
            let kk = k as f64;
            let narrow = n_h * libm::exp(-0.5 * ((kk - n_c) / 3.0).powi(2));
            let band = b_h * libm::exp(-0.5 * ((kk - b_c) / 70.0).powi(2));
            let v = narrow + band + if persistent { base * g } else { 0.0 };
            if v >= 1.5e4 {
                mz.push(150.0 + i as f64 * 0.37 + 0.0002 * ((k + i) % 3) as f64);
                y.push(v * (0.97 + 0.06 * ((k * 31 + i * 17) % 11) as f64 / 10.0));
            }
        }
        x += &format!(
            r#"<spectrum index="{k}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            k + 1,
            mz.len(),
            k as f64 * DT_S / 60.0,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x += "</spectrumList></run></mzML>";
    x.into_bytes()
}

/// The `.mzidx` bytes of [`mzml`].
pub fn index_bytes(scans: usize, ions: usize) -> Vec<u8> {
    let text = mzml(scans, ions);
    let mut src = text.clone();
    let p = prepare(&mut src, text.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &text, 64 << 20).unwrap().0
}

pub fn open(bytes: Vec<u8>) -> Index<Vec<u8>> {
    let n = bytes.len() as u64;
    Index::open(bytes, n).unwrap()
}
