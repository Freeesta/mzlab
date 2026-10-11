//! Time, peak memory, number of features and SHA-256 of the output of `detect` on one or more `.mzidx` files.
//!
//!     cargo run --release -p mzlab-feat --example feat -- <file.mzidx>... [--neg] [--budget-mb N] [--analyse] [--tsv DIR]
//!
//! `--analyse` runs the whole pipeline (quality, isotopes, MS2 with purity) and prints the counts; `--tsv DIR` writes `<name>.tsv` with the table.

use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::ms2::Ms2Links;
use mzlab_feat::pipeline::Steps;
use mzlab_feat::table::{FileFeatures, FLAG_GOOD, FLAG_ISOTOPE};
use mzlab_feat::trace::{detect, Params};
use sha2::{Digest, Sha256};

/// Peak resident memory of the process, MB (Linux `VmHWM`; 0 elsewhere).
fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let mut p = Params::default();
    let mut files = Vec::new();
    let (mut full, mut tsv, mut split) = (false, None::<String>, true);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => p.polarity = Some(-1),
            "--analyse" => full = true,
            "--no-split" => split = false,
            "--tsv" => tsv = args.next(),
            "--budget-mb" => {
                p.budget = args
                    .next()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(96)
                    << 20
            }
            _ => files.push(a),
        }
    }
    let mut total = 0;
    for path in &files {
        let f = std::fs::File::open(path).expect("open");
        let size = f.metadata().expect("size").len();
        let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
        let t0 = Instant::now();
        let (feats, links) = if full {
            analyse_timed(&mut ix, &p, split, path, t0)
        } else {
            (
                detect(&mut ix, &p).unwrap_or_else(|e| panic!("{path}: {e}")),
                Default::default(),
            )
        };
        let secs = t0.elapsed().as_secs_f64();
        if full {
            let n = |b: u8| feats.flags.iter().filter(|f| **f & b != 0).count();
            let ms2 = feats.ms2_n.iter().filter(|m| **m > 0).count();
            println!(
                "  good {}  isotopologues {}  grouped {}  with MS2 {}  links {}",
                n(FLAG_GOOD),
                n(FLAG_ISOTOPE),
                feats.iso_group.iter().filter(|g| **g != 0).count(),
                ms2,
                links.scan.len()
            );
        }
        if let Some(dir) = &tsv {
            write_tsv(dir, path, &feats);
        }
        let sha = Sha256::digest(feats.to_bytes());
        let hex: String = sha.iter().map(|b| format!("{b:02x}")).collect();
        println!(
            "{path}\t{:.2} s\tpeak {:.0} MB\t{} features\t{hex}",
            secs,
            peak_rss_mb(),
            feats.len()
        );
        total += feats.len();
    }
    println!("total\t{total} features");
}

/// The whole pipeline with the time of every step on stderr.
fn analyse_timed(
    ix: &mut Index<std::fs::File>,
    p: &Params,
    split: bool,
    path: &str,
    t0: Instant,
) -> (FileFeatures, Ms2Links) {
    let st = Steps::default();
    let t = |t0: Instant| format!("{:.2}", t0.elapsed().as_secs_f64());
    let mut f = detect(ix, p).unwrap_or_else(|e| panic!("{path}: {e}"));
    let d = t(t0);
    let t1 = Instant::now();
    let mut multi = Vec::new();
    if split {
        let r = mzlab_feat::resolve::assess_and_resolve(ix, &mut f, p, &st.quality, &st.resolve)
            .unwrap();
        eprintln!(
            "  resolved: {} cut into {} rows, {} with several maxima left",
            r.split,
            r.pieces,
            r.multi.len()
        );
        multi = r.multi;
    } else {
        mzlab_feat::quality::assess(ix, &mut f, p, &st.quality).unwrap();
    }
    let q = t(t1);
    let t2 = Instant::now();
    mzlab_feat::isotopes::group(&mut f, &st.iso);
    let i = t(t2);
    let t3 = Instant::now();
    let l = mzlab_feat::ms2::link(ix, &mut f, p, &st.ms2).unwrap();
    let ms2s = t(t3);
    let t4 = Instant::now();
    let mx = mzlab_feat::resolve::mark_mixtures(ix, &mut f, &l, &multi, p, &st.resolve).unwrap();
    eprintln!(
        "  detect {d} s, quality+resolve {q} s, isotopes {i} s, MS2 {ms2s} s, mixtures {mx} ({} s)",
        t(t4)
    );
    (f, l)
}

/// One line per feature: the columns a script needs to compare the table with a truth.
fn write_tsv(dir: &str, path: &str, f: &mzlab_feat::table::FileFeatures) {
    use std::fmt::Write;
    let mut o = String::from("mz\trt\theight\tarea\tapex\tnpts\tflags\tiso_group\tcharge\tshape\tsnr\tfwhm\tasym\tgaps\tqreason\tms2_n\tpurity\tarea_win\n");
    for i in 0..f.len() {
        let _ = writeln!(o, "{:.6}\t{:.4}\t{:.0}\t{:.0}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.1}\t{:.2}\t{:.2}\t{}\t{}\t{}\t{:.3}\t{:.0}", f.mz[i], f.rt[i], f.height[i], f.area[i], f.apex[i], f.npts[i], f.flags[i], f.iso_group[i], f.charge[i], f.shape[i], f.snr[i], f.fwhm[i], f.asym[i], f.gaps[i], f.qreason[i], f.ms2_n[i], f.purity[i], f.area_win[i]);
    }
    let name = std::path::Path::new(path)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{dir}/{name}.tsv"), o).unwrap();
}
