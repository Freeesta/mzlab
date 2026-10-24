//! Fragment tolerance in ppm: a fixed 0.01 Da window cannot tell two HR compounds whose fragments differ by 0.008 Da (95 ppm at m/z 85), 5 ppm can.
//! Synthetic spectra only. Also: a plain `f64` tolerance keeps the bits of the Da path.
mod common;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::scores::{match_peaks, modified_cosine, pair_scores, Spec};
use mzlab_lib::search::{identity, identity_with, IdentityOpts, Query, Tol};
use mzlab_lib::spec::{Meta, Spectrum};

const MZ: [f64; 4] = [85.0284, 127.0390, 301.1410, 455.2500];
const IT: [f64; 4] = [30.0, 100.0, 60.0, 40.0];

fn sp(mz: &[f64], name: &str) -> Spectrum {
    Spectrum {
        prec: 500.3,
        pol: 1,
        mz: mz.to_vec(),
        it: IT.to_vec(),
        meta: Meta {
            name: name.into(),
            ..Default::default()
        },
        rt: None,
    }
}

fn shifted(da: f64) -> Vec<f64> {
    MZ.iter().map(|m| m + da).collect()
}

fn with_error(ppm: f64) -> Vec<f64> {
    MZ.iter().map(|m| m * (1.0 + ppm * 1e-6)).collect()
}

fn lib_bytes() -> Vec<u8> {
    // the wrong compound goes first: a tie is decided by the library order
    let set = vec![sp(&shifted(0.008), "wrong"), sp(&MZ, "right")];
    build_in_memory(&set, BuildOpts::default()).unwrap().0
}

#[test]
fn ppm_separates_what_a_fixed_da_window_cannot() {
    let b = lib_bytes();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    let q = Query::new(500.3, 1, &with_error(3.0), &IT).unwrap();
    let o = IdentityOpts {
        prec_tol: Tol::Da(0.5),
        frag_da: 0.01,
        pol: 1,
        top: 10,
    };
    let name = |h: &mzlab_lib::search::Hit| lib.meta(h.index).unwrap().name;
    let da = identity(&lib, &q, &o).unwrap();
    let ppm = identity_with(&lib, &q, &o, FragTol::ppm(5.0)).unwrap();
    let (da_w, da_r) = (
        da.iter().find(|h| name(h) == "wrong").unwrap(),
        da.iter().find(|h| name(h) == "right").unwrap(),
    );
    let (pp_w, pp_r) = (
        ppm.iter().find(|h| name(h) == "wrong").unwrap(),
        ppm.iter().find(|h| name(h) == "right").unwrap(),
    );
    println!(
        "0.01 Da: wrong {:.4} right {:.4} first {}",
        da_w.entropy,
        da_r.entropy,
        name(&da[0])
    );
    println!(
        "5 ppm:   wrong {:.4} right {:.4} first {}",
        pp_w.entropy,
        pp_r.entropy,
        name(&ppm[0])
    );
    // the Da window sees both compounds equally well and the order of the library decides
    assert!((da_w.entropy - da_r.entropy).abs() < 1e-9);
    assert_eq!(name(&da[0]), "wrong");
    // the ppm window drops every fragment of the wrong one
    assert_eq!(pp_w.shared, 0);
    assert_eq!(pp_r.shared, 4);
    assert_eq!(name(&ppm[0]), "right");
    assert!(pp_r.entropy > 0.99);
}

#[test]
fn the_ppm_window_scales_with_the_mz_of_the_peak() {
    let (a_mz, a_it) = ([100.0, 800.0], [1.0, 1.0]);
    // 0.002 Da off: 20 ppm at m/z 100, 2.5 ppm at m/z 800
    let (b_mz, b_it) = ([100.002, 800.002], [1.0, 1.0]);
    let t = FragTol::ppm(5.0);
    assert_eq!(match_peaks(&a_mz, &a_it, &b_mz, &b_it, t), vec![(1, 1)]);
    assert_eq!(match_peaks(&a_mz, &a_it, &b_mz, &b_it, 0.01).len(), 2);
}

#[test]
fn a_da_tolerance_keeps_its_bits() {
    let a = Spec {
        mz: with_error(3.0),
        it: IT.to_vec(),
        prec: 500.3,
    };
    let b = Spec {
        mz: shifted(0.004),
        it: IT.to_vec(),
        prec: 500.3,
    };
    let want = pair_scores(&a, &b, 0.01);
    assert_eq!(want, pair_scores(&a, &b, FragTol::from_params(0.01, 0.0)));
    let m = modified_cosine(&a, &b, 0.01);
    let n = modified_cosine(&a, &b, FragTol::from_params(0.01, 0.0));
    assert_eq!(m.cosine.to_bits(), n.cosine.to_bits());
    assert_eq!(m.pairs, n.pairs);
}
