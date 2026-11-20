//! Synthetic DDA-like `.mzidx` shared by the scan-table measurement test and `examples/mem_idx.rs` (no real data).
#![allow(dead_code)]

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

/// The `.mzidx` of a DDA-like mzML: one MS1 then `ms2` MS2 scans with filter, precursor and native id (the strings are what weigh in the table).
pub fn mzidx(scans: usize, ms2: usize) -> Vec<u8> {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "r".into(), ..Default::default() }).unwrap();
    w.open_spectra(scans).unwrap();
    for i in 0..scans {
        let l2 = i % (ms2 + 1) != 0;
        let prec = 200.0 + (i % 700) as f64 * 0.37;
        let s = Spectrum {
            index: i,
            id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
            level: if l2 { 2 } else { 1 },
            rt: i as f64 * 0.001,
            polarity: 1,
            profile: false,
            filter: if l2 {
                format!("FTMS + c ESI d Full ms2 {prec:.4}@hcd30.00 [50.0000-{:.4}]", prec + 20.0)
            } else {
                "FTMS + c ESI Full ms [100.0000-1000.0000]".into()
            },
            precursor: l2.then_some(prec),
            mz: vec![100.0, 150.0],
            intensity: vec![1.0, 2.0],
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = mzlab_core::upload::prepare(&mut src, b.len() as u64, "r.mzML").unwrap();
    mzlab_core::mzidx::build_mzml_to_vec(&p.table, &b, 64 << 20).unwrap().0
}
