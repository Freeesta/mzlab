//! Bruker `.d` damage that is left out and counted, not refused: a frame without a usable m/z calibration, a frame whose block does not decode (0 in the
//! BPC), and a damaged side table (named in `metodo/1.avvisi`). Synthetic SQLite + `tdf_bin` built in the test, no laboratory data.
#![cfg(feature = "bruker")]

use mzlab_core::readers::{FileSet, Losses, Member};

mod common_sqlite;
use common_sqlite::*;

fn folder(bin: Vec<u8>, tdf: Vec<u8>) -> FileSet {
    let m = |n: &str, b: Vec<u8>| Member {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    }
}

#[test]
fn a_frame_without_calibration_is_left_out_and_counted() {
    let ok = mzlab_lettori::apri(folder(block2(), tdf_frames_cal(&[0], 3, 1)), &limits());
    assert!(ok.is_ok(), "{:?}", ok.err().map(|e| e.key));
    // a run whose only frame has no usable calibration: nothing can be read, refused at open
    let e = mzlab_lettori::apri(folder(block2(), tdf_frames_cal(&[0], 3, 7)), &limits())
        .err()
        .unwrap();
    assert_eq!(e.key, "err.file.brukerCalibration");
    assert!(
        e.params.contains(&("frame", "1".to_string())),
        "{:?}",
        e.params
    );
    // the middle one of three: its spectrum is left out (no m/z can be computed) and counted, the others are read
    let b = block2();
    let n = b.len() as i64;
    let whole = [b.clone(), b.clone(), b].concat();
    let tdf = db(&tdf_tables(&[0, n, 2 * n], 3, &[1, 7, 1]));
    let mut l = mzlab_lettori::apri(folder(whole, tdf), &limits()).unwrap();
    assert_eq!((l.index(0).unwrap().len(), l.missing(0)), (2, 1));
    assert_eq!(
        l.losses(0).unwrap(),
        Losses::of(3, 2, "warn.bruker.noCalibration")
    );
    assert_eq!(l.scan(0, 1).unwrap().mz.len(), 2);
    assert_eq!(l.chromatogram(0, "BPC").unwrap().time.len(), 2);
}

#[test]
fn an_undecodable_frame_is_zero_in_the_bpc() {
    // the second of three frames is not zstd: its spectrum fails (and is written with no points by a conversion), the BPC has 0 there
    let b = block2();
    let n = b.len() as i64;
    let mut bad = b.clone();
    for x in &mut bad[8..] {
        *x = 0x5a;
    }
    let whole = [b.clone(), bad, b].concat();
    let mut l =
        mzlab_lettori::apri(folder(whole, tdf_frames(&[0, n, 2 * n], 3)), &limits()).unwrap();
    assert!(l.scan(0, 1).is_err());
    let bpc = l.chromatogram(0, "BPC").unwrap();
    assert_eq!(bpc.intensity.len(), 3);
    assert!(bpc.intensity[0] > 0.0 && bpc.intensity[2] > 0.0);
    assert_eq!(bpc.intensity[1], 0.0);
}

#[test]
fn a_damaged_side_table_is_named_and_the_run_is_read() {
    // a GlobalMetadata whose page has a type no SQLite page has: the frames are read, the method document names the table
    let mut tables = tdf_tables(&[0], 3, &[1]);
    let mut page = leaf(&[vec![V::T("SampleName"), V::T("x")]]);
    page[0] = 99;
    tables.push((
        "GlobalMetadata",
        "CREATE TABLE GlobalMetadata (Key TEXT, Value TEXT)",
        page,
    ));
    let mut l = mzlab_lettori::apri(folder(block2(), db(&tables)), &limits()).unwrap();
    assert_eq!(l.index(0).unwrap().len(), 1);
    assert_eq!(l.scan(0, 0).unwrap().mz.len(), 2);
    let doc = l.method(0).unwrap();
    assert!(
        doc.contains("meta.danneggiato") && doc.contains("analysis.tdf: table GlobalMetadata"),
        "{doc}"
    );
}

#[test]
fn a_cut_bin_and_a_frame_without_calibration_are_both_said() {
    // four frames: the second names an absent calibration, the copy of `tdf_bin` ends inside the fourth block
    let b = block2();
    let n = b.len() as i64;
    let bin: Vec<u8> = b.iter().copied().cycle().take(3 * b.len() + 9).collect();
    let tdf = db(&tdf_tables(&[0, n, 2 * n, 3 * n], 3, &[1, 7, 1, 1]));
    let mut l = mzlab_lettori::apri(folder(bin, tdf), &limits()).unwrap();
    // the losses keep the cut `_bin` cause and count both; the method document counts the uncalibrated one
    assert_eq!(
        l.losses(0).unwrap(),
        Losses::of(4, 2, "warn.file.brukerCutBin")
    );
    let doc = l.method(0).unwrap();
    assert!(
        doc.contains("1 spectra whose frame has no usable MzCalibration"),
        "{doc}"
    );
}

#[test]
fn a_damaged_page_of_frames_leaves_out_its_frames_and_counts_it() {
    // 300 frames span several leaves of `Frames` (pages 2, 3, ... under an interior root); the first leaf (page 2) gets a page type no SQLite page has
    let b = block2();
    let n = b.len() as i64;
    let tims: Vec<i64> = (0..300).map(|i| i * n).collect();
    let bin: Vec<u8> = b.iter().copied().cycle().take(300 * b.len()).collect();
    let whole = db_rows(&tdf_rows(&tims, 3, &[1; 300]));
    let mut l = mzlab_lettori::apri(folder(bin.clone(), whole.clone()), &limits()).unwrap();
    assert_eq!(l.index(0).unwrap().len(), 300);
    let mut broken = whole;
    broken[PAG] = 99;
    let mut l = mzlab_lettori::apri(folder(bin, broken), &limits()).unwrap();
    // the frames of the other leaves are read, those of the damaged one are absent (their number is not in the file: not guessed)
    let read = l.index(0).unwrap().len();
    assert!(read > 0 && read < 300, "{read}");
    assert_eq!(l.scan(0, 0).unwrap().mz.len(), 2);
    let doc = l.method(0).unwrap();
    assert!(
        doc.contains("analysis.tdf: table Frames, 1 damaged pages and 0 damaged records left out"),
        "{doc}"
    );
}
