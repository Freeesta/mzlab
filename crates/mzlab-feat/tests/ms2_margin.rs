//! The MS2 pairing window of a row is its RT range widened by the trigger margin (`Ms2Params::margin_s`): a DDA scan triggered just outside the
//! narrow integrated edges but inside the margin is paired, one just outside the margin is not. The file is a synthetic mzML made here.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::ms2::{link, Ms2Params};
use mzlab_feat::table::FileFeatures;
use mzlab_feat::trace::Params;

const MZ: f64 = 300.0;
/// MS1 spacing, minutes (6 s).
const STEP: f64 = 0.1;

fn spectrum(index: usize, level: u8, rt: f64, precursor: Option<f64>) -> Spectrum {
    Spectrum {
        index,
        id: format!("scan={}", index + 1),
        level,
        rt,
        polarity: 1,
        profile: false,
        filter: format!("+ c ESI Full ms{level}"),
        precursor,
        mz: vec![MZ, 500.0],
        intensity: vec![1e6, 1e5],
        wide: None,
    }
}

/// MS1 scans at 1.0, 1.1 ... 2.0 min with the ion in each; an MS2 of the ion after the MS1 scans numbered in `triggers`.
fn file(triggers: &[usize]) -> Vec<u8> {
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(11 + triggers.len()).unwrap();
    let mut n = 0;
    for k in 0..11 {
        let rt = 1.0 + k as f64 * STEP;
        w.spectrum(&spectrum(n, 1, rt, None)).unwrap();
        n += 1;
        if triggers.contains(&k) {
            w.spectrum(&spectrum(n, 2, rt + 0.01, Some(MZ))).unwrap();
            n += 1;
        }
    }
    w.close_spectra().unwrap();
    let bytes = w.finish().unwrap();
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, "dda").unwrap();
    build_mzml_to_vec(&p.table, &bytes, 64 << 20).unwrap().0
}

/// One row of the ion with narrow edges: MS1 scans 4..=6 (1.4 to 1.6 min).
fn row() -> FileFeatures {
    let mut f = FileFeatures::default();
    f.mz.push(MZ);
    f.rt.push(1.5);
    f.left.push(1.4);
    f.right.push(1.6);
    f.apex.push(5);
    f.scan_lo.push(4);
    f.scan_hi.push(6);
    f.area.push(1e6);
    f.height.push(1e6);
    f.npts.push(3);
    f.flags.push(0);
    f.iso_group.push(0);
    f.charge.push(0);
    for v in [
        &mut f.shape,
        &mut f.snr,
        &mut f.fwhm,
        &mut f.asym,
        &mut f.purity,
    ] {
        v.push(0.0);
    }
    f.gaps.push(0);
    f.qreason.push(0);
    f.ms2_n.push(0);
    f.area_win.push(1e6);
    f.family_id.push(0);
    f.role.push(0);
    f.alt.push(0);
    f.dt = 6.0;
    f
}

/// Number of MS2 scans paired to the row with margin `margin_s`.
fn paired(triggers: &[usize], margin_s: f64) -> usize {
    let bytes = file(triggers);
    let mut ix = Index::open(bytes.clone(), bytes.len() as u64).unwrap();
    let mut f = row();
    let q = Ms2Params {
        margin_s,
        ..Ms2Params::default()
    };
    let p = Params {
        polarity: Some(1),
        ..Params::default()
    };
    let links = link(&mut ix, &mut f, &p, &q).unwrap();
    assert_eq!(f.ms2_n[0] as usize, links.of(0).0.len());
    links.of(0).0.len()
}

#[test]
fn a_trigger_inside_the_row_is_paired_without_a_margin() {
    assert_eq!(paired(&[5], 1e-6), 1);
}

#[test]
fn a_trigger_just_outside_the_narrow_bounds_needs_the_margin() {
    // MS1 scan 3 (1.3 min) is 6 s before the left edge
    assert_eq!(paired(&[3], 1e-6), 0);
    assert_eq!(paired(&[3], 10.0), 1);
}

#[test]
fn a_trigger_beyond_the_margin_is_not_paired() {
    // MS1 scan 1 (1.1 min) is 18 s before the left edge, MS1 scan 8 (1.8 min) is 12 s after the right one
    assert_eq!(paired(&[1], 10.0), 0);
    assert_eq!(paired(&[8], 10.0), 0);
    assert_eq!(paired(&[8], 13.0), 1);
}

#[test]
fn the_default_margin_is_one_cycle_of_the_file() {
    assert_eq!(paired(&[3, 7], 0.0), 2);
    assert_eq!(paired(&[2, 8], 0.0), 0);
}
