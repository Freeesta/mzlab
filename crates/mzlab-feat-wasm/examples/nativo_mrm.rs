//! The MRM pipeline of the bridge on files, natively (reference of the validation tools, `tools/valida/lr_nativo.py`).
//!
//!     cargo run --release -p mzlab-feat-wasm --example nativo_mrm -- OUT_DIR file.mzML...
//!
//! Writes `<name>.mzfeatf` for every file and `series.mzfeat` (aligned by transition and RT, empty cells integrated) and prints the SHA-256 of each.

use std::fs::File;
use std::path::Path;

use mzlab_core::mzmlhead::Mzml;
use mzlab_core::sorgente::TracceMzml;
use mzlab_feat::mrm::{self, MrmParams};
use sha2::{Digest, Sha256};

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn traces(path: &str) -> Vec<mrm::Srm> {
    let f = File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    mrm::read_srm(&mut TracceMzml::new(&mut Mzml::new(f, size), 0))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("output folder");
    let files: Vec<String> = args.collect();
    let p = MrmParams::default();
    let mut stored = Vec::new();
    for f in &files {
        let name = Path::new(f)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let digest = sha(&std::fs::read(f).expect("read"));
        let b = mrm::file_bytes(&traces(f), &name, &digest, &p);
        println!("{name}.mzfeatf {}", sha(&b));
        std::fs::write(Path::new(&out).join(format!("{name}.mzfeatf")), &b).expect("write");
        stored.push(b);
    }
    let (feats, origin) = mzlab_feat_wasm::run::read_files(&stored).expect("read");
    let table = mrm::table_bytes(&feats, &origin, &p, |k| Ok(traces(&files[k]))).expect("table");
    println!("series.mzfeat {}", sha(&table));
    std::fs::write(Path::new(&out).join("series.mzfeat"), &table).expect("write");
}
