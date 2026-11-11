#![cfg(feature = "waters")]
//! The mark of the index word (WA-01) and its 22-bit count (WA-02), on the synthetic `flags_folder`: the polynomial of `_HEADER.TXT` is applied to the 0x18
//! scans and not to the 0x4C ones the acquisition stored calibrated; a flag bit above the count does not change it; an unknown mark keeps the polynomial with a notice.
mod common_waters;
use common_waters::*;
use mzlab_core::readers::{Limits, Reader};

fn open(mark2: u8) -> Box<dyn Reader> {
    mzlab_lettori::open_bare(file_set(flags_folder(mark2)), &Limits::sito())
        .unwrap_or_else(|e| panic!("{}", e.key))
}

fn calibrated(mz: f64) -> f64 {
    let (a, b) = FLAGS_CAL;
    (a + b * mz.sqrt()).powi(2)
}

/// The m/z of the encoded word, before any polynomial (the word keeps 27 bits of mantissa).
fn stored(mz: f64) -> f64 {
    let u = word_mz(mz);
    f64::from(u & 0x07FF_FFFF) / f64::from(1u32 << 27) * 2f64.powi((u >> 27) as i32)
}

#[test]
fn the_polynomial_goes_to_0x18_scans_only() {
    let mut l = open(0x4C);
    let ix = l.index(0).unwrap().to_vec();
    assert_eq!(
        ix.len(),
        6,
        "3 scans per function, flag bit 22 does not change the count"
    );
    for (k, v) in ix.iter().enumerate() {
        let s = l.scan(0, k).unwrap();
        let f1 = v.id.starts_with("function=1 ");
        let n: u32 = v.id.rsplit('=').next().unwrap().parse().unwrap();
        let want = peaks(n - 1);
        assert_eq!(s.mz.len(), 3, "{}", v.id);
        for (got, (mz, _)) in s.mz.iter().zip(&want) {
            let x = if f1 {
                calibrated(stored(*mz))
            } else {
                stored(*mz)
            };
            assert_eq!(*got, x, "{}: {got} vs {x}", v.id);
        }
        let h = l.scan_header(0, k).unwrap();
        let cal = &h
            .iter()
            .find(|f| f.nome == "m/z calibration")
            .unwrap()
            .valore;
        assert_eq!(cal.contains("0x4C"), !f1, "{}: {cal}", v.id);
    }
    let m = l.method(0).unwrap();
    assert!(!m.contains("unknown calibration flag"), "{m}");
}

#[test]
fn an_unknown_mark_keeps_the_polynomial_and_is_counted() {
    let mut l = open(0x20);
    let ix = l.index(0).unwrap().to_vec();
    let k = ix
        .iter()
        .position(|v| v.id.starts_with("function=2 "))
        .unwrap();
    let s = l.scan(0, k).unwrap();
    assert_eq!(s.mz[0], calibrated(stored(peaks(0)[0].0)));
    let m = l.method(0).unwrap();
    assert!(
        m.contains(
            "function 2: unknown calibration flag 0x20 in 3 scans, polynomial applied as for 0x18"
        ),
        "{m}"
    );
}
