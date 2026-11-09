//! Load time of an mzML file: `cargo run --release --example load -- FILE` (reads it and sums the TIC).
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: carico FILE.mzML");
    let t = Instant::now();
    let run = mzlab_core::mzml::read(std::fs::File::open(&path).expect("file")).expect("mzML");
    let (_, tic) = mzlab_core::analysis::tic(&run.spectra, 1);
    println!(
        "{} spectra, {} chromatograms, TIC {:.6e}, {:.2} s",
        run.spectra.len(),
        run.chromatograms.len(),
        tic.iter().sum::<f64>(),
        t.elapsed().as_secs_f64()
    );
}
