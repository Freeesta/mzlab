//! A synthetic Agilent `.D` cut short at every length of `AcqData/MSScan.bin` and of `AcqData/MSPeak.bin`: the reader keeps the whole scans, exactly as the
//! whole folder reads them, and counts the others in `missing`, or it gives one keyed error; never a scan read short. The folder announces its 4 scans in
//! `MSTS.xml`; without it, a part of a record stays an error. No laboratory data.

#![allow(clippy::disallowed_methods)]

mod agilent_synth;

use agilent_synth::{encode, sample};
use mzlab_core::lettori::{self, FileSet, Limits, Member, Reader};

const MSTS: &[u8] =
    b"<?xml version=\"1.0\"?><TimeSegments><TimeSegment><StartTime>3</StartTime><EndTime>3.1</EndTime><NumOfScans>4</NumOfScans></TimeSegment></TimeSegments>";

fn apri(scan: &[u8], peak: &[u8], ts: bool) -> mzlab_core::error::Result<Box<dyn Reader>> {
    let mut members = vec![
        ("x.D/AcqData/MSScan.bin", scan.to_vec()),
        ("x.D/AcqData/MSPeak.bin", peak.to_vec()),
    ];
    if ts {
        members.push(("x.D/AcqData/MSTS.xml", MSTS.to_vec()));
    }
    lettori::apri(
        FileSet {
            membri: members
                .into_iter()
                .map(|(n, b)| Member {
                    nome: n.into(),
                    dim: b.len() as u64,
                    dati: Box::new(b),
                    sha1: None,
                })
                .collect(),
        },
        &Limits::sito(),
    )
}

/// (id, m/z, intensities) of every scan of the index.
fn scansioni(l: &mut Box<dyn Reader>) -> Vec<(String, Vec<f64>, Vec<f64>)> {
    let ix = l.index(0).expect("index").to_vec();
    (0..ix.len())
        .map(|j| {
            let s = l.scan(0, j).expect("a whole scan reads");
            let i = (0..s.mz.len()).map(|k| s.intensity_f64(k)).collect();
            (ix[j].id.clone(), s.mz.clone(), i)
        })
        .collect()
}

/// The keys a folder cut too short may give.
const KEYS: [&str; 3] = [
    "err.file.agilent",
    "err.file.agilentCut",
    "err.file.agilentNoPeaks",
];

/// Opens the cut folder and checks it against the whole one; returns whether it opened.
fn check(scan: &[u8], peak: &[u8], intere: &[(String, Vec<f64>, Vec<f64>)], at: &str) -> bool {
    match apri(scan, peak, true) {
        Ok(mut l) => {
            let s = scansioni(&mut l);
            assert!(
                s.len() < intere.len() || l.missing(0) == 0,
                "{at}: nothing counted"
            );
            assert_eq!(s[..], intere[..s.len()], "{at}: the scans kept differ");
            // three scans with points and one empty of four: what is not kept is missing
            assert!(
                l.missing(0) >= intere.len() - s.len(),
                "{at}: {} missing",
                l.missing(0)
            );
            assert!(l.missing(0) <= 4, "{at}: {} missing", l.missing(0));
            true
        }
        // a cut inside the 160-byte header leaves nothing that says Agilent: the folder is not recognised
        Err(e) if scan.len() < 160 && e.key == "err.file.notAccepted" => false,
        Err(e) => {
            assert!(KEYS.contains(&e.key), "{at}: {}", e.key);
            false
        }
    }
}

#[test]
fn a_cut_folder_keeps_its_whole_scans_and_counts_the_rest() {
    let (idx, peak) = encode(&sample());
    let mut l = apri(&idx, &peak, true).expect("the whole folder opens");
    assert_eq!(l.missing(0), 0);
    let intere = scansioni(&mut l);
    assert_eq!(intere.len(), 3);
    let mut aperte = 0;
    for n in 0..idx.len() {
        aperte += usize::from(check(
            &idx[..n],
            &peak,
            &intere,
            &format!("MSScan.bin cut at {n}"),
        ));
    }
    for n in 0..peak.len() {
        aperte += usize::from(check(
            &idx,
            &peak[..n],
            &intere,
            &format!("MSPeak.bin cut at {n}"),
        ));
    }
    assert!(aperte > idx.len() / 2, "only {aperte} cuts opened");
    // A record cut in half: three whole records, one announced scan left out.
    let mut l = apri(&idx[..idx.len() - 7], &peak, true).expect("opens");
    assert_eq!((scansioni(&mut l).len(), l.missing(0)), (2, 1));
    // The points of the last scan cut away.
    let mut l = apri(&idx, &peak[..peak.len() - 1], true).expect("opens");
    assert_eq!((scansioni(&mut l).len(), l.missing(0)), (2, 1));
    // Nothing whole is left: one keyed notice.
    let e = apri(&idx, &peak[..1], true).err().expect("refused");
    assert_eq!(e.key, "err.file.agilentCut");
    assert!(
        e.params.iter().any(|(k, v)| *k == "name" && v == "x.D"),
        "{:?}",
        e.params
    );
    // A whole index with an empty spectrum file names that file, not a cut.
    let e = apri(&idx, &[], true).err().expect("refused");
    assert_eq!(e.key, "err.file.agilentNoPeaks");
}

#[test]
fn without_an_announced_count_a_part_of_a_record_stays_an_error() {
    let (idx, peak) = encode(&sample());
    let e = apri(&idx[..idx.len() - 7], &peak, false)
        .err()
        .expect("refused");
    assert_eq!(e.key, "err.file.agilent");
}
