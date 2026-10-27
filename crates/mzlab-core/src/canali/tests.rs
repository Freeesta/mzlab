//! The channels of synthetic files: the lists below are written by hand from the structure of each file, not read back from the code.

use super::*;
use crate::idxread::Index;
use crate::mzidx::build_mzml_to_vec;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, MzmlWriter};
use crate::routes::{Query, Routes};
use crate::upload::prepare;

fn scan(level: u8, rt: f64, stages: &[(f64, f64)]) -> Scan {
    Scan {
        level,
        rt,
        precursor: stages.last().map(|s| s.0),
        path: (!stages.is_empty()).then(|| stages.iter().map(|(m, ce)| (*m, "CID".to_string(), *ce)).collect()),
        ..Default::default()
    }
}

/// (id, family, level, n, parent) of every channel.
fn riassunto(c: &Canali) -> Vec<(String, &'static str, u8, u32, Option<u32>)> {
    c.v.iter().map(|x| (x.id.clone(), x.famiglia.nome(), x.livello, x.n, x.genitore)).collect()
}

fn voce(id: &str, f: &'static str, l: u8, n: u32, g: Option<u32>) -> (String, &'static str, u8, u32, Option<u32>) {
    (id.to_string(), f, l, n, g)
}

fn da(scans: &[Scan]) -> Canali {
    Canali::da(scans, &[], &Facts::default(), "x.mzML")
}

#[test]
fn full_scan_file() {
    let scans: Vec<Scan> = (0..30).map(|i| scan(1, i as f64 * 0.1, &[])).collect();
    let c = da(&scans);
    assert_eq!(riassunto(&c), [voce("tic", "tic", 0, 30, None), voce("bpc", "bpc", 0, 30, Some(0)), voce("ms:ms1", "ms", 1, 30, Some(0))]);
    assert_eq!(c.tipo, Tipo { proposto: "lc-full", motivo: "ms1_only" });
    assert_eq!(c.v[2].etichetta, "MS¹");
    assert_eq!(c.v[2].omologo, "ms|1|||||");
    assert!(c.v[2].spettri && !c.v[0].spettri);
}

#[test]
fn targeted_ms2_in_lc_has_one_channel_per_precursor() {
    // MS1, then MS2 of 222.1 and of 300.2 at 35 eV, every cycle
    let mut scans = vec![];
    for i in 0..20 {
        let rt = i as f64 * 0.1;
        scans.push(scan(1, rt, &[]));
        scans.push(scan(2, rt + 0.01, &[(222.1, 35.0)]));
        scans.push(scan(2, rt + 0.02, &[(300.2, 35.0)]));
    }
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 60, None),
            voce("bpc", "bpc", 0, 60, Some(0)),
            voce("ms:ms1", "ms", 1, 20, Some(0)),
            voce("ms:ms2 222 @cid", "ms", 2, 20, Some(2)),
            voce("ms:ms2 300 @cid", "ms", 2, 20, Some(2)),
        ]
    );
    assert_eq!(c.tipo.proposto, "lc-ms2-mirato");
    assert_eq!(c.v[3].etichetta, "MS² 222 @CID35");
    assert_eq!(c.v[3].omologo, "ms|2|222|cid|35||");
    // the partition: scans of the channel in file order, 4 bytes per scan
    assert_eq!(c.scansioni(3), (0..20).map(|i| 3 * i + 1).collect::<Vec<u32>>());
    assert_eq!(c.scansioni(4), (0..20).map(|i| 3 * i + 2).collect::<Vec<u32>>());
    assert_eq!(c.scansioni(0), (0..60).collect::<Vec<u32>>());
    assert_eq!(c.scan_ids.len(), 60);
}

#[test]
fn msn_in_blocks_is_a_tree_of_channels() {
    // an infusion: MS1, then 20 scans of MS2 317, MS3 317>261 and MS3 317>299 held one after the other
    let paths: [&[(f64, f64)]; 3] = [&[(317.2, 25.0)], &[(317.2, 25.0), (261.1, 30.0)], &[(317.2, 25.0), (299.2, 30.0)]];
    let mut scans = vec![];
    let mut rt = 0.0;
    for p in paths {
        scans.push(scan(1, rt, &[]));
        for _ in 0..20 {
            rt += 0.01;
            scans.push(scan(p.len() as u8 + 1, rt, p));
        }
    }
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 63, None),
            voce("bpc", "bpc", 0, 63, Some(0)),
            voce("ms:ms1", "ms", 1, 3, Some(0)),
            voce("ms:ms2 317 @cid", "ms", 2, 20, Some(2)),
            voce("ms:ms3 317 > 261 @cid30", "ms", 3, 20, Some(3)),
            voce("ms:ms3 317 > 299 @cid30", "ms", 3, 20, Some(3)),
        ]
    );
    assert_eq!(c.tipo, Tipo { proposto: "infusione-msn", motivo: "blocks" });
    assert_eq!(c.v[4].etichetta, "MS³ 317→261 @CID30");
    assert_eq!(c.v[4].omologo, "ms|3|317>261|cid|30||");
}

#[test]
fn many_precursors_make_one_dda_channel() {
    // 70 distinct precursors at level 2, each MS2 hanging from the cycle's MS1; one of them has an MS3
    let mut scans = vec![];
    for i in 0..70 {
        let rt = i as f64 * 0.05;
        scans.push(scan(1, rt, &[]));
        scans.push(scan(2, rt + 0.01, &[(200.0 + i as f64 * 3.0, 30.0)]));
    }
    scans.push(scan(3, 4.0, &[(200.0, 30.0), (150.0, 30.0)]));
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 141, None),
            voce("bpc", "bpc", 0, 141, Some(0)),
            voce("ms:ms1", "ms", 1, 70, Some(0)),
            voce("dd:2", "dd", 2, 70, Some(0)),
            voce("ms:ms3 200 > 150 @cid30", "ms", 3, 1, Some(3)),
        ]
    );
    assert_eq!(c.v[3].sigla, Some("DDA"));
    assert_eq!(c.v[3].etichetta, "MS² DDA");
    assert_eq!(c.tipo, Tipo { proposto: "lc-dda", motivo: "many_precursors" });
    assert_eq!(c.scan_ids.len(), 141);
}

#[test]
fn two_polarities_with_the_same_key_are_two_channels() {
    let mut scans = vec![];
    for i in 0..10 {
        for pol in [1i8, -1] {
            scans.push(Scan { polarity: pol, ..scan(1, i as f64 * 0.1, &[]) });
        }
    }
    let c = da(&scans);
    let ids: Vec<&str> = c.v.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["tic", "bpc", "ms:ms1#+", "ms:ms1#-"]);
    assert_eq!((c.v[2].pol, c.v[3].pol), (1, -1));
}

fn srm(index: usize, q1: f64, q3: f64, ce: f64, name: &str) -> Chrom {
    Chrom { index, id: format!("SRM SIC Q1={q1} Q3={q3}"), kind: "srm".into(), q1: Some(q1), q3: Some(q3), ce: Some(ce), name: name.into(), n: Some(400), ..Default::default() }
}

#[test]
fn mrm_file_has_a_summed_tic_and_one_channel_per_transition() {
    let chroms = vec![srm(0, 230.0, 162.0, 25.0, "bentazone 1"), srm(1, 230.0, 190.0, 18.0, "")];
    let facts = Facts { srm_rt: Some((0.0, 20.0)), ..Default::default() };
    let c = Canali::da(&[], &chroms, &facts, "m.mzML");
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 400, None),
            voce("srm:230>162@25", "srm", 2, 400, Some(0)),
            voce("srm:230>190@18", "srm", 2, 400, Some(0)),
        ]
    );
    assert_eq!(c.v[0].sorgente, Sorgente::Somma);
    assert_eq!(c.v[1].sorgente, Sorgente::Cromatogramma(0));
    assert_eq!(c.v[1].etichetta, "MRM 230→162 @25 «bentazone 1»");
    assert_eq!(c.v[1].omologo, "srm|2|230.0>162.0||25||");
    assert!(!c.v[1].spettri);
    assert_eq!(c.tipo, Tipo { proposto: "lc-mrm", motivo: "srm_only" });
    // the TIC of the maker is the TIC of the file
    let mut con_tic = chroms.clone();
    con_tic.push(Chrom { index: 2, id: "TIC".into(), kind: "tic".into(), ..Default::default() });
    let c = Canali::da(&[], &con_tic, &facts, "m.mzML");
    assert_eq!(c.v[0].sorgente, Sorgente::Cromatogramma(2));
}

#[test]
fn mrm_with_product_ion_scans_is_mixed() {
    let scans: Vec<Scan> = (0..10).map(|i| scan(2, i as f64 * 0.1, &[(364.1 + (i % 2) as f64, 30.0)])).collect();
    let c = Canali::da(&scans, &[srm(1, 364.1, 194.1, 20.0, "Quant")], &Facts::default(), "c.mzML");
    assert_eq!(c.tipo.proposto, "misto");
    let ids: Vec<&str> = c.v.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["tic", "bpc", "ms:ms2 364 @cid", "ms:ms2 365 @cid", "srm:364.1>194.1@20"]);
}

#[test]
fn a_file_with_only_a_uv_trace_has_no_channel_yet() {
    let c = Canali::da(&[], &[], &Facts { pda: true, ..Default::default() }, "u.mzML");
    assert!(c.v.is_empty());
    assert_eq!(c.tipo.proposto, "solo-uv");
}

#[test]
fn memory_beyond_the_scan_table_is_at_most_eight_bytes_per_scan() {
    let n = 100_000usize;
    let scans: Vec<Scan> = (0..n)
        .map(|i| match i % 5 {
            0 => scan(1, i as f64 * 1e-3, &[]),
            k => scan(2, i as f64 * 1e-3, &[(200.0 + k as f64 * 10.0, 30.0)]),
        })
        .collect();
    let c = da(&scans);
    assert_eq!(c.v.len(), 2 + 1 + 4);
    assert!(c.byte() <= 8 * n, "{} bytes for {n} scans", c.byte());
}

/// An mzML (Thermo-style filter strings) of the cycles `(level, filter, precursor)`, one cycle every 0.1 min.
fn indice(cicli: usize, scansioni: &[(u8, &str, Option<f64>)]) -> Routes<Vec<u8>> {
    let mut w = MzmlWriter::new(Vec::new(), &Header { run: "canali".into(), ..Default::default() }).unwrap();
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
    s.replace('%', "%25").replace(' ', "%20").replace('+', "%2B").replace('@', "%40").replace('&', "%26")
}

const MS2_B: &str = "ITMS + c ESI Full ms2 300.20@cid35.00 [50.00-350.00]";

fn ask(r: &mut Routes<Vec<u8>>, path: &str, q: &str) -> String {
    r.handle(path, &Query::parse(q)).unwrap().unwrap()
}

#[test]
fn route_lists_the_channels_of_the_file() {
    let mut r = indice(10, &[(1, FULL, None), (2, MS2_A, Some(222.1)), (2, MS2_B, Some(300.2))]);
    let v = crate::json::parse(ask(&mut r, "/api/canali", "k=0").as_bytes()).unwrap();
    assert_eq!(v.get("v").and_then(|x| x.str()), Some("canali/1"));
    assert_eq!(v.get("tipo").and_then(|t| t.get("proposto")).and_then(|x| x.str()), Some("lc-ms2-mirato"));
    let ids: Vec<&str> = v.get("canali").unwrap().arr().iter().map(|c| c.get("id").unwrap().str().unwrap()).collect();
    assert_eq!(ids, ["tic", "bpc", "ms:ITMS + c ESI Full ms", "ms:ITMS + ms2 222 @cid", "ms:ITMS + ms2 300 @cid"]);
    let ms2 = &v.get("canali").unwrap().arr()[3];
    assert_eq!(ms2.get("n").unwrap().f64(), Some(10.0));
    assert_eq!(ms2.get("gen").unwrap().f64(), Some(2.0));
    assert_eq!(ms2.get("spettri").unwrap().bool(), Some(true));
}

#[test]
fn channel_of_a_key_is_the_filter_of_that_key_byte_for_byte() {
    let mut r = indice(10, &[(1, FULL, None), (2, MS2_A, Some(222.1)), (2, MS2_B, Some(300.2))]);
    let key = enc("ITMS + c ESI Full ms");
    for kind in ["tic", "bpc"] {
        assert_eq!(
            ask(&mut r, "/api/chrom", &format!("k=0&kind={kind}&canale=ms:{key}")),
            ask(&mut r, "/api/chrom", &format!("k=0&kind={kind}&filt={key}")),
        );
    }
    assert_eq!(ask(&mut r, "/api/scanlist", &format!("k=0&canale=ms:{key}")), ask(&mut r, "/api/scanlist", &format!("k=0&filt={key}")));
    // an unknown channel says what is wrong
    let e = r.handle("/api/chrom", &Query::parse("k=0&canale=ms:nope")).unwrap_err();
    assert_eq!(e.key, "err.canali.unknown");
}

#[test]
fn xic_inside_two_precursors_gives_different_traces() {
    let mut r = indice(10, &[(1, FULL, None), (2, MS2_A, Some(222.1)), (2, MS2_B, Some(300.2))]);
    let trace = |r: &mut Routes<Vec<u8>>, q: &str| crate::json::parse(ask(r, "/api/xic", q).as_bytes()).unwrap();
    let ys = |v: &crate::json::Value| -> Vec<f64> { v.get("y").unwrap().arr().iter().map(|x| x.f64().unwrap()).collect() };
    let a = trace(&mut r, &format!("k=0&mz=150&tol=0.5&canale=ms:{}", enc("ITMS + ms2 222 @cid")));
    let b = trace(&mut r, &format!("k=0&mz=150&tol=0.5&canale=ms:{}", enc("ITMS + ms2 300 @cid")));
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
    let tutti = crate::json::parse(ask(&mut r, "/api/xic", "k=0&mz=150&tol=0.5&canale=tic").as_bytes()).unwrap();
    assert_eq!(tutti.get("y").unwrap().arr().len(), 30);
}

#[test]
fn tic_channel_joins_the_levels_in_time_order() {
    let mut r = indice(5, &[(1, FULL, None), (2, MS2_A, Some(222.1))]);
    let v = crate::json::parse(ask(&mut r, "/api/chrom", "k=0&canale=tic").as_bytes()).unwrap();
    let rt: Vec<f64> = v.get("rt").unwrap().arr().iter().map(|x| x.f64().unwrap()).collect();
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
    assert_eq!(r.handle("/api/chrom", &Query::parse("k=0&canale=srm:230>162@25")).unwrap(), None);
}
