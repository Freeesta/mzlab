#![cfg(feature = "thermo")]
//! One damaged unit of a Thermo `.raw` never refuses the file (the reader rules R1-R4): a scan whose index entry points past the end, a scan whose record
//! does not decode, a controller whose run header cannot be read (before or after the MS one) are left out or left without points and counted; every other
//! scan, the TIC and the other analog channels load. Built files only (`common/thermo.rs`, invented numbers).
mod common;

use common::thermo::Raw;
use mzlab_core::readers::{FileSet, Limits, Losses, Member, Reader, RAW_DAMAGED, TRUNCATED};
use mzlab_core::readers_mzml::Conversion;

const SCANS: u32 = 12;

fn reader(b: Vec<u8>) -> Box<dyn Reader> {
    let set = FileSet {
        membri: vec![Member {
            nome: "x.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    match mzlab_lettori::apri(set, &Limits::sito()) {
        Ok(l) => l,
        Err(e) => panic!("refused: {} {:?}", e.key, e.params),
    }
}

/// Every scan decoded through both reads the engine uses; the point counts by row.
fn points(l: &mut dyn Reader) -> Vec<usize> {
    let n = l.index(0).unwrap().len();
    (0..n)
        .map(|k| {
            let a = l.scan(0, k).unwrap().mz.len();
            assert_eq!(l.scan_data(0, k).unwrap().mz.len(), a, "scan {k}");
            a
        })
        .collect()
}

fn method(l: &mut dyn Reader) -> String {
    l.method(0).unwrap()
}

#[test]
fn a_sound_file_has_no_damaged_scan() {
    let mut l = reader(Raw::tsq(63, SCANS).bytes());
    assert_eq!(points(&mut *l), vec![3; SCANS as usize]);
    assert_eq!(l.losses(0).unwrap(), Losses::complete(SCANS as usize));
}

#[test]
fn an_entry_past_the_end_loses_its_scan_only() {
    // the last entry past the end is a cut file as far as the bytes tell: counted as cut, the scans before it load
    let mut r = Raw::tsq(63, SCANS);
    r.entry_past_the_end = Some(SCANS - 1);
    let mut l = reader(r.bytes());
    assert_eq!(points(&mut *l), vec![3; SCANS as usize - 1]);
    assert_eq!(
        l.losses(0).unwrap(),
        Losses::of(SCANS as usize, SCANS as usize - 1, TRUNCATED)
    );
    for k in 0..SCANS - 1 {
        let mut r = Raw::tsq(63, SCANS);
        r.entry_past_the_end = Some(k);
        let mut l = reader(r.bytes());
        // known when the file opens: the count is whole before any scan is read (conv/1 of a conversion)
        let want = Losses::of(SCANS as usize, SCANS as usize - 1, RAW_DAMAGED);
        assert_eq!(l.losses(0).unwrap(), want, "scan {k}");
        let p = points(&mut *l);
        for (i, n) in p.iter().enumerate() {
            assert_eq!(
                *n,
                if i == k as usize { 0 } else { 3 },
                "scan {i} of damaged {k}"
            );
        }
        assert_eq!(l.losses(0).unwrap(), want, "scan {k}");
        assert!(method(&mut *l).contains("ScanData not read"), "scan {k}");
        // the damaged row keeps its time, level and number
        let row = &l.index(0).unwrap()[k as usize];
        assert!(row.position.is_none());
        assert!(row.id.ends_with(&format!("scan={}", k + 1)));
    }
}

#[test]
fn a_record_that_does_not_decode_loses_its_scan_only() {
    for k in 0..SCANS {
        let mut r = Raw::tsq(63, SCANS);
        r.entry_too_big = Some(k);
        let mut l = reader(r.bytes());
        let p = points(&mut *l);
        for (i, n) in p.iter().enumerate() {
            assert_eq!(
                *n,
                if i == k as usize { 0 } else { 3 },
                "scan {i} of damaged {k}"
            );
        }
        // counted once the scans are decoded (index, conversion)
        assert_eq!(
            l.losses(0).unwrap(),
            Losses::of(SCANS as usize, SCANS as usize - 1, RAW_DAMAGED),
            "scan {k}"
        );
    }
}

#[test]
fn a_conversion_writes_every_scan_and_counts_the_damaged_one() {
    let mut r = Raw::tsq(63, SCANS);
    r.entry_past_the_end = Some(4);
    let mut l = reader(r.bytes());
    let xml = Conversion::default()
        .write(&mut *l, 0, Vec::new(), &mut |_, _| {})
        .expect("one damaged scan never stops the conversion");
    let t = String::from_utf8_lossy(&xml);
    assert_eq!(t.matches("<spectrum ").count(), SCANS as usize);
    assert!(t.contains("warn.raw.damaged"), "conv/1 names the cause");
    assert!(t.contains("&quot;lette&quot;:11"), "conv/1 counts it");
}

#[test]
fn an_unreadable_controller_before_the_ms_one_does_not_refuse_the_file() {
    let mut r = Raw::tsq(63, SCANS);
    r.stray_first = true;
    let mut l = reader(r.bytes());
    assert_eq!(points(&mut *l), vec![3; SCANS as usize]);
    assert_eq!(l.losses(0).unwrap(), Losses::complete(SCANS as usize));
    let m = method(&mut *l);
    assert!(m.contains("RunHeader not read"), "{m}");
    // the same controller is not reported again as an analog trace
    assert!(!m.contains("AnalogTrace"), "{m}");
}

#[test]
fn an_unreadable_analog_controller_leaves_the_other_channels() {
    let mut r = Raw::tsq(63, SCANS);
    r.analog = vec![true, false, true];
    let mut l = reader(r.bytes());
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, ["TIC", "Analog 0", "Analog 2"]);
    let c = l.chromatogram(0, "Analog 2").unwrap();
    assert_eq!(c.intensity.len(), SCANS as usize);
    assert_eq!(l.chromatogram(0, "TIC").unwrap().time.len(), SCANS as usize);
    let m = method(&mut *l);
    assert!(m.contains("AnalogTrace not read"), "{m}");
    assert!(m.contains("1 of 4 controllers"), "{m}");
    assert_eq!(points(&mut *l), vec![3; SCANS as usize]);
}

#[test]
fn every_defect_at_once_still_loads_the_rest() {
    let mut r = Raw::tsq(63, SCANS);
    r.stray_first = true;
    r.analog = vec![true, false];
    r.entry_past_the_end = Some(4);
    r.entry_too_big = Some(7);
    let mut l = reader(r.bytes());
    let p = points(&mut *l);
    assert_eq!(p.iter().filter(|&&n| n == 3).count(), SCANS as usize - 2);
    assert_eq!((p[4], p[7]), (0, 0));
    assert_eq!(
        l.losses(0).unwrap(),
        Losses::of(SCANS as usize, SCANS as usize - 2, RAW_DAMAGED)
    );
    assert_eq!(l.chromatograms(0).unwrap().len(), 2);
}
