//! The whole-file maps of the session grid and the run background read from the per-scan cells (`mapscan`) are the maps that the peaks give, bit by bit:
//! the same requests with the cells switched off (`set_scan_cells(false)`) are the reference. Files of unit and of high resolution with peaks on, just
//! beside and away from the cell edges, ties, zeros and sessions of several files whose ranges move the grid.

use std::collections::HashMap;

use mzlab_core::idxread::Index;
use mzlab_core::maproute::map_route;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::originroute::ItemMeta;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;

struct Caso(u64);
impl Caso {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// m/z of a peak: mostly anywhere in `lo..hi`, a third of them at a cell edge `n + phase` moved by a few rounding steps.
fn mz_of(r: &mut Caso, lo: f64, hi: f64, phase: f64, hr: bool) -> f64 {
    let m = lo + r.unit() * (hi - lo);
    if r.next().is_multiple_of(4) {
        return m;
    }
    // the edge as `session_grid` writes it (`n - 0.2`) or as the phase adds it
    let edge = if phase > 0.0 && r.next().is_multiple_of(2) {
        (m + 0.2).round() - 0.2
    } else {
        (m - phase).round() + phase
    };
    let jitter = [
        0.0, 1e-15, -1e-15, 1e-13, -1e-13, 1e-9, -1e-9, 1e-7, -1e-7, 1e-6, -1e-6, 1e-5, -1e-5,
        2e-4, -2e-4, 3e-4, -3e-4,
    ][(r.next() % 17) as usize];
    let v = edge + jitter;
    if hr {
        v as f32 as f64
    } else {
        v
    }
}

fn file(seed: u64, scans: usize, (lo, hi): (f64, f64), rt0: f64, hr: bool) -> Routes<Vec<u8>> {
    let mut r = Caso(seed);
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "scan cells".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(scans).unwrap();
    for i in 0..scans {
        let mut mz: Vec<f64> = (0..40 + (r.next() % 60) as usize)
            .map(|_| mz_of(&mut r, lo, hi, if hr { 0.0 } else { 0.8 }, hr))
            .collect();
        mzlab_core::sortc::slice_by(&mut mz, |a, b| a.total_cmp(b));
        mz.dedup();
        let intensity = (0..mz.len())
            .map(|_| {
                [0.0, 1.0, 1.0, 7.5, 3.25][(r.next() % 5) as usize]
                    * (1.0 + (r.next() % 1000) as f32)
            })
            .collect();
        let filter = if hr {
            "FTMS + p ESI Full ms [100.00-1000.00]"
        } else {
            "ITMS + c ESI Full ms [100.00-1000.00]"
        };
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: rt0 + 0.037 * i as f64,
            polarity: 1,
            profile: false,
            filter: filter.into(),
            precursor: None,
            mz,
            intensity,
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let p = prepare(&mut mzml.clone(), size, "cells.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

fn session(hr: bool, cells: bool) -> (HashMap<u32, Routes<Vec<u8>>>, Vec<ItemMeta>) {
    let ranges = [(100.3, 640.9), (96.7, 800.2), (250.0, 1000.0)];
    let mut files = HashMap::new();
    for (k, rg) in ranges.iter().enumerate() {
        let mut r = file(11 + k as u64, 220 + 40 * k, *rg, 0.3 * k as f64, hr);
        let si = r.idx.section_of(1, None).unwrap();
        assert_eq!(
            r.idx.sections[si].f4, hr,
            "the section has the precision of the resolution"
        );
        r.set_scan_cells(cells);
        files.insert(k as u32, r);
    }
    let items = (0..3)
        .map(|k| ItemMeta {
            slot: k,
            label: format!("f{k}"),
            time: None,
            full: true,
        })
        .collect();
    (files, items)
}

fn same(hr: bool) {
    let queries = [
        "k=0&level=1",
        "k=1&level=1",
        "k=2&level=1",
        "k=1&level=1&bg=1",
        "k=2&level=1&bg=1",
        "k=0&level=1&agg=max&bg=1",
        "k=0&level=1&agg=mean",
        "k=1&level=1&ref=0",
        "k=0&level=1&rt0=1&rt1=9&mz0=150&mz1=400&nrt=64&nmz=128",
        "k=0&level=1&rt0=1&rt1=9&mz0=150&mz1=400&nrt=64&nmz=250&bg=1",
        "k=0&level=1&rt0=1&rt1=9&mz0=150&mz1=400&nrt=64&nmz=250",
        "k=1&level=1&rt0=0&rt1=12&mz0=100&mz1=1000&nrt=300&nmz=300",
        "k=1&level=1&rt0=2&rt1=5&mz0=120&mz1=880&nrt=50&nmz=190&bg=1",
        "k=2&level=1&rt0=0&rt1=12&mz0=250&mz1=1000&nrt=100&nmz=750",
        "k=2&level=1&rt0=3&rt1=7.5&mz0=300&mz1=900&nrt=77&nmz=100",
        "k=2&level=1&rt0=0&rt1=12&mz0=250&mz1=1000&nrt=100&nmz=750&mzscale=log",
    ];
    let (mut on, items) = session(hr, true);
    let (mut off, _) = session(hr, false);
    let mut compared = 0usize;
    for q in queries {
        let (a, b) = (
            map_route("/api/map", &items, &mut on, &Query::parse(q)).unwrap(),
            map_route("/api/map", &items, &mut off, &Query::parse(q)).unwrap(),
        );
        assert!(a.is_some(), "{q}: no answer");
        assert_eq!(a, b, "hr={hr} {q}");
        compared += a.map_or(0, |s| s.len());
    }
    assert!(compared > 20_000, "the test must compare real maps");
    let built: usize = on.values().map(|r| r.cell_tables()).sum();
    assert!(built >= 3, "the cells must have answered ({built} tables)");
    assert_eq!(off.values().map(|r| r.cell_tables()).sum::<usize>(), 0);
}

#[test]
fn unit_resolution_maps_equal_the_peaks() {
    same(false);
}

#[test]
fn high_resolution_maps_equal_the_peaks() {
    same(true);
}

#[test]
fn warm_builds_the_table_and_sends_nothing() {
    let (mut files, items) = session(false, true);
    let a = map_route(
        "/api/map",
        &items,
        &mut files,
        &Query::parse("k=1&level=1&warm=1"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(a, "{\"warm\":true}");
    assert_eq!(files[&1].cell_tables(), 1);
}
