//! Route tests of the channels engine (`api/canali`, `canale=`) on in-memory indexes.

use super::tests::*;
use super::*;
use crate::idxread::Index;
use crate::mzidx::build_mzml_to_vec;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};
use crate::upload::prepare;

/// An mzML (Thermo-style filter strings) of the cycles `(level, filter, precursor)`, one cycle every 0.1 min.
fn indice(cicli: usize, scansioni: &[(u8, &str, Option<f64>)]) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "canali".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(cicli * scansioni.len()).unwrap();
    let mut i = 0;
    for c in 0..cicli {
        for (level, filter, prec) in scansioni {
            let rt = c as f64 * 0.1 + i as f64 * 1e-4;
            let mz = vec![100.0, 150.0, 222.0];
            // the signal at 150 belongs to the precursor 222, at 100 to the precursor 300
            let it = match prec {
                Some(p) if *p > 250.0 => vec![500.0, 10.0, 10.0],
                Some(_) => vec![10.0, 800.0 + c as f64, 10.0],
                None => vec![1.0, 2.0, 3.0],
            };
            w.spectrum(&Spectrum {
                index: i,
                id: format!("scan={}", i + 1),
                level: *level,
                rt,
                polarity: 1,
                filter: filter.to_string(),
                precursor: *prec,
                mz,
                intensity: it.iter().map(|x| *x as f32).collect(),
                ..Default::default()
            })
            .unwrap();
            i += 1;
        }
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    let mut src = b.clone();
    let p = prepare(&mut src, b.len() as u64, "c.mzML").unwrap();
    let idx = build_mzml_to_vec(&p.table, &b, 64 << 20).unwrap().0;
    let n = idx.len() as u64;
    Routes::new(Index::open(idx, n).unwrap())
}

const FULL: &str = "ITMS + c ESI Full ms [50.00-400.00]";
const MS2_A: &str = "ITMS + c ESI Full ms2 222.10@cid35.00 [50.00-250.00]";
/// A query value: the characters the parser of `Query` would read as something else.
fn enc(s: &str) -> String {
    s.replace('%', "%25")
        .replace(' ', "%20")
        .replace('+', "%2B")
        .replace('@', "%40")
        .replace('&', "%26")
}

const MS2_B: &str = "ITMS + c ESI Full ms2 300.20@cid35.00 [50.00-350.00]";

fn ask(r: &mut Routes<Vec<u8>>, path: &str, q: &str) -> String {
    r.handle(path, &Query::parse(q)).unwrap().unwrap()
}

#[test]
fn route_lists_the_channels_of_the_file() {
    let mut r = indice(
        10,
        &[
            (1, FULL, None),
            (2, MS2_A, Some(222.1)),
            (2, MS2_B, Some(300.2)),
        ],
    );
    let v = crate::json::parse(ask(&mut r, "/api/canali", "k=0").as_bytes()).unwrap();
    assert_eq!(v.get("v").and_then(|x| x.str()), Some("canali/1"));
    assert_eq!(
        v.get("tipo")
            .and_then(|t| t.get("proposto"))
            .and_then(|x| x.str()),
        Some("lc-ms2-mirato")
    );
    let ids: Vec<&str> = v
        .get("canali")
        .unwrap()
        .arr()
        .iter()
        .map(|c| c.get("id").unwrap().str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "tic",
            "bpc",
            "ms:ITMS + c ESI Full ms",
            "ms:ITMS + ms2 222 @cid",
            "ms:ITMS + ms2 300 @cid"
        ]
    );
    let ms2 = &v.get("canali").unwrap().arr()[3];
    assert_eq!(ms2.get("n").unwrap().f64(), Some(10.0));
    assert_eq!(ms2.get("gen").unwrap().f64(), Some(2.0));
    assert_eq!(ms2.get("spettri").unwrap().bool(), Some(true));
}

#[test]
fn channel_of_a_key_is_the_filter_of_that_key_byte_for_byte() {
    let mut r = indice(
        10,
        &[
            (1, FULL, None),
            (2, MS2_A, Some(222.1)),
            (2, MS2_B, Some(300.2)),
        ],
    );
    let key = enc("ITMS + c ESI Full ms");
    for kind in ["tic", "bpc"] {
        assert_eq!(
            ask(
                &mut r,
                "/api/chrom",
                &format!("k=0&kind={kind}&canale=ms:{key}")
            ),
            ask(&mut r, "/api/chrom", &format!("k=0&kind={kind}&filt={key}")),
        );
    }
    assert_eq!(
        ask(&mut r, "/api/scanlist", &format!("k=0&canale=ms:{key}")),
        ask(&mut r, "/api/scanlist", &format!("k=0&filt={key}"))
    );
    // an unknown channel says what is wrong
    let e = r
        .handle("/api/chrom", &Query::parse("k=0&canale=ms:nope"))
        .unwrap_err();
    assert_eq!(e.key, "err.canali.unknown");
}

#[test]
fn xic_inside_two_precursors_gives_different_traces() {
    let mut r = indice(
        10,
        &[
            (1, FULL, None),
            (2, MS2_A, Some(222.1)),
            (2, MS2_B, Some(300.2)),
        ],
    );
    let trace = |r: &mut Routes<Vec<u8>>, q: &str| {
        crate::json::parse(ask(r, "/api/xic", q).as_bytes()).unwrap()
    };
    let ys = |v: &crate::json::Value| -> Vec<f64> {
        v.get("y")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.f64().unwrap())
            .collect()
    };
    let a = trace(
        &mut r,
        &format!(
            "k=0&mz=150&tol=0.5&canale=ms:{}",
            enc("ITMS + ms2 222 @cid")
        ),
    );
    let b = trace(
        &mut r,
        &format!(
            "k=0&mz=150&tol=0.5&canale=ms:{}",
            enc("ITMS + ms2 300 @cid")
        ),
    );
    let (ya, yb) = (ys(&a), ys(&b));
    assert_eq!((ya.len(), yb.len()), (10, 10));
    assert_eq!(ya[0], 800.0);
    assert!(yb.iter().all(|y| *y == 10.0));
    // the trace of the level adds both precursors in 20 scans
    let level = r.xic(150.0, 0.5, 2).unwrap();
    let level = crate::json::parse(level.as_bytes()).unwrap();
    assert_eq!(level.get("y").unwrap().arr().len(), 20);
    assert_ne!(ya, ys(&level));
    // the TIC channel adds every level
    let tutti =
        crate::json::parse(ask(&mut r, "/api/xic", "k=0&mz=150&tol=0.5&canale=tic").as_bytes())
            .unwrap();
    assert_eq!(tutti.get("y").unwrap().arr().len(), 30);
}

#[test]
fn tic_channel_joins_the_levels_in_time_order() {
    let mut r = indice(5, &[(1, FULL, None), (2, MS2_A, Some(222.1))]);
    let v = crate::json::parse(ask(&mut r, "/api/chrom", "k=0&canale=tic").as_bytes()).unwrap();
    let rt: Vec<f64> = v
        .get("rt")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    assert_eq!(rt.len(), 10);
    assert!(rt.windows(2).all(|w| w[0] <= w[1]));
    // the old parameters stay as they were
    let old = crate::json::parse(ask(&mut r, "/api/chrom", "k=0&level=2").as_bytes()).unwrap();
    assert_eq!(old.get("rt").unwrap().arr().len(), 5);
}

#[test]
fn a_transition_is_not_a_scan_route() {
    let mut r = indice(3, &[(1, FULL, None)]);
    let chroms = vec![srm(0, 230.0, 162.0, 25.0, "")];
    r.pon_cromatogrammi("c.mzML", &chroms, &Facts::default());
    assert!(r.canali().trova("srm:230>162@25").is_some());
    assert_eq!(
        r.handle("/api/chrom", &Query::parse("k=0&canale=srm:230>162@25"))
            .unwrap(),
        None
    );
}
