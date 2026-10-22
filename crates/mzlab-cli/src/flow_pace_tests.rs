//! Progress of the native index build (`flow_pace::build_index_with`) and of `write_mzml_with`: monotone, at most one report per 1 %, ends at the total, and
//! the file made is the same bytes as without a callback.

use mzlab_core::lettori_mzml::Picchi;
use mzlab_core::mzmlwrite::Opzioni;

use crate::convert_fixtures::{folder, makers};

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
    crate::flow_pace::build_index_with(&src, "", &b, budget, &mut |d, t| seen.push((d, t)))
        .unwrap();
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
    crate::flow_base::write_mzml_with(&src, None, &b, Picchi::File, Opzioni::default(), step)
        .unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    well_formed(&seen);
    let _ = std::fs::remove_dir_all(&d);
}
