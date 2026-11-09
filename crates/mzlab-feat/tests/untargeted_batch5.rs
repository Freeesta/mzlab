//! `untargeted::pipeline::run_file` on a synthetic run: the slabs, the kept chromatograms of the common-mode pass and the reused trace buffer leave the
//! features as they are, the broad bands come with their scans, SNR and width, and the table of the file keeps them (round trip).
#![cfg(feature = "untargeted")]

#[path = "untargeted_common/synth.rs"]
mod synth;

use mzlab_feat::untargeted::pipeline::{
    from_file_features, run_file, to_file_features, BaselineParams, UntParams,
};
use mzlab_feat::untargeted::Feature;

fn params(broad: bool, gt: bool, baseline: bool) -> UntParams {
    let mut p = UntParams::default();
    p.broad.enabled = broad;
    p.broad.gt = gt;
    p.baseline = baseline.then(BaselineParams::default);
    p
}

fn run(bytes: &[u8], p: &UntParams) -> Vec<Feature> {
    run_file(&mut synth::open(bytes.to_vec()), p).unwrap()
}

#[test]
fn slab_size_and_kept_chromatograms_do_not_change_the_features() {
    let bytes = synth::index_bytes(700, 120);
    for (broad, gt, baseline) in [
        (false, false, false),
        (true, false, false),
        (true, true, false),
        (true, true, true),
    ] {
        let mut p = params(broad, gt, baseline);
        let whole = run(&bytes, &p);
        assert!(whole.len() > 100, "{} features", whole.len());
        if broad {
            assert!(whole.iter().any(|f| f.broad), "no band: {broad} {gt}");
        }
        // many small slabs; with `gt` the chromatograms of the first pass no longer fit the quarter that is kept and the second pass builds them again
        for slab_bytes in [1 << 12, 1 << 16, 1 << 20] {
            p.slab_bytes = slab_bytes;
            assert_eq!(
                run(&bytes, &p),
                whole,
                "{broad} {gt} {baseline} slab {slab_bytes}"
            );
        }
    }
}

#[test]
fn bands_and_peaks_carry_their_scans_and_the_table_keeps_them() {
    let bytes = synth::index_bytes(700, 120);
    let list = run(&bytes, &params(true, true, false));
    let (narrow, broad): (Vec<&Feature>, Vec<&Feature>) = list.iter().partition(|f| !f.broad);
    assert!(!narrow.is_empty() && !broad.is_empty());
    for f in &list {
        assert!(f.scan_lo <= f.apex && f.apex <= f.scan_hi, "{f:?}");
        assert!(f.scan_hi < 700 && f.npts >= 1, "{f:?}");
        assert!(f.rt_start <= f.rt && f.rt <= f.rt_end);
    }
    assert!(broad.iter().all(|f| f.snr > 0.0 && f.fwhm > 0.0));
    assert!(narrow.iter().all(|f| f.snr == 0.0 && f.fwhm == 0.0));
    let table = to_file_features(&list);
    assert_eq!(table.apex.len(), list.len());
    for (i, f) in list.iter().enumerate() {
        assert_eq!(
            (
                table.apex[i],
                table.scan_lo[i],
                table.scan_hi[i],
                table.npts[i]
            ),
            (f.apex, f.scan_lo, f.scan_hi, f.npts)
        );
        assert_eq!((table.snr[i], table.fwhm[i]), (f.snr, f.fwhm));
    }
    // the area goes through counts x seconds and back: equal to the last bits
    let back = from_file_features(&table);
    assert_eq!(back.len(), list.len());
    for (b, f) in back.iter().zip(&list) {
        assert!((b.area - f.area).abs() <= 1e-12 * f.area.abs());
        assert_eq!(
            Feature {
                area: f.area,
                ..b.clone()
            },
            *f
        );
    }
}
