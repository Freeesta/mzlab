//! Converts a vendor file (Thermo `.raw`, ...) to mzML through the registry: `cargo run --release --example raw2mzml -- IN.raw OUT.mzML [profile]`.
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::readers_mzml::{Conversion, Peaks};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = std::path::Path::new(&a[1]);
    let dim = std::fs::metadata(path).expect("stat").len();
    let set = FileSet {
        membri: vec![Member {
            nome: path.file_name().unwrap().to_string_lossy().into_owned(),
            dim,
            dati: Box::new(std::fs::File::open(path).expect("open")),
            sha1: None,
        }],
    };
    let mut l = readers::apri(set, &Limits::desktop()).expect("recognise");
    let out = std::io::BufWriter::new(std::fs::File::create(&a[2]).expect("create mzML"));
    let conv = Conversion {
        peaks: if a.get(3).is_some_and(|p| p == "profile") {
            Peaks::Profilo
        } else {
            Peaks::File
        },
        ..Conversion::default()
    };
    conv.write(&mut *l, 0, out, &mut |_, _| {})
        .expect("convert");
}
