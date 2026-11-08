mod agilent_synth;
use agilent_synth::*;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
#[test]
fn probe() {
    let mut s = sample();
    for x in s.iter_mut().skip(1) { x.kind = 256; x.level = 2; }
    let (idx, peak) = encode(&s);
    let membri = vec![("x.D/AcqData/MSScan.bin", idx), ("x.D/AcqData/MSPeak.bin", peak)]
        .into_iter().map(|(n, b)| Membro { nome: n.into(), dim: b.len() as u64, dati: Box::new(b), sha1: None }).collect();
    match lettori::apri(FileSet { membri }, &Limiti::sito()) {
        Ok(mut l) => { let c = l.samples().to_vec(); let tic = l.chromatogram(0, "TIC").map(|c| c.intensity.len()); eprintln!("PROBE scans {:?} tic {:?} chroms {:?}", c.iter().map(|c| c.scansioni).collect::<Vec<_>>(), tic, l.chromatograms(0).map(|v| v.iter().map(|c| c.id.clone()).collect::<Vec<_>>())); }
        Err(e) => eprintln!("PROBE err {} {:?}", e.key, e.params),
    }
}
