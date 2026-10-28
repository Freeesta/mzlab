//! Bench of the mzML header readers: polarity of a big file and the SRM time range of 50 chromatograms (LOTTO9).
use mzlab_core::iteminfo::facts_from;
use mzlab_core::mzmlhead::Mzml;
use std::time::Instant;

fn main() {
    let mut doc = br#"<mzML><cvParam accession="MS:1000130" name="positive scan"/><run>"#.to_vec();
    let spec = b"<spectrum index=\"0\"><binaryDataArrayList><binaryDataArray><binary>QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=</binary></binaryDataArray></binaryDataArrayList></spectrum>\n";
    while doc.len() < 200 << 20 {
        doc.extend_from_slice(spec);
    }
    let size = doc.len() as u64;
    let mut m = Mzml::new(doc, size);
    let t = Instant::now();
    let p = m.header_polarity().unwrap();
    println!("header_polarity 200 MB: {p} {:?}", t.elapsed());

    let n = 5000;
    let b64 = {
        let mut b = vec![];
        for i in 0..n {
            b.extend_from_slice(&(i as f64 * 0.001).to_le_bytes());
        }
        let t = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_bytes();
        let mut s = String::new();
        for c in b.chunks(3) {
            let v = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            for k in 0..4 {
                s.push(t[((v >> (18 - 6 * k)) & 63) as usize] as char);
            }
        }
        s
    };
    let mut doc = String::from("<mzML><run>");
    for i in 0..50 {
        doc += &format!("<chromatogram index=\"{i}\" id=\"SRM SIC 100.{i} -> 50.0\" defaultArrayLength=\"{n}\"><binaryDataArrayList><binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000595\"/><binary>{b64}</binary></binaryDataArray><binaryDataArray><cvParam accession=\"MS:1000523\"/><cvParam accession=\"MS:1000515\"/><binary>{b64}</binary></binaryDataArray></binaryDataArrayList></chromatogram>");
    }
    doc += "</run></mzML>";
    let size = doc.len() as u64;
    let mut m = Mzml::new(doc.into_bytes(), size);
    let t = Instant::now();
    let mut h = 0u64;
    for _ in 0..20 {
        let f = facts_from(&mut m, 0, 50, true).unwrap();
        h ^= f.srm_rt.map_or(0, |(a, b)| a.to_bits() ^ b.to_bits().rotate_left(7));
    }
    let f = facts_from(&mut m, 0, 50, true).unwrap();
    println!("srm {} rt {:?}", f.srm, f.srm_rt);
    println!("facts_from 50 SRM x20: {:?} hash {h:x}", t.elapsed());
}
