#![cfg(feature = "waters")]
//! A member the folder has is read or the folder is refused: an `_FUNCnnn.IDX` over the 8 MB of the small members (a long run, about 380 000 scans) is read
//! whole, and one over the byte budget gives `error.memoryBudget`, never a function with no scans as if the index were not there (review WA-05).
mod common_waters;
use common_waters::*;
use mzlab_core::readers::Limits;

/// The synthetic folder with `extra` more one-record scans in function 1 (a 22-B index row and an 8-B record each).
fn long_run(extra: usize) -> Vec<(String, Vec<u8>)> {
    let mut f = folder_file();
    let dat_len = f.iter().find(|x| x.0 == "_FUNC001.DAT").unwrap().1.len();
    let (mut dat, mut idx) = (Vec::new(), Vec::new());
    for k in 0..extra {
        let off = (dat_len + 8 * k) as u32;
        dat.extend(word_int(1000).to_le_bytes());
        dat.extend(word_mz(150.0).to_le_bytes());
        idx.extend(idx_row(off, 1, 1000.0, 10.0 + k as f32 * 1e-4));
    }
    for (name, add) in [("_FUNC001.DAT", dat), ("_FUNC001.IDX", idx)] {
        f.iter_mut().find(|x| x.0 == name).unwrap().1.extend(add);
    }
    f
}

#[test]
fn an_index_over_8_mb_is_read_whole() {
    let extra = 400_000;
    let f = long_run(extra);
    let idx_len = f.iter().find(|x| x.0 == "_FUNC001.IDX").unwrap().1.len();
    assert!(idx_len > 8 << 20, "{idx_len}");
    let mut l = mzlab_lettori::open_bare(file_set(f), &Limits::desktop())
        .unwrap_or_else(|e| panic!("{}", e.key));
    let n = l.index(0).unwrap().len();
    assert!(n >= extra, "{n} scans: the index of function 1 was dropped");
    let losses = l.losses(0).unwrap();
    assert_eq!(losses.declared, losses.read, "{losses:?}");
}

#[test]
fn an_index_over_the_byte_budget_is_an_error_not_an_empty_function() {
    let f = long_run(400_000);
    let small = Limits {
        byte: 4 << 20,
        ..Limits::desktop()
    };
    match mzlab_lettori::open_bare(file_set(f), &small) {
        Ok(mut l) => panic!("opened with {} scans", l.index(0).unwrap().len()),
        Err(e) => assert_eq!(e.key, "error.memoryBudget"),
    }
}
