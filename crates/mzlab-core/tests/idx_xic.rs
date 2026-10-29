//! The peak readers of an index (`each_peak`, `each_inten`, `sum_window`, `max_window`) agree with one another on a made-up reader, whatever the
//! window, and a `pos` outside the scans of its section is an error (`error.badIndex`), not an index out of bounds.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod lettore_finto;

use std::io::Cursor;

use lettore_finto::Finto;
use mzlab_core::idxread::Index;
use mzlab_core::json::{self, Value};
use mzlab_core::{mzidx, upload};

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn index_bytes(n: usize, peaks: usize) -> Vec<u8> {
    let mut l = Finto::vario(n, peaks, 777);
    let p = upload::prepare_lettore(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut l, 0, &mut out, mzidx::DEFAULT_BUDGET).unwrap();
    out.into_inner()
}

fn open(b: &[u8]) -> Index<Vec<u8>> {
    Index::open(b.to_vec(), b.len() as u64).unwrap()
}

#[test]
fn readers_agree() {
    let b = index_bytes(400, 300);
    let mut ix = open(&b);
    for si in 0..ix.sections.len() {
        let np = ix.sections[si].n_peaks;
        let ns = ix.sections[si].n_scans;
        for (a, z) in [
            (0, np),
            (0, np / 3),
            (np / 3, np / 2),
            (np / 2, np / 2),
            (np.saturating_sub(5), np),
        ] {
            let (mut full, mut part) = (Vec::new(), Vec::new());
            ix.each_peak(si, a, z, |p, _, y| full.push((p, y.to_bits())))
                .unwrap();
            ix.each_inten(si, a, z, |p, y| part.push((p, y.to_bits())))
                .unwrap();
            assert_eq!(full, part);
            let (mut sum, mut max) = (vec![0.0; ns], vec![0.0; ns]);
            for &(p, y) in &full {
                let y = f64::from_bits(y);
                sum[p] += y;
                if y > max[p] {
                    max[p] = y;
                }
            }
            assert_eq!(ix.sum_window(si, a, z).unwrap(), sum);
            assert_eq!(ix.max_window(si, a, z).unwrap(), max);
        }
    }
}

#[test]
fn pos_outside_the_scans_is_an_error() {
    let mut b = index_bytes(400, 300);
    let meta = json::parse(&b[u64_at(&b, 16) as usize..][..u64_at(&b, 24) as usize]).unwrap();
    let sec = &meta.get("sections").unwrap().arr()[0];
    let at = |k: &str| sec.get("off").unwrap().get(k).and_then(Value::f64).unwrap() as usize;
    b[at("pos")..at("pos") + 4].copy_from_slice(&(-1i32).to_le_bytes());
    let mut ix = open(&b);
    let n = ix.sections[0].n_peaks;
    assert_eq!(ix.sum_window(0, 0, n).unwrap_err().key, "error.badIndex");
    assert_eq!(
        ix.each_peak(0, 0, n, |_, _, _| {}).unwrap_err().key,
        "error.badIndex"
    );
    // the pieces that do not reach the damaged peak still read
    assert!(ix.sum_window(0, 1, n).is_ok());
}
