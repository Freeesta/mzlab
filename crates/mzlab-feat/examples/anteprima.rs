//! Preview of the parameters of one file (the same JSON the panel "Pipeline" of mzFinder shows):
//!
//!     cargo run --release -p mzlab-feat --example anteprima -- file.mzidx [floor]
//!
//! Prints the time and the JSON of `params::preview` (the floor is the estimated one unless it is given).

use std::fs::File;
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::params;
use mzlab_feat::trace::Params;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().expect("file.mzidx");
    let f = File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    let mut ix = Index::open(f, size).expect("index");
    let t0 = Instant::now();
    let mut p = Params::default();
    match args.get(1).and_then(|x| x.parse::<f64>().ok()) {
        Some(x) => p.floor = x,
        None => {
            let si = ix.section_of(1, p.polarity).expect("MS1 section");
            let h = params::intensity_hist(&mut ix, si).expect("histogram");
            if let Some(n) = params::noise_from_hist(&h) {
                p.floor = n.floor;
            }
        }
    }
    let json = params::preview(&mut ix, &p).expect("preview");
    println!(
        "{:.2} s, floor {:e}\n{json}",
        t0.elapsed().as_secs_f64(),
        p.floor
    );
}
