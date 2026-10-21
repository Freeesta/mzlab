//! Converts a vendor file (Thermo `.raw`, ...) to mzML through the registry: `cargo run --release --example raw2mzml -- IN.raw OUT.mzML [profile]`.
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml::{Conversione, Picchi};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = std::path::Path::new(&a[1]);
    let dim = std::fs::metadata(path).expect("stat").len();
    let set = FileSet {
        membri: vec![Membro {
            nome: path.file_name().unwrap().to_string_lossy().into_owned(),
            dim,
            dati: Box::new(std::fs::File::open(path).expect("open")),
            sha1: None,
        }],
    };
    let mut l = lettori::apri(set, &Limiti::desktop()).expect("recognise");
    let out = std::io::BufWriter::new(std::fs::File::create(&a[2]).expect("create mzML"));
    let conv = Conversione {
        peaks: if a.get(3).is_some_and(|p| p == "profile") {
            Picchi::Profilo
        } else {
            Picchi::File
        },
        ..Conversione::default()
    };
    conv.scrivi(&mut *l, 0, out, &mut |_, _| {})
        .expect("convert");
}
