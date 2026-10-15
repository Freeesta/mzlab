//! The pyramid of tiles of `maptiles` gives, for any level and any aligned rectangle, exactly the cells that `region_map` computes on the same bins
//! (Rust against Rust; the file is synthetic: evenly spaced scans and m/z on a binary grid, so every cell edge is exact). Also: the answer of
//! `api/maptiles`, the bytes round trip, and the refusal of bytes of another index.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::idxread::Index;
use mzlab_core::maproute::map_route;
use mzlab_core::maptiles::{cell_width, Pyramid};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::originroute::ItemMeta;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;
use std::collections::HashMap;

const SCANS: usize = 520;

fn scan(i: usize) -> Spectrum {
    let n = 60;
    // m/z on a grid of 1/64 Da over 0..300, intensities that repeat and collide in cells
    let mz: Vec<f64> = {
        let mut v: Vec<f64> = (0..n)
            .map(|b| ((i * 37 + b * 411) % 19200) as f64 / 64.0)
            .collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v.dedup();
        v
    };
    let intensity = (0..mz.len())
        .map(|b| 1.0 + ((i * 13 + b * 7919) % 100_003) as f32 * 1.37)
        .collect();
    Spectrum {
        index: i,
        id: format!("scan={}", i + 1),
        level: 1,
        rt: 0.5 * i as f64,
        polarity: 1,
        profile: false,
        filter: String::new(),
        precursor: None,
        mz,
        intensity,
        wide: None,
    }
}

fn routes() -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "tiles".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(SCANS).unwrap();
    for i in 0..SCANS {
        w.spectrum(&scan(i)).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "tiles.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

#[test]
fn every_level_equals_the_region_map_on_the_same_bins() {
    let mut r = routes();
    let p = r.pyramid_for_test(1).expect("a pyramid");
    assert!(p.max_level() >= 4);
    let mut compared = 0usize;
    for l in 0..=5usize {
        let w = cell_width(l);
        let step = 1usize << l;
        // the whole rows as a power-of-two count of scans: region_map divides the span by `nrt` exactly only then
        let nrt = SCANS.next_power_of_two() / step;
        for (c0, ncols) in [(0usize, 64usize), (3, 64), (17, 32), (60, 128)] {
            let (mz0, mz1) = (c0 as f64 * w, (c0 + ncols) as f64 * w);
            let want = r
                .region_map(
                    1,
                    (0.0, 0.5 * SCANS.next_power_of_two() as f64),
                    nrt,
                    (mz0, mz1),
                    (ncols, false),
                    false,
                )
                .unwrap();
            let got = p.rect(l, (0, nrt), (c0, c0 + ncols));
            assert_eq!(got.len(), want.len(), "level {l} cols {c0}+{ncols}");
            for (i, (a, b)) in got.iter().zip(&want).enumerate() {
                assert_eq!(
                    a.to_bits(),
                    b.to_bits(),
                    "level {l} cols {c0}+{ncols} cell {i}: {a} against {b}"
                );
            }
            compared += got.iter().filter(|v| **v > 0.0).count();
        }
    }
    assert!(
        compared > 5000,
        "the test must compare real cells, got {compared}"
    );
}

#[test]
fn bytes_round_trip_and_refuse_another_index() {
    let mut r = routes();
    let p = r.pyramid_for_test(1).unwrap();
    let b = p.to_bytes(42);
    let q = Pyramid::from_bytes(&b, 42).expect("same stamp");
    for l in [0, 2, 5] {
        assert_eq!(q.rect(l, (0, 64), (0, 200)), p.rect(l, (0, 64), (0, 200)));
    }
    assert!(Pyramid::from_bytes(&b, 43).is_none());
    assert!(Pyramid::from_bytes(&b[..b.len() - 3], 42).is_none());
    assert!(Pyramid::from_bytes(b"", 42).is_none());
}

#[test]
fn the_route_answers_with_aligned_cells_that_fit_the_request() {
    let mut files = HashMap::new();
    files.insert(0u32, routes());
    let items = [ItemMeta {
        slot: 0,
        label: "a".into(),
        time: None,
        full: true,
    }];
    let ask = |files: &mut HashMap<u32, Routes<Vec<u8>>>, q: &str| {
        map_route("/api/maptiles", &items, files, &Query::parse(q))
            .unwrap()
            .expect("served")
    };
    let j: serde_json::Value = serde_json::from_str(&ask(
        &mut files,
        "k=0&level=1&rt0=0&rt1=260&mz0=0&mz1=300&nrt=64&nmz=128",
    ))
    .unwrap();
    let (nrt, nmz) = (j["nrt"].as_u64().unwrap(), j["nmz"].as_u64().unwrap());
    assert!(nrt <= 64 && nmz <= 128, "{nrt} x {nmz}");
    assert_eq!(j["rt"].as_array().unwrap().len() as u64, nrt + 1);
    assert_eq!(
        j["dmz"].as_f64().unwrap(),
        cell_width(j["tile_level"].as_u64().unwrap() as usize)
    );
    // a deep view stays at level 0: one row per scan
    let d: serde_json::Value = serde_json::from_str(&ask(
        &mut files,
        "k=0&level=1&rt0=100&rt1=110&mz0=100&mz1=108&nrt=2048&nmz=1024",
    ))
    .unwrap();
    assert_eq!(d["tile_level"].as_u64(), Some(0));
    assert_eq!(d["nrt"].as_u64(), Some(21));
    // an empty rectangle or a bad parameter is refused like the other map routes
    assert!(map_route(
        "/api/maptiles",
        &items,
        &mut files,
        &Query::parse("k=0&rt0=5&rt1=5&mz0=0&mz1=9")
    )
    .unwrap()
    .is_none());
}

#[test]
fn saved_pyramid_serves_the_same_answers_and_shares_the_cache_budget() {
    let query = Query::parse("k=0&level=1&rt0=0&rt1=260&mz0=0&mz1=300&nrt=64&nmz=128");
    let items = [ItemMeta {
        slot: 0,
        label: "a".into(),
        time: None,
        full: true,
    }];
    let mut a = HashMap::new();
    a.insert(0u32, routes());
    let want = map_route("/api/maptiles", &items, &mut a, &query)
        .unwrap()
        .unwrap();
    let bytes = a.get_mut(&0).unwrap().tiles_export(1).unwrap();
    assert!(!bytes.is_empty());
    // the same bytes in a fresh engine answer without building; another index or damaged bytes are refused
    let mut b = HashMap::new();
    b.insert(0u32, routes());
    assert!(b.get_mut(&0).unwrap().tiles_import(1, &bytes));
    assert_eq!(
        map_route("/api/maptiles", &items, &mut b, &query)
            .unwrap()
            .unwrap(),
        want
    );
    assert!(!b
        .get_mut(&0)
        .unwrap()
        .tiles_import(1, &bytes[..bytes.len() / 2]));
    let mut other = routes();
    other.idx.sections[0].n_peaks += 1;
    assert!(!other.tiles_import(1, &bytes));
    // the pyramid counts in the byte budget of the one LRU and goes when it does not fit
    let held = mzlab_core::routes::tiles_cache_bytes(&b);
    assert!(held > 0 && mzlab_core::routes::cache_bytes(&b) >= held);
    mzlab_core::routes::trim_cache(&mut b, 0);
    assert_eq!(mzlab_core::routes::tiles_cache_bytes(&b), 0);
    assert_eq!(
        map_route("/api/maptiles", &items, &mut b, &query)
            .unwrap()
            .unwrap(),
        want
    );
}
