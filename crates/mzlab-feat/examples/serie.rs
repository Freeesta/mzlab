//! A series of `.mzidx` files to the aligned table `.mzfeat`: analysis of each file, alignment, gap filling, with the times of each step.
//!
//!     cargo run --release -p mzlab-feat --example serie -- <file.mzidx>... [--neg] [--ion] [--out FILE.mzfeat] [--twice]
//!
//! Prints rows, cells by state, the SHA-256 of the `.mzfeat` and whether reading it back gives the same table; `--twice` builds it a second time and
//! compares the digests; `--ion` aligns per ion instead of per family. The meta of the file carries no times, so two runs give the same bytes.

use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::series::{align_and_fill, analyse_file, source_sha256};
use mzlab_feat::store::{read_table, write_table, Origin};
use mzlab_feat::table::{CELL_ABSENT, CELL_DETECTED, CELL_FILLED};
use sha2::{Digest, Sha256};

type Idx = Index<std::fs::File>;

fn open(path: &str) -> Idx {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Builds the table of the series; returns its bytes and the printed report.
fn build(files: &[String], r: &Recipe) -> Vec<u8> {
    let t0 = Instant::now();
    let mut feats = Vec::new();
    let mut origin = Origin::default();
    for path in files {
        let mut ix = open(path);
        origin.inputs.push((
            path.rsplit('/').next().unwrap_or(path).to_string(),
            source_sha256(&ix),
        ));
        feats.push(analyse_file(&mut ix, r).unwrap_or_else(|e| panic!("{path}: {e}")));
    }
    let t1 = Instant::now();
    let (table, filled) =
        align_and_fill(&feats, r, |k| Ok(open(&files[k]))).expect("align and fill");
    let count = |s: u8| table.state.iter().filter(|&&x| x == s).count();
    println!(
        "analysis of {} files: {:.2} s; alignment and filling: {:.2} s",
        files.len(),
        (t1 - t0).as_secs_f64(),
        t1.elapsed().as_secs_f64()
    );
    println!(
        "{} rows, cells: {} detected, {} filled ({filled}), {} absent",
        table.len(),
        count(CELL_DETECTED),
        count(CELL_FILLED),
        count(CELL_ABSENT)
    );
    let bytes = write_table(&table, r, &origin);
    let (back, _) = read_table(&bytes).expect("read back");
    println!(
        "read back: {}",
        if back.to_bytes() == table.to_bytes() {
            "same table"
        } else {
            "DIFFERENT"
        }
    );
    bytes
}

fn main() {
    let mut r = Recipe::default();
    let (mut files, mut out, mut twice) = (Vec::new(), None::<String>, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => r.detect.polarity = Some(-1),
            "--ion" => r.mode = Mode::Ion,
            "--out" => out = args.next(),
            "--twice" => twice = true,
            _ => files.push(a),
        }
    }
    let bytes = build(&files, &r);
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect::<String>()
    };
    println!("{} bytes, sha256 {}", bytes.len(), digest(&bytes));
    if twice {
        let again = build(&files, &r);
        println!(
            "second run: {}",
            if again == bytes {
                "identical bytes"
            } else {
                "DIFFERENT"
            }
        );
    }
    if let Some(p) = out {
        std::fs::write(p, &bytes).expect("write");
    }
}
