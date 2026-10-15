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

fn write_one(sp: &Spectrum, opz: Opzioni) -> Vec<u8> {
    let mut w = MzmlWriter::con_opzioni(Vec::new(), &Header { run: "x".into(), ..Default::default() }, opz).unwrap();
    w.open_spectra(1).unwrap();
    w.spectrum(sp).unwrap();
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

#[test]
fn numpress_is_lossy_but_close_and_off_by_default() {
    let mz: Vec<f64> = (0..400).map(|i| 100.0 + i as f64 * 0.37 + (i % 7) as f64 * 1e-3).collect();
    let intensity: Vec<f32> = (0..400).map(|i| 10.0 + ((i * 37) % 1000) as f32).collect();
    let sp = Spectrum { index: 0, id: "scan=1".into(), level: 1, rt: 1.0, polarity: 1, profile: false, mz: mz.clone(), intensity: intensity.clone(), ..Default::default() };
    let plain = write_one(&sp, Opzioni::default());
    let np = write_one(&sp, Opzioni { numpress: true, ..Default::default() });
    assert!(!String::from_utf8_lossy(&plain).contains("MS:1002312"));
    let t = String::from_utf8_lossy(&np);
    assert!(t.contains("MS:1002312") || t.contains("MS:1002746"));
    assert!(np.len() < plain.len());
    // the core reader refuses numpress (the readers of the «aperti» module decode it): read the arrays back with the codec itself
    let text = String::from_utf8(np).unwrap();
    let mut back = Vec::new();
    for part in text.split("<binaryDataArray ").skip(1) {
        let (codec, zlib) = part.split("accession=\"").skip(1).find_map(|a| numpress::of_accession(&a[..a.find('"').unwrap()])).expect("numpress codec");
        let b = part.split("<binary>").nth(1).unwrap().split("</binary>").next().unwrap();
        back.push(numpress::decode_text(codec, zlib, b, 400).unwrap());
    }
    assert_eq!(back.len(), 2);
    for (a, b) in back[0].iter().zip(&mz) { assert!((a - b).abs() < 1e-4, "{a} {b}"); }
    for (a, b) in back[1].iter().zip(&intensity) { assert!(((a - *b as f64) / *b as f64).abs() < 1e-3, "{a} {b}"); }
    let p = &mzml::read(&plain[..]).unwrap().spectra[0];
    assert!(p.mz == mz && p.intensity == intensity);
}
