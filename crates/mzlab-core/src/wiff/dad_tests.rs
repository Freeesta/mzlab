//! The DAD trace at one wavelength (no band: the channel nearest to it within half a step, the lower one of two equally near) and the TDC bin to m/z conversion.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use super::*;

fn dad(wl: &[f64], step: f64) -> DadData {
    // one row, every channel holds its own wavelength so the trace says which channel was taken
    DadData {
        time_min: vec![0.0],
        twc_mau: vec![0.0],
        wavelengths_nm: wl.to_vec(),
        wl_start_nm: wl[0],
        wl_stop_nm: wl[wl.len() - 1],
        wl_step_nm: step,
        duration_min: 1.0,
        matrix_mau: wl.iter().map(|&w| w as f32).collect(),
    }
}

#[test]
fn no_band_takes_the_nearest_channel_within_half_a_step() {
    let d = dad(&[253.0, 255.0, 257.0], 2.0);
    // 254 is as near to 253 as to 255: the lower one
    assert_eq!(dad_wl_chrom(&d, 254.0, None, 0).unwrap().intensity, [253.0]);
    // 254.6 is nearer to 255 although 253 also lies within a step
    assert_eq!(dad_wl_chrom(&d, 254.6, None, 0).unwrap().intensity, [255.0]);
    // 256.1 is within half a step of 255 and of 257: 257 is nearer
    assert_eq!(dad_wl_chrom(&d, 256.1, None, 0).unwrap().intensity, [257.0]);
    assert!(dad_wl_chrom(&d, 259.5, None, 0).is_none());
    // the order of the channels does not change the tie
    let r = dad(&[257.0, 255.0, 253.0], 2.0);
    assert_eq!(dad_wl_chrom(&r, 254.0, None, 0).unwrap().intensity, [253.0]);
}

#[test]
fn a_band_averages_the_channels_inside_it() {
    let d = dad(&[253.0, 255.0, 257.0], 2.0);
    let c = dad_wl_chrom(&d, 255.0, Some(4.0), 0).unwrap();
    assert_eq!(c.intensity, [255.0]);
    let c = dad_wl_chrom(&d, 254.0, Some(2.0), 0).unwrap();
    assert_eq!(c.intensity, [254.0]);
}

#[test]
fn a_bin_before_t0_has_no_mz() {
    let t = TofSampleData {
        tdc_bin_time_ns: 0.5,
        initial_a: 2.0,
        initial_t0: 10.0,
        scan_calibration: Vec::new(),
        itc: Vec::new(),
        tdc_statistics_count: 0,
    };
    assert_eq!(t.bin_to_mz(0, 0), 0.0);
    assert_eq!(t.bin_to_mz(0, 20), 0.0);
    // (30 * 0.5 - 10) * 2 = 10: 100
    assert_eq!(t.bin_to_mz(0, 30), 100.0);
    let mz: Vec<f64> = (0..100).map(|b| t.bin_to_mz(0, b)).collect();
    assert!(mz.windows(2).all(|w| w[0] <= w[1]), "monotone");
}

#[test]
fn a_row_count_that_wraps_a_32_bit_length_does_not_fit() {
    // 32 + 8 * (2^29 + 1) is 40 in 32-bit arithmetic: the site would pass a check of 40 bytes that the app fails, then loop 2^29 times
    let n = (1usize << 29) + 1;
    assert!(span(32, 8, n).is_none_or(|v| v as u64 == 32 + 8 * n as u64));
    assert_eq!(span(32, 8, 3), Some(56));
    assert_eq!(span(0, 8, usize::MAX), None);
}

#[test]
fn every_tdc_token_moves_the_bin_and_keeps_its_count() {
    // header: marker, start bin 0, one byte the reader skips; then each token kind once
    let mut b = vec![0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0];
    b.push(0x01); // count 1 at bin 0
    b.extend([0xfc, 3]); // skip 3 steps of 4 ticks
    b.push(0x02); // count 2 at bin 16
    b.extend([0xfd, 0x10, 0x00]); // skip 16 steps
    b.push(0x7e);
    b.extend(70_000u32.to_le_bytes()); // count 70 000 at bin 84
    b.push(0x7d);
    b.extend(300u16.to_le_bytes()); // count 300 at bin 88
    b.push(0xfe);
    b.extend(1u32.to_le_bytes()); // skip 1 step
    b.extend([0x83, 0x03]); // skip 3 steps, count 3 at bin 108
    b.push(0xff);
    assert_eq!(
        decode_tof_block(&b).unwrap(),
        vec![(0, 1), (16, 2), (84, 70_000), (88, 300), (108, 3)]
    );
    // a token outside the table and a count cut by the end of the block are errors
    assert!(decode_tof_block(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0x80]).is_err());
    assert!(decode_tof_block(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0x7e, 1]).is_err());
}
