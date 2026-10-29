//! `scanlist`, `nearest_scan`, the spectra of a level and the unfiltered `chrom` answer byte for byte what they answered when they copied the
//! per-scan arrays of the section (the digest below was taken on that code); `bench_*` (ignored) prints the time of the routes on a big file.
//! Synthetic data only.

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::upload::prepare;

fn scan(i: usize) -> Spectrum {
    Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", i + 1),
        level: if i % 5 == 4 { 2 } else { 1 },
        rt: 0.01 * i as f64,
        polarity: 1,
        profile: false,
        filter: String::new(),
        precursor: (i % 5 == 4).then_some(150.0 + (i % 3) as f64),
        mz: (0..6).map(|b| 100.0 + b as f64 * 7.5).collect(),
        intensity: (0..6)
            .map(|b| 10.0 + ((i * 7 + b * 3) % 50) as f32)
            .collect(),
        wide: None,
    }
}

fn routes(total: usize) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "zd".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(total).unwrap();
    for i in 0..total {
        w.spectrum(&scan(i)).unwrap();
    }
    w.close_spectra().unwrap();
    let mzml = w.finish().unwrap();
    let size = mzml.len() as u64;
    let mut src = mzml.clone();
    let p = prepare(&mut src, size, "zd.mzML").unwrap();
    let (idx, _) = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

const QUERIES: [&str; 8] = [
    "/api/scanlist?k=0&level=1",
    "/api/scanlist?k=0&level=2",
    "/api/nearest_scan?k=0&level=1&rt=3.337",
    "/api/nearest_scan?k=0&level=2&rt=3.337&precursor=151",
    "/api/spectra?k=0&level=1&i0=2&i1=9",
    "/api/chrom?k=0&level=1",
    "/api/chrom?k=0&level=1&kind=bpc",
    "/api/chrom?k=0&level=2&prec=151",
];

fn answers(r: &mut Routes<Vec<u8>>) -> String {
    QUERIES
        .iter()
        .map(|q| {
            let (path, query) = q.split_once('?').unwrap();
            format!(
                "{q} => {}",
                r.handle(path, &Query::parse(query))
                    .unwrap()
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn routes_answer_as_before_without_copying_the_arrays() {
    let out = answers(&mut routes(600));
    assert!(
        out.contains("\"n\":480") && out.contains("\"n\":120"),
        "{out}"
    );
    assert_eq!(fnv(&out), DIGEST, "the answers changed");
}

#[test]
#[ignore]
fn bench_scanlist_and_nearest_scan() {
    let mut r = routes(200_000);
    for q in [
        "/api/scanlist?k=0&level=1",
        "/api/nearest_scan?k=0&level=1&rt=1000",
        "/api/chrom?k=0&level=1",
    ] {
        let (path, query) = q.split_once('?').unwrap();
        let query = Query::parse(query);
        r.handle(path, &query).unwrap(); // arrays read once, as after the first click
        let t = std::time::Instant::now();
        for _ in 0..20 {
            r.handle(path, &query).unwrap();
        }
        println!(
            "{q}: {:.3} ms/call",
            t.elapsed().as_secs_f64() * 1000.0 / 20.0
        );
    }
}

const DIGEST: u64 = 11906426660252247255;
