//! `cargo run --release -p mzlab-lettori --features thermo --example thermo_diagnosi -- FILE.raw`: the structures of a Thermo `.raw` that does not open (format
//! versions, addresses and counts of every run header) and what the reader says of it. Prints no scan data and no file name; the output can go in a report.

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: thermo_diagnosi FILE.raw");
        std::process::exit(2);
    };
    match std::fs::File::open(&path) {
        Ok(f) => print!(
            "{}",
            opentfraw::RawFileReader::describe(std::io::BufReader::new(f))
        ),
        Err(e) => {
            eprintln!("cannot open the file: {e}");
            std::process::exit(1);
        }
    }
}
