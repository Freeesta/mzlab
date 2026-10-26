//! Numbers only for a `.wiff` + `.wiff.scan` pair: per sample the ExperimentHeader code of each experiment and, per record of the Idx, its size and
//! whether the 8 bytes at its start are the scan MARKER. Shows why a sample is read as scans or as MRM. Run: cargo run --release -p mzlab-core --example wiff_diag -- FILE.wiff
use std::io::{Read, Seek, SeekFrom};

use mzlab_core::wiff::{container, samples, Wiff};

const MARKER: [u8; 8] = [0xfe, 0xff, 0xff, 0xff, 2, 0, 0, 0];

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: wiff_diag FILE.wiff (FILE.wiff.scan next to it)");
    let ole = container(std::fs::read(&path).expect("read .wiff"), &path).expect("not a .wiff");
    let mut scan = std::fs::File::open(format!("{path}.scan")).expect("open .wiff.scan");
    let scan_len = scan.metadata().expect("scan size").len();
    for n in samples(&ole) {
        let w = match Wiff::open(&ole, &path, Some(scan_len), n) {
            Ok(w) => w,
            Err(e) => {
                println!("sample {n}: open error {}", e.key);
                continue;
            }
        };
        let codes: Vec<String> = w
            .experiments
            .iter()
            .map(|e| e.code.map_or("none".into(), |c| c.to_string()))
            .collect();
        println!(
            "sample {n}: experiments {} codes [{}] records {}",
            w.experiments.len(),
            codes.join(","),
            w.records.len()
        );
        // per (experiment): records, records with the marker, records without it, smallest and largest size
        let mut rows: Vec<(usize, usize, usize, u32, u32)> = Vec::new();
        // per experiment: records whose first 4 bytes are the scan marker `fe ff ff ff`, by the version dword that follows (0 = not a marker)
        let mut vers: Vec<[usize; 6]> = Vec::new();
        for r in &w.records {
            let mut head = [0u8; 8];
            let ok = r.at + 8 <= scan_len
                && scan.seek(SeekFrom::Start(r.at)).is_ok()
                && scan.read_exact(&mut head).is_ok();
            let marked = ok && head == MARKER;
            if vers.len() <= r.experiment {
                vers.resize(r.experiment + 1, [0; 6]);
            }
            let v = if ok && head[..4] == MARKER[..4] {
                u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize
            } else {
                0
            };
            vers[r.experiment][v.min(5)] += 1;
            if rows.len() <= r.experiment {
                rows.resize(r.experiment + 1, (0, 0, 0, u32::MAX, 0));
            }
            let row = &mut rows[r.experiment];
            row.0 += 1;
            if marked {
                row.1 += 1
            } else {
                row.2 += 1
            }
            row.3 = row.3.min(r.size);
            row.4 = row.4.max(r.size);
        }
        if w.experiments.is_empty() {
            no_method_profile(&w, &mut scan);
        }
        for (e, (all, yes, no, lo, hi)) in rows.iter().enumerate().filter(|(_, r)| r.0 > 0) {
            let chosen = if w
                .experiments
                .get(e)
                .and_then(|x| x.code)
                .is_some_and(|c| c == 15 || c == 11)
            {
                "scan"
            } else {
                "mrm-or-unknown"
            };
            let v = vers[e];
            println!("  experiment {e}: records {all} marker_yes {yes} marker_no {no} size_min {lo} size_max {hi} chosen {chosen} head_not_marker {} version1 {} version2 {} version3 {} version4_or_more {}", v[0], v[1], v[2], v[3], v[4] + v[5]);
        }
    }
}

/// A sample without an experiment in its method: what its first scans look like (numbers only), to tell a full scan from a product-ion or a survey scan by its content.
fn no_method_profile(w: &Wiff, scan: &mut std::fs::File) {
    let Ok(run) = w.run(scan) else {
        println!("  no method: read error");
        return;
    };
    let mut spans = std::collections::BTreeSet::new();
    for s in &run.spectra {
        if let (Some(a), Some(b)) = (s.mz.first(), s.mz.last()) {
            spans.insert(((a * 10.0).round() as i64, (b * 10.0).round() as i64));
        }
    }
    println!(
        "  no method: spectra {} distinct (first m/z, last m/z) pairs {}",
        run.spectra.len(),
        spans.len()
    );
    for s in run.spectra.iter().take(8) {
        let top = (0..s.mz.len())
            .max_by(|&i, &j| s.intensity_f64(i).total_cmp(&s.intensity_f64(j)))
            .map_or(0.0, |i| s.mz[i]);
        println!(
            "    {} level {} rt {:.4} points {} mz {:.2}..{:.2} base peak m/z {:.2} precursor {:?}",
            s.id,
            s.level,
            s.rt,
            s.mz.len(),
            s.mz.first().copied().unwrap_or(0.0),
            s.mz.last().copied().unwrap_or(0.0),
            top,
            s.precursor
        );
    }
}
