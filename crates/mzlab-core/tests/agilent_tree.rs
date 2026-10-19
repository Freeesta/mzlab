//! What follows the head of `AcqData/MSTree2.bin` in the real runs (skipped without the data repository): per scan the TIC as `f32`, then one block
//! per ion with the trace of that ion. Read here with a parser that shares no code with the reader.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::agilent::{AgilentD, Files};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn u32_at(b: &[u8], o: usize) -> usize {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize
}

/// Offset of the first byte after the head and the `n` times (see `info::tree`): 8, three texts, 4 × f64 + 6 bytes, two texts + u32, count, times.
fn after_times(b: &[u8]) -> (usize, usize) {
    let mut o = 8;
    for _ in 0..3 {
        o += 1 + b[o] as usize;
    }
    o += 38;
    o += 1 + b[o] as usize;
    o += 4;
    o += 1 + b[o] as usize;
    let n = u32_at(b, o);
    (o + 4 + 8 * n, n)
}

#[test]
fn the_tree_after_the_head_holds_the_tic_and_the_ion_traces() {
    let Some(gc) = dati_percorsi::percorso("dati/GC-MS") else {
        eprintln!("SKIP agilent_tree: no data repository (MZLAB_DATI)");
        return;
    };
    let mut checked = 0;
    for d in std::fs::read_dir(gc).unwrap().flatten().map(|e| e.path()) {
        let acq = d.join("AcqData");
        let (Ok(tree), Ok(idx)) = (
            std::fs::read(acq.join("MSTree2.bin")),
            std::fs::read(acq.join("MSScan.bin")),
        ) else {
            continue;
        };
        let xsd = std::fs::read(acq.join("MSScan.xsd")).ok();
        let len = std::fs::metadata(acq.join("MSPeak.bin")).unwrap().len();
        let files = Files {
            ms_scan: &idx,
            ms_scan_xsd: xsd.as_deref(),
            ms_peak_len: len,
            ..Default::default()
        };
        let run = AgilentD::open("x.D", &files).unwrap();
        let (at, n) = after_times(&tree);
        assert_eq!(n, run.records.len(), "{}: scans in the tree", d.display());
        for (i, r) in run.records.iter().enumerate() {
            let v = f32::from_le_bytes(tree[at + 4 * i..at + 4 * i + 4].try_into().unwrap());
            assert_eq!(v as f64, r.tic, "{}: TIC of scan {}", d.display(), r.id);
        }
        // First ion block: 12 bytes (1, 0, 0), a u32, the ion as f64, a u32 = 1, four zero u32, the count, then the traces as f32.
        let b = at + 4 * n;
        assert_eq!(
            &tree[b..b + 12],
            &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "{}",
            d.display()
        );
        let ion = f64::from_le_bytes(tree[b + 16..b + 24].try_into().unwrap());
        let count = u32_at(&tree, b + 44);
        assert!(
            (1.0..2000.0).contains(&ion) && count > 0 && b + 48 + 4 * count <= tree.len(),
            "{}: ion {ion} count {count}",
            d.display()
        );
        checked += 1;
    }
    assert!(checked > 0);
}
