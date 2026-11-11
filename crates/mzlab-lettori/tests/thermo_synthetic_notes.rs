#![cfg(feature = "thermo")]
//! What the reader of a synthetic Orbitrap v66 `.raw` passes on unchecked or puts aside is named in the method with its count, the data left as they were:
//! a trailer ion outside its isolation window, an MS order not decoded. The same document comes from the method alone (`raw::method`, no index built).
mod common_thermo_hr;
use common_thermo_hr::{folder, generate, reader_of};
use mzlab_core::readers::Losses;

#[test]
fn a_trailer_ion_outside_its_window_is_named_in_the_method_and_the_target_kept() {
    let d = folder("ione_fuori");
    let f = d.join("hr.raw");
    generate(&f, &["--scans", "9", "--ms2", "2", "--seed", "3"]);
    let mut b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let mut l = reader_of(b.clone());
    let before = l.method(0).expect("method");
    assert!(!before.contains("meta.ioneScartata"), "{before}");
    let target = l.index(0).expect("index")[1].precursore.expect("an MS2");
    // the trailer record of scan 1 starts with its charge (2) and its monoisotopic m/z: moved 50 m/z off a window of 1.4
    let mut rec = 2u32.to_le_bytes().to_vec();
    rec.extend(target.to_le_bytes());
    let at = b
        .windows(12)
        .position(|w| w == rec)
        .expect("trailer record")
        + 4;
    b[at..at + 8].copy_from_slice(&(target + 50.0).to_le_bytes());
    let mut l = reader_of(b.clone());
    assert_eq!(l.index(0).expect("index")[1].precursore, Some(target));
    assert_eq!(l.losses(0).expect("losses"), Losses::complete(9));
    let doc = l.method(0).expect("method");
    assert!(
        doc.contains(r#""chiave":"meta.ioneScartata","dove":"the trailer's monoisotopic m/z lies outside the isolation window in 1 of 9 scans""#),
        "{doc}"
    );
    // the document of the method alone (`raw::method`, no index built) counts the same scans
    let alone = mzlab_core::raw::method(std::io::Cursor::new(b), "hr.raw").expect("method");
    assert_eq!(alone, doc);
}

#[test]
fn an_ms_order_not_decoded_leaves_the_level_unknown_and_is_named() {
    let d = folder("ordine");
    let f = d.join("hr.raw");
    generate(&f, &["--scans", "9", "--ms2", "2", "--seed", "4"]);
    let mut b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let mut l = reader_of(b.clone());
    assert!(!l.method(0).expect("method").contains("MS order"));
    let (level, points) = (
        l.index(0).expect("index")[0].livello,
        l.scan(0, 0).expect("scan").mz,
    );
    assert_eq!(level, 1);
    // the first scan event's MS order (byte 6 of its preamble) set to MS10: a level the reader does not decode
    let raw = opentfraw::RawFileReader::open(std::io::Cursor::new(b.clone())).expect("open");
    let pre = raw.scan_event(0).expect("event").preamble.bytes;
    let at = b
        .windows(pre.len())
        .position(|w| w == pre)
        .expect("preamble")
        + 6;
    b[at] = 10;
    let mut l = reader_of(b.clone());
    assert_eq!(l.index(0).expect("index")[0].livello, 0);
    let s = l.scan(0, 0).expect("scan");
    // the data are what they were: only the level is no longer stated
    assert_eq!((s.level, s.mz), (0, points));
    let doc = l.method(0).expect("method");
    assert!(
        doc.contains(r#""chiave":"meta.nonDecodificato","dove":"MS order of 1 of 9 scans"#),
        "{doc}"
    );
    assert_eq!(
        mzlab_core::raw::method(std::io::Cursor::new(b), "hr.raw").expect("method"),
        doc
    );
}
