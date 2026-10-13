//! The one LRU of the engine: after any number of routes `cache_bytes() <= cap`, and every answer is the same before and after the eviction
//! (an evicted entry is read again from the index). Synthetic data only.

use std::collections::HashMap;

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{cache_bytes, trim_cache, Query, Routes};
use mzlab_core::upload::prepare;

fn scan(i: usize, shift: f64) -> Spectrum {
    let n = 40;
    Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
        level: 1,
        rt: 0.05 * i as f64,
        polarity: 1,
        profile: false,
        filter: String::new(),
        precursor: None,
        mz: (0..n).map(|b| 100.0 + shift + b as f64 * 1.5).collect(),
        intensity: (0..n)
            .map(|b| 10.0 + ((i * 7 + b * 3) % 50) as f32)
            .collect(),
        wide: None,
    }
}

fn routes(shift: f64) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "lru".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let total = 600;
    w.open_spectra(total).unwrap();
    for i in 0..total {
        w.spectrum(&scan(i, shift)).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "lru.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

const QUERIES: [&str; 4] = [
    "/api/chrom?k=0&level=1",
    "/api/scanlist?k=0&level=1",
    "/api/filters?k=0",
    "/api/chrom?k=0&level=1&mz0=101&mz1=130",
];

fn ask(files: &mut HashMap<u32, Routes<Vec<u8>>>) -> Vec<String> {
    let mut out = Vec::new();
    for k in [0u32, 1, 2] {
        for q in QUERIES {
            let (path, query) = q.split_once('?').unwrap();
            out.push(
                files
                    .get_mut(&k)
                    .unwrap()
                    .handle(path, &Query::parse(query))
                    .unwrap()
                    .unwrap_or_default(),
            );
        }
        let r = files.get_mut(&k).unwrap();
        out.push(r.xic(110.0, 0.4, 1).unwrap());
    }
    out
}

#[test]
fn lru_keeps_the_cap_and_the_answers() {
    let mut files: HashMap<u32, Routes<Vec<u8>>> = (0..3).map(|k| (k, routes(k as f64))).collect();
    let before = ask(&mut files);
    let held = cache_bytes(&files);
    assert!(held > 0, "the routes cached nothing");
    for cap in [held * 3 / 4, held / 2, held / 8, 0] {
        let dropped = trim_cache(&mut files, cap);
        assert!(dropped > 0, "nothing dropped at cap {cap}");
        assert!(
            cache_bytes(&files) <= cap,
            "cache {} above cap {cap}",
            cache_bytes(&files)
        );
        assert_eq!(
            ask(&mut files),
            before,
            "answers changed after evicting down to {cap}"
        );
        // the least recently used entry goes first: the entries the last round used are the ones that stay
        trim_cache(&mut files, cap);
        assert!(cache_bytes(&files) <= cap);
    }
    trim_cache(&mut files, 0);
    assert_eq!(cache_bytes(&files), 0);
}
