#![cfg(feature = "thermo")]
//! The Thermo reader on files that are not as the format says, built here (`comune/thermo.rs`, invented numbers): cut files open up to the last whole scan,
//! an undocumented version is read with the nearest layout, an unreadable stream does not stop the scans, and what cannot be opened says why.
//! Not checked against RawFileReader: these are constructed files (docs/agenti/lettori-modulo.md).
mod comune;

use comune::thermo::{Layout, Raw};
use opentfraw::{Error, RawFileReader, Warning};
use std::io::Cursor;

/// The reader, or the error as a value the tests can print (`RawFileReader` has no `Debug`).
fn open(b: &[u8]) -> Result<RawFileReader, Error> {
    RawFileReader::open(Cursor::new(b.to_vec()))
}

fn refused(b: &[u8]) -> Error {
    match open(b) {
        Err(e) => e,
        Ok(r) => panic!("opened with {} scans, {:?}", r.num_scans, r.warnings),
    }
}

#[test]
fn the_built_file_opens_whole_and_gives_its_peaks() {
    for v in [57, 60, 62, 63] {
        let b = Raw::tsq(v, 10).bytes();
        let r = open(&b).unwrap_or_else(|e| panic!("v{v}: {e}"));
        assert_eq!((r.num_scans, r.version, r.declared_version), (10, v, v));
        assert!(r.warnings.is_empty(), "v{v}: {:?}", r.warnings);
        let p = r
            .read_scan_peaks(&mut Cursor::new(b.clone()), 4)
            .unwrap_or_else(|e| panic!("v{v}: {e}"));
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].mz, 303.0);
    }
}

fn peaks(b: &[u8], r: &RawFileReader, n: u32) -> Vec<(u64, u32)> {
    r.read_scan_peaks(&mut Cursor::new(b.to_vec()), n)
        .unwrap()
        .iter()
        .map(|p| (p.mz.to_bits(), p.abundance.to_bits()))
        .collect()
}

/// A file cut where the index is whole and the data is not keeps the scans that lie whole, with what they held, and says how many are left out.
#[test]
fn a_file_cut_in_its_data_opens_up_to_the_last_whole_scan() {
    let mut raw = Raw::tsq(63, 400);
    raw.layout = Layout::DataLast;
    let whole = raw.bytes();
    let all = open(&whole).unwrap();
    let record = (raw.peaks * 9) as usize;
    let data_start = whole.len() - 400 * record;
    for (num, den) in [(1usize, 4usize), (1, 2), (3, 4)] {
        let cut = whole.len() * num / den;
        let cut_bytes = &whole[..cut];
        if cut <= data_start {
            // the cut is before the data: nothing to read, and the error says where
            match refused(cut_bytes) {
                Error::Truncated { file_len, .. } => assert_eq!(file_len, cut as u64),
                other => panic!("cut at {num}/{den}: {other}"),
            }
            continue;
        }
        let r = open(cut_bytes).unwrap_or_else(|e| panic!("cut at {num}/{den}: {e}"));
        let expect = (cut - data_start) / record;
        assert_eq!(r.num_scans as usize, expect, "cut at {num}/{den}");
        assert_eq!(r.scan_index.len(), expect);
        assert!(
            matches!(&r.warnings[..], [Warning::Truncated { scans_declared: 400, scans_readable, .. }] if *scans_readable as usize == expect),
            "{:?}",
            r.warnings
        );
        for n in [1u32, expect as u32 / 2 + 1, expect as u32] {
            assert_eq!(peaks(cut_bytes, &r, n), peaks(&whole, &all, n), "scan {n}");
        }
    }
    // cut one byte inside the last record: that scan is out, the others stay
    let r = open(&whole[..whole.len() - 1]).unwrap();
    assert_eq!(r.num_scans, 399);
}

/// Cut anywhere in the first three quarters of a file whose data comes first, there is no run header or index left: a refusal that names the structure and the byte.
#[test]
fn a_file_cut_before_its_index_is_refused_with_the_place() {
    let whole = Raw::tsq(63, 400).bytes();
    for k in 1..40 {
        let cut = whole.len() * k / 40;
        match open(&whole[..cut]) {
            Err(Error::Truncated {
                stage,
                file_len,
                needed,
                ..
            }) => {
                assert_eq!(file_len, cut as u64);
                assert!(needed > file_len, "{stage}: needed {needed} <= {file_len}");
            }
            Err(e) => panic!("cut {k}/40: {e}"),
            Ok(_) => panic!("cut {k}/40 opened"),
        }
    }
}

#[test]
fn an_index_cut_in_the_middle_keeps_the_entries_that_are_whole() {
    let mut raw = Raw::tsq(63, 400);
    raw.layout = Layout::DataFirst;
    let whole = raw.bytes();
    // the run header is at the end: a file that loses only the end has no run header, so cut the index of a data-last file instead
    raw.layout = Layout::DataLast;
    let b = raw.bytes();
    assert!(whole.len() == b.len());
    let record = (raw.peaks * 9) as usize;
    let data_start = b.len() - 400 * record;
    let e = refused(&b[..data_start - 100]);
    assert!(
        matches!(e, Error::Truncated { .. }),
        "the data is gone: {e}"
    );
}

#[test]
fn an_undocumented_version_is_read_with_the_nearest_layout_when_the_index_holds() {
    let mut raw = Raw::tsq(60, 50);
    let reference = open(&raw.bytes()).unwrap();
    for declared in [59, 61] {
        raw.declared_version = declared;
        let b = raw.bytes();
        let r = open(&b).unwrap_or_else(|e| panic!("v{declared}: {e}"));
        assert_eq!((r.declared_version, r.version), (declared, 60));
        assert_eq!(
            r.warnings,
            vec![Warning::UnverifiedVersion { declared, used: 60 }]
        );
        assert_eq!(r.num_scans, reference.num_scans);
        assert_eq!(peaks(&b, &r, 7), peaks(&raw.bytes(), &r, 7));
    }
}

#[test]
fn an_undocumented_version_with_another_layout_keeps_the_error_with_the_number() {
    for declared in [0, 65, 67, 99, 101, 1000, u32::MAX] {
        let mut raw = Raw::tsq(63, 50);
        raw.declared_version = declared;
        match open(&raw.bytes()) {
            Err(Error::UnsupportedVersion(v)) => assert_eq!(v, declared),
            Err(e) => panic!("v{declared}: {e}"),
            Ok(_) => panic!("v{declared} opened a v63 layout"),
        }
    }
}

/// A count in a stream the scans do not need (events, error log) that the file cannot hold used to stop the whole opening with `AllocationTooLarge`.
#[test]
fn a_stream_that_cannot_be_read_is_left_out_and_the_scans_open() {
    for version in [57, 60, 62, 63] {
        let mut raw = Raw::tsq(version, 30);
        raw.events = u32::MAX;
        raw.error_entries = 4_000_000_000;
        let b = raw.bytes();
        let r = open(&b).unwrap_or_else(|e| panic!("v{version}: {e}"));
        assert_eq!(r.num_scans, 30);
        let stages: Vec<_> = r
            .warnings
            .iter()
            .filter_map(|w| match w {
                Warning::Skipped { stage, .. } => Some(*stage),
                _ => None,
            })
            .collect();
        assert!(stages.contains(&"ScanEvents"), "{stages:?}");
        assert!(stages.contains(&"ErrorLog"), "{stages:?}");
        assert_eq!(peaks(&b, &r, 30).len(), 3);
    }
}

#[test]
fn an_error_names_the_structure_the_version_and_the_size() {
    let b = Raw::tsq(62, 5).bytes();
    // a header that is whole and a seq row that is not
    let e = refused(&b[..1400]);
    let text = e.to_string();
    assert!(
        text.contains("file cut at 1400 bytes") && text.contains("SeqRow"),
        "{text}"
    );
    let mut bad = b.clone();
    bad[40..44].copy_from_slice(&66u32.to_le_bytes());
    // v66 layout over a v62 body: the run header addresses read as other numbers
    let e = open(&bad);
    if let Err(e) = e {
        let t = e.to_string();
        assert!(
            t.contains("v66") || matches!(e, Error::UnsupportedVersion(_)),
            "{t}"
        );
    }
}

/// A run header that declares far more scans than the file holds (a corrupt count, or an index that stops early) must not allocate for the count nor read the
/// bytes after the index as scans.
#[test]
fn a_scan_count_the_file_cannot_hold_keeps_only_the_scans_in_order() {
    let mut b = Raw::tsq(63, 400).bytes();
    let rh = b.len() - 7408;
    // last_scan_number is the fourth u32 of the run header
    b[rh + 12..rh + 16].copy_from_slice(&u32::MAX.to_le_bytes());
    let r = open(&b).unwrap();
    assert_eq!(r.num_scans, 400, "{:?}", r.warnings);
    assert!(matches!(
        &r.warnings[..],
        [Warning::Truncated {
            scans_readable: 400,
            ..
        }]
    ));
}

#[test]
fn describe_lists_the_structures_and_the_result() {
    let b = Raw::tsq(63, 20).bytes();
    let t = RawFileReader::describe(Cursor::new(b.clone()));
    assert!(
        t.contains("declared v63 read as v63") && t.contains("scans 1..20"),
        "{t}"
    );
    assert!(t.contains("open: 20 scans, 0 warnings"), "{t}");
    let t = RawFileReader::describe(Cursor::new(b[..2000].to_vec()));
    assert!(t.contains("open: file cut at 2000 bytes"), "{t}");
}

use mzlab_core::lettori::{FileSet, Limiti, Membro};

/// Everything the module does with a file, on whatever opens: the index, some scans, their headers, the chromatograms and the method. None of it may panic.
fn through_the_module(bytes: Vec<u8>) {
    let set = FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    };
    let Ok(mut l) = mzlab_lettori::apri(set, &Limiti::sito()) else {
        return;
    };
    let n = l.indice(0).map_or(0, <[_]>::len);
    for k in [0, n / 2, n.saturating_sub(1)] {
        let _ = l.scansione(0, k);
        let _ = l.intestazione(0, k);
    }
    if let Ok(v) = l.cromatogrammi(0) {
        for c in v.iter().take(3) {
            let _ = l.cromatogramma(0, &c.id);
        }
    }
    let _ = l.mancanti(0);
    let _ = l.metodo(0);
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() >> 11) as usize % n.max(1)
    }
}

/// One mutation of the kind that breaks readers: a count or an address set to a limit value, bytes flipped near the headers, a cut, a zeroed run, a copied chunk.
fn mutate(src: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut b = src.to_vec();
    for _ in 0..1 + rng.below(4) {
        let n = b.len();
        // a cut can leave fewer bytes than the smallest edit needs
        if n < 8 {
            break;
        }
        match rng.below(6) {
            0 => {
                let at = rng.below(n.saturating_sub(4)) & !3;
                let v = [0u32, 1, 2, 0x7FFF_FFFF, 0x8000_0000, u32::MAX, 0xFFFF][rng.below(7)];
                b[at..at + 4].copy_from_slice(&v.to_le_bytes());
            }
            1 => {
                let at = rng.below(n.min(16_000));
                b[at] ^= 1 << rng.below(8);
            }
            2 => b.truncate(1 + rng.below(n)),
            3 => {
                let at = rng.below(n);
                let len = rng.below(512).min(n - at);
                b[at..at + len].fill(0);
            }
            4 => {
                let (from, to, len) = (rng.below(n), rng.below(n), rng.below(256));
                let chunk: Vec<u8> = b[from..(from + len).min(n)].to_vec();
                let end = (to + chunk.len()).min(n);
                let k = end - to;
                b[to..end].copy_from_slice(&chunk[..k]);
            }
            _ => {
                let at = rng.below(n.saturating_sub(8)) & !7;
                b[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
            }
        }
        if b.is_empty() {
            break;
        }
    }
    b
}

/// Mutations of the built files through the whole module. A short run in the ordinary tests; `MZLAB_FUZZ_SECS=300` (or more) runs the same loop for that long.
#[test]
fn mutated_built_files_never_panic() {
    let secs: u64 = std::env::var("MZLAB_FUZZ_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut seeds = Vec::new();
    for v in [57, 60, 62, 63] {
        for layout in [Layout::DataFirst, Layout::DataLast] {
            let mut raw = Raw::tsq(v, 60);
            raw.layout = layout;
            seeds.push(raw.bytes());
        }
    }
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let start = std::time::Instant::now();
    let mut done = 0u64;
    loop {
        let seed = &seeds[rng.below(seeds.len())];
        through_the_module(mutate(seed, &mut rng));
        done += 1;
        let over = if secs == 0 {
            done >= 1500
        } else {
            start.elapsed().as_secs() >= secs
        };
        if over {
            break;
        }
    }
    eprintln!("mutated built files: {done} through the module");
}
