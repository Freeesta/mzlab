//! Time and counts of the alignment of a series of `.mzidx` files and of the RT map.
//!
//!     cargo run --release -p mzlab-feat --example allinea -- <file.mzidx>... [--neg] [--tsv FILE] [--family] [--truth FILE]
//!
//! Detects every file, aligns them "per ion" and prints the groups (all, in >= 3 files, in all files), the anchors of the RT map and the times.
//! `--family` also runs the quality, isotopes, MS2 and families of every file and the alignment "per family" (rows, members, SHA-256 of two runs).
//! `--dump FILE` writes the rows of the family table. `--truth FILE` (lines `id<TAB>m/z<TAB>RT`) counts the ions of the list found in each table (+-5 ppm, +-0.15 min); `--tsv` writes the groups.

use std::io::Write;
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::famalign::{align_families, FamAlignParams};
use mzlab_feat::family::{analyse_families, FamParams};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::rtmap::{build, RtParams};
use mzlab_feat::table::{Aligned, FileFeatures};
use mzlab_feat::trace::{detect, Params};
use sha2::{Digest, Sha256};

fn load(path: &str, p: &Params, fam: bool) -> FileFeatures {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
    if !fam {
        return detect(&mut ix, p).unwrap_or_else(|e| panic!("{path}: {e}"));
    }
    let (mut f, links) =
        analyse(&mut ix, p, &Steps::default()).unwrap_or_else(|e| panic!("{path}: {e}"));
    analyse_families(&mut ix, &mut f, &links, p, &FamParams::default())
        .unwrap_or_else(|e| panic!("{path}: {e}"));
    f
}

fn seen(al: &Aligned, r: usize) -> usize {
    (0..al.n_files)
        .filter(|&k| !al.apex_rt[al.cell(r, k)].is_nan())
        .count()
}

fn report(name: &str, al: &Aligned, secs: f64) {
    let n3 = (0..al.len()).filter(|&r| seen(al, r) >= 3).count();
    let nall = (0..al.len()).filter(|&r| seen(al, r) == al.n_files).count();
    println!(
        "{name}: {secs:.2} s, {} rows, {n3} in >= 3 files, {nall} in all, members {}",
        al.len(),
        al.member_mz.len()
    );
}

fn truth(path: &str) -> Vec<(String, f64, f64)> {
    let text = std::fs::read_to_string(path).expect("truth");
    let col = |c: Option<&str>| c.and_then(|v| v.parse::<f64>().ok()).unwrap_or(f64::NAN);
    text.lines()
        .filter_map(|l| {
            let mut c = l.split('\t');
            Some((c.next()?.to_string(), col(c.next()), col(c.next())))
        })
        .collect()
}

/// Rows (reference ion) and members of `al` within 5 ppm and 0.15 min of the ion: `(row, is_reference, role)`.
fn hits(al: &Aligned, mz: f64, rt: f64) -> Vec<(usize, bool, u8)> {
    let tol = mz * 5e-6;
    let mut out = Vec::new();
    for r in (0..al.len()).filter(|&r| seen(al, r) > 0) {
        if al.member_start.is_empty() {
            if (al.mz[r] - mz).abs() <= tol && (al.rt[r] - rt).abs() <= 0.15 {
                out.push((r, true, 0));
            }
            continue;
        }
        for (j, m) in al.members(r).enumerate() {
            if (al.member_mz[m] - mz).abs() <= tol && (al.member_rt[m] as f64 - rt).abs() <= 0.15 {
                out.push((r, j == 0, al.member_role[m]));
            }
        }
    }
    out
}

/// How many ions of the list are the reference ion of a row and how many a row or a member; prints the others.
fn found(al: &Aligned, list: &[(String, f64, f64)]) -> (usize, usize) {
    let (mut rows, mut any) = (0, 0);
    for t in list {
        let h = hits(al, t.1, t.2);
        rows += h.iter().any(|x| x.1) as usize;
        any += !h.is_empty() as usize;
        if !h.is_empty() && !h.iter().any(|x| x.1) {
            println!(
                "  {} (m/z {:.4}, RT {:.2}) is only a member: rows/roles {:?}",
                t.0, t.1, t.2, h
            );
        }
        if h.is_empty() {
            println!("  {} (m/z {:.4}, RT {:.2}) not found", t.0, t.1, t.2);
        }
    }
    (rows, any)
}

/// One line per row of the family table: row, m/z, RT, members as `m/z:RT:role`.
fn dump_rows(path: &str, al: &Aligned) {
    let mut w = std::io::BufWriter::new(std::fs::File::create(path).expect("dump"));
    for r in 0..al.len() {
        let m: Vec<String> = al
            .members(r)
            .map(|m| {
                format!(
                    "{:.5}:{:.3}:{}",
                    al.member_mz[m], al.member_rt[m], al.member_role[m]
                )
            })
            .collect();
        writeln!(
            w,
            "{r}\t{:.5}\t{:.3}\t{}\t{}",
            al.mz[r],
            al.rt[r],
            seen(al, r),
            m.join(",")
        )
        .unwrap();
    }
}

fn digest(al: &Aligned) -> String {
    Sha256::digest(al.to_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn main() {
    let mut p = Params::default();
    let (mut files, mut tsv, mut tr) = (Vec::new(), None::<String>, None::<String>);
    let (mut fam, mut dump) = (false, None::<String>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => p.polarity = Some(-1),
            "--tsv" => tsv = args.next(),
            "--truth" => tr = args.next(),
            "--family" => fam = true,
            "--dump" => dump = args.next(),
            _ => files.push(a),
        }
    }
    let t0 = Instant::now();
    let feats: Vec<FileFeatures> = files.iter().map(|f| load(f, &p, fam)).collect();
    println!(
        "detect{} {} files: {:.2} s",
        if fam { " + families" } else { "" },
        feats.len(),
        t0.elapsed().as_secs_f64()
    );
    let refs: Vec<&FileFeatures> = feats.iter().collect();
    let t1 = Instant::now();
    let al = align(&refs, &AlignParams::default());
    report("per ion", &al, t1.elapsed().as_secs_f64());
    let t2 = Instant::now();
    let m = build(&al, &RtParams::default());
    println!(
        "rt map: {:.2} s, {} anchors, {} nodes, rms max {:.4} min",
        t2.elapsed().as_secs_f64(),
        m.n_anchors,
        m.grid.len(),
        m.rms.iter().cloned().fold(0.0, f64::max)
    );
    let list = tr.map(|t| truth(&t)).unwrap_or_default();
    if !list.is_empty() {
        println!(
            "ion list in the ion table: {:?} of {}",
            found(&al, &list),
            list.len()
        );
    }
    if fam {
        let t3 = Instant::now();
        let fa = align_families(&refs, &FamAlignParams::default());
        report("per family", &fa, t3.elapsed().as_secs_f64());
        println!(
            "sha256 {} / {}",
            digest(&fa),
            digest(&align_families(&refs, &FamAlignParams::default()))
        );
        if !list.is_empty() {
            println!(
                "ion list in the family table (row, row or member): {:?} of {}",
                found(&fa, &list),
                list.len()
            );
        }
    }
    if let (Some(path), true) = (dump, fam) {
        dump_rows(&path, &align_families(&refs, &FamAlignParams::default()));
    }
    if let Some(path) = tsv {
        let mut w = std::io::BufWriter::new(std::fs::File::create(path).expect("tsv"));
        for r in 0..al.len() {
            writeln!(w, "{:.6}\t{:.4}\t{}", al.mz[r], al.rt[r], seen(&al, r)).unwrap();
        }
    }
}
