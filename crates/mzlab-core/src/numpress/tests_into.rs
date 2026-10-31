//! The `_into` encoders write the bytes of the vector ones.

use super::*;

/// The encoders as they were before the `_into` forms (vectors of integers and half bytes): the new ones must write the same bytes.
mod reference {
    fn put_int(x: u32, halves: &mut Vec<u8>) {
        let mask = 0xf000_0000u32;
        let init = x & mask;
        if init == 0 {
            let l = (0..8).find(|&i| x & (mask >> (4 * i)) != 0).unwrap_or(8);
            halves.push(l as u8);
            halves.extend((l..8).map(|i| ((x >> (4 * (i - l))) & 0xf) as u8));
        } else if init == mask {
            let l = (0..8)
                .find(|&i| x & (mask >> (4 * i)) != mask >> (4 * i))
                .unwrap_or(7);
            halves.push((l + 8) as u8);
            halves.extend((l..8).map(|i| ((x >> (4 * (i - l))) & 0xf) as u8));
        } else {
            halves.push(0);
            halves.extend((0..8).map(|i| ((x >> (4 * i)) & 0xf) as u8));
        }
    }

    fn pack(halves: &[u8], out: &mut Vec<u8>) {
        for p in halves.chunks(2) {
            out.push((p[0] << 4) | p.get(1).copied().unwrap_or(0x8));
        }
    }

    pub fn linear(values: &[f64], fp: f64) -> Vec<u8> {
        let mut out = fp.to_be_bytes().to_vec();
        let ints: Vec<i64> = values.iter().map(|v| (v * fp + 0.5) as i64).collect();
        for v in ints.iter().take(2) {
            out.extend((*v as u32).to_le_bytes());
        }
        let mut halves = Vec::new();
        for i in 2..ints.len() {
            let extra = ints[i - 1] + (ints[i - 1] - ints[i - 2]);
            put_int((ints[i] - extra) as i32 as u32, &mut halves);
        }
        pack(&halves, &mut out);
        out
    }

    pub fn pic(values: &[f64]) -> Vec<u8> {
        let mut halves = Vec::new();
        for v in values {
            put_int((v + 0.5) as u32, &mut halves);
        }
        let mut out = Vec::new();
        pack(&halves, &mut out);
        out
    }

    pub fn slof(values: &[f64], fp: f64) -> Vec<u8> {
        let mut out = fp.to_be_bytes().to_vec();
        for v in values {
            out.extend((((v + 1.0).ln() * fp + 0.5) as u16).to_le_bytes());
        }
        out
    }

    pub fn slof_fixed_point(values: &[f64]) -> f64 {
        let max = values.iter().copied().fold(0.0, f64::max);
        (65535.0 / (max + 1.0).ln().max(1e-9))
            .floor()
            .clamp(1.0, 65535.0)
    }
}

#[test]
fn the_into_encoders_write_the_bytes_of_the_vector_ones_on_ten_thousand_random_spectra() {
    let mut seed = 99u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64
    };
    // a buffer that already holds bytes: the encoders append
    let mut buf = vec![7u8; 3];
    for k in 0..10_000 {
        let n = (rnd() * 40.0) as usize + if k % 100 == 0 { 0 } else { 1 };
        let mut mz = 50.0 + rnd() * 100.0;
        let mzs: Vec<f64> = (0..n)
            .map(|_| {
                mz += rnd() * 5.0;
                mz
            })
            .collect();
        let ints: Vec<f64> = (0..n).map(|_| (rnd() * rnd() * 1e7).floor()).collect();
        let narrow: Vec<f32> = ints.iter().map(|&x| x as f32).collect();
        let wide: Vec<f64> = narrow.iter().map(|&x| f64::from(x)).collect();

        let fp = linear_fixed_point(&mzs);
        buf.truncate(3);
        encode_linear_into(&mut buf, &mzs, fp);
        assert_eq!(&buf[3..], &reference::linear(&mzs, fp)[..], "linear {k}");
        assert_eq!(encode_linear(&mzs, fp), reference::linear(&mzs, fp));

        let fp = slof_fixed_point(&ints);
        assert_eq!(fp, reference::slof_fixed_point(&ints));
        buf.truncate(3);
        encode_slof_into(&mut buf, &ints, fp);
        assert_eq!(&buf[3..], &reference::slof(&ints, fp)[..], "slof {k}");

        let fp32 = slof_fixed_point_f32(&narrow);
        assert_eq!(fp32, slof_fixed_point(&wide));
        buf.truncate(3);
        encode_slof_f32_into(&mut buf, &narrow, fp32);
        assert_eq!(&buf[3..], &reference::slof(&wide, fp32)[..], "slof f32 {k}");

        assert_eq!(encode_pic(&ints), reference::pic(&ints), "pic {k}");
    }
}
