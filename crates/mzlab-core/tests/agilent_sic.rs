//! SIC per SIM ion: built from the SIM scans of a synthetic `.D`; the mzML of the registry writes the TIC and the scans.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod conv_aiuto;

use agilent_synth::*;
use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::lettori::FileSet;
use mzlab_core::mzml;

#[test]
fn sim_scans_give_one_sic_per_ion_and_it_is_written() {
    let sim = |t: f64, k: f64| {
        Scan {
            kind: 2,
            pts: vec![(57.0, 10.0 * k), (71.001, 20.0 * k)],
            ..sample().remove(0)
        }
        .at(t)
    };
    let scans: Vec<Scan> = vec![sim(1.0, 1.0), sim(1.1, 2.0), sim(1.2, 3.0)];
    let (idx, peak) = encode(&scans);
    let files = Files {
        ms_scan: &idx,
        ms_peak_len: peak.len() as u64,
        ..Default::default()
    };
    let d = AgilentD::open("x.D", &files).unwrap();
    let run = d.run(&mut peak.clone()).unwrap();
    let sic: Vec<_> = run
        .chromatograms
        .iter()
        .filter(|c| c.kind == mzml::ChromKind::Sic)
        .collect();
    assert_eq!(
        sic.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["SIC 57.00", "SIC 71.00"]
    );
    assert_eq!(
        (sic[0].intensity.clone(), sic[1].time.len()),
        (vec![10.0, 20.0, 30.0], 3)
    );
    let out = conv_aiuto::mzml(FileSet {
        membri: vec![
            conv_aiuto::membro("x.D/AcqData/MSScan.bin", &idx),
            conv_aiuto::membro("x.D/AcqData/MSPeak.bin", &peak),
        ],
    });
    // through the registry the SICs are the ones of the SIM groups of the method: this folder has none, the TIC and the scans are written
    let back = mzml::read(&out[..]).unwrap();
    assert_eq!(back.spectra.len(), 3);
    assert!(back
        .chromatograms
        .iter()
        .any(|c| c.kind == mzml::ChromKind::Tic));
}

impl Scan {
    fn at(self, time: f64) -> Scan {
        Scan { time, ..self }
    }
}
