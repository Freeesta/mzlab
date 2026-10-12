//! Time and counts of the alignment of a series of `.mzidx` files ("per ion" mode) and of the RT map.
//!
//!     cargo run --release -p mzlab-feat --example allinea -- <file.mzidx>... [--neg] [--tsv FILE]
//!
//! Detects every file (`trace::detect`), aligns them and prints the groups (all, in >= 3 files, in all files), the anchors of the RT map and the
//! times; `--tsv` writes the groups (m/z, RT, files seen).

use std::io::Write;
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::rtmap::{build, RtParams};
use mzlab_feat::table::FileFeatures;
use mzlab_feat::trace::{detect, Params};

fn load(path: &str, p: &Params) -> FileFeatures {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
    detect(&mut ix, p).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn main() {
    let mut p = Params::default();
    let (mut files, mut tsv) = (Vec::new(), None::<String>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => p.polarity = Some(-1),
            "--tsv" => tsv = args.next(),
            _ => files.push(a),
        }
    }
    let t0 = Instant::now();
    let feats: Vec<FileFeatures> = files.iter().map(|f| load(f, &p)).collect();
    println!(
        "detect {} files: {:.2} s",
        feats.len(),
        t0.elapsed().as_secs_f64()
    );
    let refs: Vec<&FileFeatures> = feats.iter().collect();
    let t1 = Instant::now();
    let al = align(&refs, &AlignParams::default());
    let ta = t1.elapsed().as_secs_f64();
    let seen = |r: usize| {
        (0..al.n_files)
            .filter(|&k| !al.apex_rt[al.cell(r, k)].is_nan())
            .count()
    };
    let n3 = (0..al.len()).filter(|&r| seen(r) >= 3).count();
    let nall = (0..al.len()).filter(|&r| seen(r) == al.n_files).count();
    println!(
        "align: {:.2} s, {} groups, {} in >= 3 files, {} in all",
        ta,
        al.len(),
        n3,
        nall
    );
    let t2 = Instant::now();
    let m = build(&al, &RtParams::default());
    println!(
        "rt map: {:.2} s, {} anchors, {} nodes, rms max {:.4} min",
        t2.elapsed().as_secs_f64(),
        m.n_anchors,
        m.grid.len(),
        m.rms.iter().cloned().fold(0.0, f64::max)
    );
    if let Some(path) = tsv {
        let mut w = std::io::BufWriter::new(std::fs::File::create(path).expect("tsv"));
        for r in 0..al.len() {
            writeln!(w, "{:.6}\t{:.4}\t{}", al.mz[r], al.rt[r], seen(r)).unwrap();
        }
    }
}
