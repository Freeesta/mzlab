//! The DAD map JSON written in pieces against the one-shot encoding.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]
use super::*;

fn dad(nt: usize, nl: usize) -> DadData {
    DadData {
        time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
        twc_mau: vec![0.0; nt],
        wavelengths_nm: (0..nl).map(|j| 200.0 + j as f64).collect(),
        wl_start_nm: 200.0,
        wl_stop_nm: 200.0 + (nl - 1) as f64,
        wl_step_nm: 1.0,
        duration_min: nt as f64 * 0.01,
        matrix_mau: (0..nt * nl).map(|k| (k % 97) as f32).collect(),
    }
}

#[test]
fn small_matrix_is_sent_as_read() {
    let j = dad_to_map_json(&dad(40, 21));
    assert!(
        j.contains("\"nrt\":40")
            && j.contains("\"nmz\":21")
            && j.contains("\"agg\":\"none\"")
            && j.contains("\"dmz\":1,")
    );
}

#[test]
fn large_matrix_is_averaged_within_the_caps() {
    let (nt, nl) = (5500, 1201);
    let j = dad_to_map_json(&dad(nt, nl));
    let (ft, fl) = (nt.div_ceil(MAP_MAX_RT), nl.div_ceil(MAP_MAX_NM));
    assert!(
        j.contains(&format!("\"nrt\":{}", nt.div_ceil(ft)))
            && j.contains(&format!("\"nmz\":{}", nl.div_ceil(fl)))
    );
    assert!(
        nt.div_ceil(ft) <= MAP_MAX_RT
            && nl.div_ceil(fl) <= MAP_MAX_NM
            && j.contains("\"agg\":\"mean\"")
            && j.contains(&format!("\"dmz\":{fl},"))
    );
}

#[test]
fn the_map_is_the_one_shot_encoding_to_the_byte() {
    use base64::Engine;
    for (nt, nl) in [
        (1, 1),
        (2, 1),
        (40, 21),
        (1024, 3),
        (1025, 3),
        (3, 1025),
        (2000, 500),
        (5500, 1201),
    ] {
        let d = dad(nt, nl);
        let (vals, nrt, nmz, fl, reduced) = map_values(&d);
        let bytes: Vec<u8> = vals.iter().flat_map(|v| v.to_le_bytes()).collect();
        let want = format!(
            "{{\"data\":\"{}\",\"rt0\":{},\"rt1\":{},\"nrt\":{nrt},\"mz0\":{},\"mz1\":{},\"nmz\":{nmz},\"dmz\":{},\"level\":1,\"region\":false,\"agg\":\"{}\",\"log\":false,\"bg\":false,\"hr\":false,\"kind\":\"pda\"}}",
            base64::engine::general_purpose::STANDARD.encode(&bytes),
            d.time_min[0],
            d.time_min[nt - 1],
            d.wl_start_nm,
            d.wl_stop_nm,
            d.wl_step_nm * fl as f64,
            if reduced { "mean" } else { "none" }
        );
        assert_eq!(dad_to_map_json(&d), want, "{nt} x {nl}");
    }
}

#[test]
fn reduce_averages_blocks_and_keeps_the_edge() {
    // 3 x 3 in blocks of 2 x 2: the last row and column are blocks of one
    let m: Vec<f32> = (1..=9).map(|v| v as f32).collect();
    let (r, rt, rl) = reduce(&m, 3, 3, 2, 2);
    assert_eq!((rt, rl), (2, 2));
    assert_eq!(r, vec![3.0, 4.5, 7.5, 9.0]);
}
