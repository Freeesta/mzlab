//! A numpress mzML cut at every length: the whole scans are read, those cut away are counted by `missing`, and a TIC lost with the cut is
//! built from the `totIonCurrent` of the scans left (the same values the file's TIC holds); the whole file keeps its own TIC.
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::numpress::Np;

fn tic(l: &mut Box<dyn mzlab_core::lettori::Lettore>) -> Chromatogram {
    l.chromatogram(0, "TIC").expect("the TIC is there")
}

#[test]
fn a_cut_numpress_mzml_keeps_its_whole_scans_and_a_tic() {
    let scan = scansioni();
    let ok = mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Pic,
            zlib: true,
        },
        None,
    );
    let mut full = apri(ok.clone());
    let whole = tic(&mut full);
    assert_eq!(
        full.chromatograms(0).unwrap().len(),
        1,
        "the whole file lists its own TIC only"
    );
    let want: Vec<_> = (0..scan.len()).map(|k| full.scan(0, k).unwrap()).collect();
    for cut in 0..ok.len() {
        let Ok(mut l) = mzlab_lettori::apri(
            insieme("campione.mzML", ok[..cut].to_vec()),
            &Limiti::sito(),
        ) else {
            continue;
        };
        let n = l.index(0).unwrap().len();
        assert_eq!(
            n + l.missing(0),
            scan.len(),
            "cut at {cut}: a scan lost unsaid"
        );
        for (k, w) in want.iter().enumerate().take(n) {
            let s = l.scan(0, k).unwrap();
            assert!(
                s.mz == w.mz && s.intensity == w.intensity,
                "cut at {cut}: scan {k} read short"
            );
        }
        if n == 0 {
            continue;
        }
        let voci = l.chromatograms(0).unwrap();
        assert!(
            voci.iter()
                .any(|v| v.id == "TIC" && v.tipo == ChromKind::Tic),
            "cut at {cut}: the TIC is gone"
        );
        let t = tic(&mut l);
        assert_eq!(
            t.time.len(),
            n,
            "cut at {cut}: one TIC point per whole scan"
        );
        for (i, (x, y)) in t.time.iter().zip(&t.intensity).enumerate() {
            assert!((x - whole.time[i]).abs() < 1e-6, "cut at {cut}: time {i}");
            assert!(
                (y - whole.intensity[i]).abs() <= 1e-6 * whole.intensity[i].abs().max(1.0),
                "cut at {cut}: TIC {i}"
            );
        }
        // the writer goes over every scan again: a sample of the cuts is enough for it
        assert!(
            cut % 16 != 0
                || mzlab_core::lettori_mzml::scrivi(l.as_mut(), 0, Vec::new(), &mut |_, _| {})
                    .is_ok(),
            "cut at {cut}: the mzML is not written"
        );
    }
}
