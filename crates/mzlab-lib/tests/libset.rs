//! `LibSet`: three libraries searched together equal the union of the three searches; the duplicate sidecar groups the same spectrum found in two libraries.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::flash::FlashMode;
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::libset::{LibSet, Sidecar};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{analogs, identity_with, AnalogOpts, IdentityOpts, Query, Tol};
use mzlab_lib::spec::Spectrum;

fn open(set: &[Spectrum]) -> Library<Vec<u8>> {
    let (b, _) = build_in_memory(set, BuildOpts::default()).unwrap();
    Library::open(b, CACHE_BYTES).unwrap()
}

/// Libraries A = 0..250, B = 150..400, C = 350..500 of one synthetic set: A and B share 100 spectra, B and C 50.
fn three() -> (Vec<Spectrum>, LibSet<Vec<u8>>) {
    let all = synth_set(11, 500, 14);
    let set = LibSet::new(vec![
        open(&all[0..250]),
        open(&all[150..400]),
        open(&all[350..500]),
    ])
    .unwrap();
    (all, set)
}

fn query_of(s: &Spectrum) -> Query {
    let it: Vec<f64> =
        s.it.iter()
            .enumerate()
            .map(|(i, v)| v * (0.8 + 0.05 * (i % 5) as f64))
            .collect();
    Query::new(s.prec, 1, &s.mz, &it).unwrap()
}

const IDO: IdentityOpts = IdentityOpts {
    prec_tol: Tol::Da(25.0),
    frag_da: 0.02,
    pol: 1,
    top: 8,
};

#[test]
fn three_libraries_are_the_union_of_the_three_searches() {
    let (all, set) = three();
    assert_eq!(set.len(), 3);
    assert_eq!(set.all(), 0b111);
    let frag = FragTol::from(0.02);
    for s in all.iter().step_by(37).filter(|s| s.pol > 0) {
        let q = query_of(s);
        // identity: the best 8 of all the hits of the three libraries
        let mut want: Vec<(u16, u32, f64)> = Vec::new();
        for k in 0..3 {
            let o = IdentityOpts { top: 100, ..IDO };
            for h in identity_with(set.lib(k).unwrap(), &q, &o, frag).unwrap() {
                want.push((k as u16, h.index, h.entropy));
            }
        }
        want.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        want.truncate(8);
        let got = set.identity(&q, &IDO, frag, set.all()).unwrap();
        let got: Vec<(u16, u32, f64)> = got
            .iter()
            .map(|h| (h.lib, h.index, h.hit.entropy))
            .collect();
        assert_eq!(got.len(), want.len());
        for (a, b) in got.iter().zip(&want) {
            assert_eq!((a.0, a.1), (b.0, b.1));
            assert!((a.2 - b.2).abs() < 1e-12);
        }
        // a mask leaves the other libraries out
        let only_b = set.identity(&q, &IDO, frag, 0b010).unwrap();
        assert!(only_b.iter().all(|h| h.lib == 1));

        // analogues and the open search: the best of the three
        let ao = AnalogOpts {
            top: 5,
            ..AnalogOpts::default()
        };
        let mut want: Vec<(u16, u32, f64)> = Vec::new();
        for k in 0..3 {
            let ao = AnalogOpts {
                top: 50,
                ..ao.clone()
            };
            for h in analogs(set.lib(k).unwrap(), &q, &ao).unwrap() {
                want.push((k as u16, h.index, h.mc.cosine));
            }
        }
        want.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        want.truncate(5);
        let got = set.analogs(&q, &ao, set.all()).unwrap();
        let got: Vec<(u16, u32)> = got.iter().map(|h| (h.lib, h.index)).collect();
        assert_eq!(got, want.iter().map(|w| (w.0, w.1)).collect::<Vec<_>>());

        let open = set
            .open(&q, FlashMode::Hybrid, frag, 1, 10, set.all())
            .unwrap();
        let mut want: Vec<(u16, u32, f64)> = Vec::new();
        for k in 0..3 {
            let lib = set.lib(k).unwrap();
            let r = lib.ranges(1, f64::NEG_INFINITY, f64::INFINITY).unwrap();
            for h in
                mzlab_lib::flash::flash_scores_with(lib, &q, FlashMode::Hybrid, frag, &r).unwrap()
            {
                want.push((k as u16, h.index, h.score));
            }
        }
        want.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        want.truncate(10);
        let got: Vec<(u16, u32)> = open.iter().map(|h| (h.lib, h.index)).collect();
        assert_eq!(got, want.iter().map(|w| (w.0, w.1)).collect::<Vec<_>>());
    }
}

#[test]
fn more_than_sixteen_libraries_are_refused() {
    let one = synth_set(1, 3, 6);
    let libs: Vec<_> = (0..17).map(|_| open(&one)).collect();
    assert_eq!(LibSet::new(libs).err().unwrap().key, "lib.err.tooLarge");
}

#[test]
fn the_sidecar_groups_the_same_spectrum_found_in_two_libraries() {
    let (all, mut set) = three();
    let side = set.dedup().unwrap();
    // 500 distinct spectra over 650 entries
    assert_eq!(side.n_groups(), 500);
    let bytes = side.to_bytes();
    assert!(!set.attach_sidecar(&bytes[..bytes.len() - 1]));
    assert!(set.attach_sidecar(&bytes));
    assert_eq!(set.sidecar(), Some(&side));
    // another set of files does not accept it
    let other = LibSet::new(vec![open(&all[0..250])]).unwrap();
    assert!(Sidecar::from_bytes(&bytes, &other.fingerprints()).is_none());

    // a spectrum held by A and B is listed once, with both libraries named
    let s = &all[200];
    let q = query_of(s);
    let frag = FragTol::from(0.02);
    let hits = set.identity(&q, &IDO, frag, set.all()).unwrap();
    let best = &hits[0];
    assert_eq!(best.hit.prec, s.prec);
    assert_eq!(best.libs, 0b011);
    let groups: std::collections::HashSet<_> = hits.iter().map(|h| h.group).collect();
    assert_eq!(groups.len(), hits.len(), "each group once");
    // the mask limits the libraries reported
    let hits = set.identity(&q, &IDO, frag, 0b110).unwrap();
    assert_eq!(hits[0].libs, 0b010);
    // without the sidecar the duplicate is listed twice
    let (_, plain) = three();
    let hits = plain.identity(&q, &IDO, frag, plain.all()).unwrap();
    assert_eq!(hits.iter().filter(|h| h.hit.prec == s.prec).count(), 2);
}
