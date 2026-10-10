//! Build / read round trips of the `.mzlib` format, and its robustness against damaged files.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts, Builder};
use mzlab_lib::clean::{clean, CleanOpts};
use mzlab_lib::format::{pol_key, ENTRIES_PER_PAGE};
use mzlab_lib::io::ReadSeekSource;
use mzlab_lib::read::{cache_budget, Library, CACHE_BYTES, CACHE_BYTES_SMALL};
use mzlab_lib::spec::{licence, Spectrum};
use mzlab_lib::util::crc32c;
use std::io::Cursor;

fn expected_order(set: &[Spectrum]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..set.len()).collect();
    idx.sort_by(|&a, &b| {
        pol_key(set[a].pol)
            .cmp(&pol_key(set[b].pol))
            .then(set[a].prec.total_cmp(&set[b].prec))
            .then(a.cmp(&b))
    });
    idx
}

#[test]
fn round_trip_small() {
    let set = synth_set(1, 40, 25);
    let (bytes, info) = build_in_memory(&set, BuildOpts::default()).unwrap();
    assert_eq!(info.n_spectra, 40);
    assert_eq!(info.bytes as usize, bytes.len());
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    lib.verify().unwrap();
    assert_eq!(lib.n_spectra(), 40);
    for (i, &o) in expected_order(&set).iter().enumerate() {
        let s = &set[o];
        let r = lib.rec(i as u32).unwrap();
        assert_eq!((r.prec, r.pol, r.orig), (s.prec, s.pol, o as u32));
        assert_eq!(r.licence, licence::class_of(&s.meta.license));
        let m = lib.meta(i as u32).unwrap();
        assert_eq!(m, s.meta);
        let (cmz, cit) = clean(s.prec, &s.mz, &s.it, &CleanOpts::default());
        let (mz, it) = lib.peaks(i as u32).unwrap();
        assert_eq!(mz.len(), cmz.len());
        let mx = cit.iter().cloned().fold(0.0, f64::max);
        for j in 0..mz.len() {
            assert!((mz[j] - cmz[j]).abs() <= 5.1e-6, "m/z quantum");
            assert!(
                (it[j] - cit[j] / mx).abs() <= 1e-5,
                "intensity {} vs {}",
                it[j],
                cit[j] / mx
            );
        }
    }
}

#[test]
fn many_spectra_chunks_pages_and_ranges() {
    // more than two peak chunks (4096 spectra) and several index pages
    let set = synth_set(7, 9000, 12);
    let (bytes, info) = build_in_memory(&set, BuildOpts::default()).unwrap();
    assert!(info.n_peaks > 3 * ENTRIES_PER_PAGE as u64);
    let lib = Library::open(&bytes[..], 4 << 20).unwrap();
    lib.verify().unwrap();
    let order = expected_order(&set);
    for k in [0usize, 1, 4095, 4096, 4097, 8191, 8192, 8999] {
        let s = &set[order[k]];
        assert_eq!(lib.meta(k as u32).unwrap().accession, s.meta.accession);
        assert_eq!(lib.rec(k as u32).unwrap().prec, s.prec);
    }
    // polarity ranges and precursor windows are contiguous index ranges
    for (pol, lo, hi) in [
        (1i8, 400.0, 400.5),
        (-1, 150.0, 900.0),
        (0, 500.0, 500.01),
        (1, 0.0, 10.0),
    ] {
        let want: Vec<usize> = (0..order.len())
            .filter(|&i| {
                let s = &set[order[i]];
                let pol_ok = match pol {
                    1 => s.pol >= 0,
                    -1 => s.pol <= 0,
                    _ => true,
                };
                pol_ok && s.prec >= lo && s.prec <= hi
            })
            .collect();
        let got: Vec<usize> = lib
            .ranges(pol, lo, hi)
            .unwrap()
            .into_iter()
            .flatten()
            .map(|x| x as usize)
            .collect();
        let mut want = want;
        want.sort_by_key(|&i| (pol_key(set[order[i]].pol), i));
        let mut got = got;
        got.sort_by_key(|&i| (pol_key(set[order[i]].pol), i));
        assert_eq!(got, want, "pol {pol} [{lo}, {hi}]");
    }
}

#[test]
fn index_entries_match_the_peaks() {
    let set = synth_set(3, 300, 15);
    let (bytes, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    let mut seen = std::collections::HashMap::new();
    lib.for_each_entry(false, 0, u32::MAX, |mz, sp, q| {
        seen.insert((sp, mz), q);
    })
    .unwrap();
    let mut last = 0;
    lib.for_each_entry(false, 0, u32::MAX, |mz, _, _| {
        assert!(mz >= last);
        last = mz;
    })
    .unwrap();
    let mut nl_seen = std::collections::HashSet::new();
    lib.for_each_entry(true, 0, u32::MAX, |mz, sp, _| {
        nl_seen.insert((sp, mz));
    })
    .unwrap();
    for i in 0..lib.n_spectra() {
        let r = lib.rec(i).unwrap();
        let (mq, _) = lib.peaks_q(i).unwrap();
        for m in mq {
            assert!(seen.contains_key(&(i, m)), "fragment entry of spectrum {i}");
            let nl = ((r.prec - m as f64 / 1e5) * 1e5).round() as u32;
            assert!(
                nl_seen.contains(&(i, nl)),
                "neutral loss entry of spectrum {i}"
            );
        }
    }
    // the windowed read returns exactly the entries inside
    let mut n_all = 0;
    lib.for_each_entry(false, 10_000_000, 12_000_000, |mz, _, _| {
        assert!((10_000_000..=12_000_000).contains(&mz));
        n_all += 1;
    })
    .unwrap();
    assert_eq!(
        n_all,
        seen.keys()
            .filter(|(_, m)| (10_000_000..=12_000_000).contains(m))
            .count()
    );
}

#[test]
fn cache_budget_constants() {
    assert_eq!((CACHE_BYTES, CACHE_BYTES_SMALL), (64 << 20, 32 << 20));
    assert_eq!(cache_budget(false), 64 << 20);
    assert_eq!(cache_budget(true), 32 << 20);
}

#[test]
fn cache_stays_inside_the_budget_and_serves_repeats() {
    let set = synth_set(5, 9000, 12);
    let (bytes, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&bytes[..], 200_000).unwrap();
    for i in (0..9000).step_by(97) {
        lib.peaks_q(i).unwrap();
        assert!(
            lib.cached_bytes() <= 200_000 + 160_000,
            "cached {}",
            lib.cached_bytes()
        );
    }
    let before = lib.block_reads.get();
    lib.peaks_q(8999).unwrap();
    lib.peaks_q(8999).unwrap();
    assert!(lib.block_reads.get() <= before + 2);
    let b2 = lib.block_reads.get();
    lib.peaks_q(8999).unwrap();
    assert_eq!(lib.block_reads.get(), b2, "a repeat is served by the cache");
}

#[test]
fn works_over_read_seek_and_through_the_scratch_file_path() {
    let set = synth_set(2, 100, 10);
    let (bytes, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let src = ReadSeekSource::new(Cursor::new(bytes.clone())).unwrap();
    let a = Library::open(src, CACHE_BYTES).unwrap();
    let b = Library::open(&bytes[..], CACHE_BYTES).unwrap();
    for i in 0..100 {
        assert_eq!(a.peaks_q(i).unwrap(), b.peaks_q(i).unwrap());
    }
    // a builder fed spectrum by spectrum gives the same file as the convenience function
    let mut bld = Builder::new(Vec::<u8>::new(), BuildOpts::default());
    for s in &set {
        bld.add(s).unwrap();
    }
    let mut out = Vec::new();
    bld.finish(&mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn spectra_without_peaks_after_cleaning_are_counted_not_stored() {
    let mut set = synth_set(4, 10, 8);
    set.push(Spectrum {
        prec: 100.0,
        pol: 1,
        mz: vec![99.0, 99.5],
        it: vec![1.0, 1.0],
        meta: Default::default(),
    });
    let (_, info) = build_in_memory(&set, BuildOpts::default()).unwrap();
    assert_eq!((info.n_spectra, info.emptied), (10, 1));
    let e = build_in_memory(&set[10..], BuildOpts::default()).unwrap_err();
    assert_eq!(e.key, "lib.err.noSpectrum");
}

// ------------------------------------------------------------ robustness

fn small_file() -> Vec<u8> {
    build_in_memory(&synth_set(11, 60, 14), BuildOpts::default())
        .unwrap()
        .0
}

fn read_all(bytes: &[u8]) -> Result<Vec<(Vec<u32>, Vec<u16>, String)>, String> {
    let lib = Library::open(bytes, CACHE_BYTES).map_err(|e| e.key.to_string())?;
    lib.verify().map_err(|e| e.key.to_string())?;
    let mut v = Vec::new();
    for i in 0..lib.n_spectra() {
        let (m, it) = lib.peaks_q(i).map_err(|e| e.key.to_string())?;
        v.push((m, it, lib.meta(i).map_err(|e| e.key.to_string())?.name));
    }
    Ok(v)
}

#[test]
fn truncated_files_are_errors_never_panics() {
    let f = small_file();
    let good = read_all(&f).unwrap();
    for cut in (0..f.len())
        .step_by(37)
        .chain([f.len() - 1, f.len() - 2, 35, 36, 255, 256])
    {
        let r = read_all(&f[..cut]);
        assert!(r.is_err(), "cut at {cut} of {} was accepted", f.len());
    }
    assert_eq!(read_all(&f).unwrap(), good);
}

#[test]
fn changed_bits_are_found_or_harmless() {
    let f = small_file();
    let good = read_all(&f).unwrap();
    let mut r = Rng(99);
    let mut found = 0;
    for _ in 0..1500 {
        let mut g = f.clone();
        let at = r.next() as usize % g.len();
        g[at] ^= 1 << (r.next() % 8);
        match read_all(&g) {
            Err(_) => found += 1,
            Ok(v) => assert_eq!(
                v, good,
                "a flipped bit at {at} changed the content without an error"
            ),
        }
    }
    assert!(found > 1000, "{found} of 1500 flips found");
}

fn patch_header(f: &[u8], at: usize, bytes: &[u8]) -> Vec<u8> {
    let mut g = f.to_vec();
    g[at..at + bytes.len()].copy_from_slice(bytes);
    let c = crc32c(&g[..32]);
    g[32..36].copy_from_slice(&c.to_le_bytes());
    // the section table keeps its own checksum
    let n = u32::from_le_bytes([g[12], g[13], g[14], g[15]]) as usize;
    let end = 36 + n.min(8) * 32;
    let c2 = crc32c(&g[..end]);
    g[end..end + 4].copy_from_slice(&c2.to_le_bytes());
    g
}

#[test]
fn enormous_declared_counts_are_refused_without_allocating() {
    let f = small_file();
    for (at, v) in [
        (8usize, u32::MAX.to_le_bytes()),
        (8, 0x7FFF_FFFFu32.to_le_bytes()),
        (12, 1_000_000u32.to_le_bytes()),
        (8, 0u32.to_le_bytes()),
    ] {
        let g = patch_header(&f, at, &v);
        assert!(read_all(&g).is_err(), "count at {at} = {v:?}");
    }
    let g = patch_header(&f, 16, &u64::MAX.to_le_bytes());
    assert!(read_all(&g).is_err());
    // a version from the future
    let g = patch_header(&f, 6, &9u16.to_le_bytes());
    assert_eq!(
        Library::open(&g[..], 1 << 20).err().unwrap().key,
        "lib.err.version"
    );
    // not a library at all
    assert_eq!(
        Library::open(&b"hello world, this is not a library at all"[..], 1 << 20)
            .err()
            .unwrap()
            .key,
        "lib.err.archive"
    );
}

#[test]
fn a_section_pointing_outside_the_file_is_refused() {
    let f = small_file();
    // first section entry: offset at 36 + 8, length at 36 + 16
    let g = patch_header(&f, 36 + 16, &(u64::MAX / 2).to_le_bytes());
    assert!(read_all(&g).is_err());
    let g = patch_header(&f, 36 + 8, &(u64::MAX - 3).to_le_bytes());
    assert!(read_all(&g).is_err());
}

#[test]
fn very_long_names_are_cut_not_fatal() {
    let mut s = synth_set(1, 1, 10).remove(0);
    s.meta.name = "x".repeat(10_000);
    let (b, _) = build_in_memory(&[s], BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    lib.verify().unwrap();
    assert_eq!(
        lib.meta(0).unwrap().name.len(),
        10_000,
        "the builder keeps what the parser gave; the parser is what cuts"
    );
}
