//! `cargo run --release -p mzlab-lib --example abre -- FILE.mzlib` opens a library from the disk and checks it.
use mzlab_lib::io::ReadSeekSource;
use mzlab_lib::read::{Library, CACHE_BYTES};
fn main() {
    let f = std::fs::File::open(std::env::args().nth(1).unwrap()).unwrap();
    let t = std::time::Instant::now();
    match Library::open(ReadSeekSource::new(f).unwrap(), CACHE_BYTES) {
        Ok(l) => {
            println!("opened {} spectra in {:?}", l.n_spectra(), t.elapsed());
            println!("verify: {:?}", l.verify().map_err(|e| e.to_string()));
        }
        Err(e) => println!("error: {e}"),
    }
}
