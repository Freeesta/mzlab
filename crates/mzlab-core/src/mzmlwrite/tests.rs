use super::*;
use crate::mzml;

#[test]
fn what_is_written_is_read_back() {
    let sp = Spectrum { index: 0, id: "sample=1 period=1 cycle=1 experiment=2".into(), level: 2, rt: 1.25, polarity: 1, profile: true, precursor: Some(229.1), mz: vec![100.02, 100.08], intensity: vec![1.5, 2.5], wide: Some(vec![1.5, 2.25]), ..Default::default() };
    let ch = Chromatogram { index: 0, id: "SRM SIC Q1=300.5 Q3=100.25 sample=1".into(), kind: ChromKind::Srm, q1: Some(300.5), q3: Some(100.25), unit: "number of detector counts".into(), time: vec![0.5, 0.75], intensity: vec![3.0, 4.0] };
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "x".into(), ..Default::default() }).unwrap();
    w.open_spectra(1).unwrap();
    w.spectrum(&sp).unwrap();
    w.close_spectra().unwrap();
    w.open_chromatograms(1).unwrap();
    w.chromatogram(&ch).unwrap();
    w.close_chromatograms().unwrap();
    let bytes = w.finish().unwrap();
    let run = mzml::read(&bytes[..]).unwrap();
    let g = &run.spectra[0];
    assert_eq!((g.id.as_str(), g.level, g.polarity, g.profile, g.precursor), (sp.id.as_str(), 2, 1, true, Some(229.1)));
    assert!(g.rt == 1.25 && g.mz == sp.mz && g.wide == sp.wide);
    let c = &run.chromatograms[0];
    assert_eq!((c.kind, c.q1, c.q3, c.time.clone(), c.intensity.clone()), (ChromKind::Srm, Some(300.5), Some(100.25), ch.time.clone(), ch.intensity.clone()));
}
