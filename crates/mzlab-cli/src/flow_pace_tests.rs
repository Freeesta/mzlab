//! Progress of the native index build (`flow_pace::build_index_with`) and of `write_mzml_with`: monotone, at most one report per 1 %, ends at the total, and
//! the file made is the same bytes as without a callback.

use mzlab_core::lettori_mzml::Picchi;
use mzlab_core::mzmlwrite::Opzioni;

use crate::convert_fixtures::{folder, makers, wiff};

fn well_formed(seen: &[(usize, usize)]) {
    assert!(
        seen.len() <= 101 && seen.windows(2).all(|w| w[0].0 < w[1].0),
        "{seen:?}"
    );
    assert_eq!(seen.last().map(|&(d, t)| d == t), Some(true), "{seen:?}");
}

#[test]
fn the_index_reports_monotone_progress_and_is_the_same_bytes() {
    let d = folder("pace_idx");
    let src = d.join("a.mzML");
    std::fs::write(&src, crate::tests::mzml(1.0)).unwrap();
    let (a, b) = (d.join("a.mzidx"), d.join("b.mzidx"));
    let budget = mzlab_core::mzidx::DEFAULT_BUDGET;
    crate::build_index_of(&src, "", &a, budget).unwrap();
    let mut seen = Vec::new();
    let step = &mut |d, t| seen.push((d, t));
    crate::flow_pace::build_index_with(&src, "", &b, budget, step, &None).unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    well_formed(&seen);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_conversion_reports_monotone_progress_and_is_the_same_bytes() {
    let d = folder("pace_conv");
    let src = makers(&d, false).join("a.mzXML");
    let (a, b) = (d.join("a.mzML"), d.join("b.mzML"));
    crate::flow_base::write_mzml(&src, None, &a, Picchi::File, Opzioni::default()).unwrap();
    let mut seen = Vec::new();
    let step = &mut |d, t| seen.push((d, t));
    let how = (Picchi::File, Opzioni::default());
    crate::flow_base::write_mzml_with(&src, None, &b, how, step, &None).unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    well_formed(&seen);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_hash_taken_while_writing_is_the_hash_of_the_file_and_the_memo_of_the_input_changes_no_byte()
{
    let d = folder("pace_hash");
    let src = makers(&d, false).join("a.mzXML");
    let how = (Picchi::File, Opzioni::default());
    // no memo of the input yet: the reader takes the SHA-1 of the header itself
    let a = d.join("a.mzML");
    let (_, bytes, sha) =
        crate::flow_base::write_mzml_with(&src, None, &a, how, &mut |_, _| {}, &None).unwrap();
    assert_eq!(
        crate::flow_hash::file_sha256(&a).unwrap(),
        (bytes, sha.clone())
    );
    assert_eq!(std::fs::metadata(&a).unwrap().len(), bytes);
    // `digest` hashes the input once for both, the reader then takes the SHA-1 from it
    let (len, sha256) = crate::flow_set::digest(&src).unwrap();
    assert_eq!(len, std::fs::metadata(&src).unwrap().len());
    assert_eq!(sha256, crate::flow_hash::file_sha256(&src).unwrap().1);
    let given = crate::flow_set::file_set(&src).unwrap().membri[0]
        .sha1
        .clone();
    let mut f = std::fs::File::open(&src).unwrap();
    let size = f.metadata().unwrap().len();
    assert_eq!(
        given,
        Some(mzlab_core::metodo::sha1_hex(&mut f, size).unwrap())
    );
    let b = d.join("b.mzML");
    crate::flow_base::write_mzml_with(&src, None, &b, how, &mut |_, _| {}, &None).unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_tiny_budget_spills_to_a_scratch_file_that_is_removed_and_changes_no_byte() {
    let d = folder("pace_spill");
    let src = d.join("a.mzML");
    std::fs::write(&src, crate::tests::mzml(1.0)).unwrap();
    let (big, small) = (d.join("big.mzidx"), d.join("small.mzidx"));
    crate::build_index_of(&src, "", &big, 1 << 30).unwrap();
    crate::build_index_of(&src, "", &small, 1 << 10).unwrap();
    assert_eq!(std::fs::read(&big).unwrap(), std::fs::read(&small).unwrap());
    let mut names: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["a.mzML", "big.mzidx", "small.mzidx"],
        "no scratch is left"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// A file read through the registry (mzXML, ANDI) has its fingerprints taken by a second thread while its scans are decoded: the index is the same
/// bytes as with the fingerprints taken first (`upload::impronte`), and a job cancelled before it starts ends with `err.flow.cancelled`.
#[test]
fn the_fingerprints_taken_beside_the_scans_change_no_byte_and_a_cancel_stops() {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    let d = folder("pace_prints");
    let src = makers(&d, false);
    let budget = mzlab_core::mzidx::DEFAULT_BUDGET;
    for f in ["a.mzXML", "b.cdf"] {
        let p = src.join(f);
        let a = d.join(format!("{f}.mzidx"));
        crate::build_index_of(&p, "", &a, budget).unwrap();
        let mut fs = crate::flow_set::file_set(&p).unwrap();
        let imp = mzlab_core::upload::impronte(&mut fs).unwrap();
        let mut l = mzlab_lettori::apri(fs, &mzlab_core::lettori::Limiti::desktop()).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        mzlab_core::upload::indicizza_lettore_con(
            &mut *l,
            0,
            &mut || Ok(imp.clone()),
            f,
            &mut out,
            budget,
            None,
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), out.into_inner(), "{f}");
        let stop = Some(Arc::new(AtomicBool::new(true)));
        let e = crate::flow_pace::build_index_with(
            &p,
            "",
            &d.join("x.mzidx"),
            budget,
            &mut |_, _| {},
            &stop,
        )
        .unwrap_err();
        assert_eq!(e.key, "err.flow.cancelled", "{f}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_dataset_hashed_before_its_index_is_not_read_again_for_its_prints_and_gives_the_same_bytes() {
    let d = folder("pace_prints");
    if !wiff(&d, "w", 2) {
        return;
    }
    let src = d.join("w.wiff");
    let (a, b) = (d.join("a.mzidx"), d.join("b.mzidx"));
    let budget = mzlab_core::mzidx::DEFAULT_BUDGET;
    // not hashed yet: the fingerprints come from a read of their own
    assert!(crate::flow_set::known_prints(&src).unwrap().is_none());
    crate::build_index_of(&src, "w.wiff@2", &a, budget).unwrap();
    crate::flow_set::digest(&src).unwrap();
    let mut fs = crate::flow_set::file_set(&src).unwrap();
    let read = mzlab_core::upload::impronte(&mut fs).unwrap();
    assert_eq!(read.len(), 2);
    assert_eq!(crate::flow_set::known_prints(&src).unwrap(), Some(read));
    crate::build_index_of(&src, "w.wiff@2", &b, budget).unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    // a member that changed since `digest` (another size) is read again, never taken from the stale hash
    let scan = d.join("w.wiff.scan");
    let mut bytes = std::fs::read(&scan).unwrap();
    bytes.push(0);
    std::fs::write(&scan, bytes).unwrap();
    assert!(crate::flow_set::known_prints(&src).unwrap().is_none());
    let _ = std::fs::remove_dir_all(&d);
}
