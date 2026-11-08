//! The family `andi` against hostile netCDF files: a count, a dimension, an offset or a name that lies must be an error with a key, never a panic or a huge allocation; and (with the private data
//! repository, `MZLAB_DATI`) against the real files exported by ChemStation and MassHunter. The real-data tests are skipped without it.
#![cfg(feature = "andi")]

mod comune;

use comune::andi::*;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn buono() -> Vec<u8> {
    andi_ms(&FLAT_RUN)
}

fn pos(b: &[u8], ago: &[u8]) -> usize {
    b.windows(ago.len()).position(|w| w == ago).unwrap()
}

#[test]
fn a_netcdf4_file_has_its_key() {
    let mut b = b"\x89HDF\r\n\x1a\n".to_vec();
    b.resize(512, 0);
    assert_eq!(andi_error(b), "err.file.netcdf4");
}

#[test]
fn no_file_length_panics_the_reader() {
    let b = buono();
    for n in (0..b.len()).step_by(7).chain([b.len() - 1]) {
        let _ = andi_error(b[..n].to_vec());
    }
}

#[test]
fn huge_counts_and_sizes_are_errors_without_allocating() {
    let b = buono();
    // the number of dimensions, of attributes and of variables
    for da in [
        8usize,
        pos(&b, b"experiment_title") - 4 - 4 - 4,
        pos(&b, b"scan_acquisition_time") - 8 - 4,
    ] {
        let mut x = b.clone();
        x[da + 4..da + 8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_ne!(andi_error(x), "ok", "{da}");
    }
    // the length of the dimension scan_number: more scans than the file has points for
    let mut x = b.clone();
    let i = pos(&x, b"scan_number") + 12;
    x[i..i + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_ne!(andi_error(x), "ok");
}

#[test]
fn a_too_long_name_is_an_error() {
    let mut x = buono();
    let i = pos(&x, b"point_number") - 4;
    x[i..i + 4].copy_from_slice(&(1u32 << 30).to_be_bytes());
    assert_eq!(andi_error(x), "err.dam.unreadable");
}

#[test]
fn a_begin_outside_the_file_is_a_keyed_error() {
    let b = buono();
    // the `begin` of scan_index: the last four bytes of its entry
    let i = pos(&b, b"scan_index") + 12 + 4 + 4 + 8 + 4 + 4;
    let mut x = b.clone();
    x[i..i + 4].copy_from_slice(&0x7fff_fff0u32.to_be_bytes());
    assert_eq!(andi_error(x), "err.dam.unreadable");
}

#[test]
fn negative_point_count_or_indices_outside_the_points_do_not_panic() {
    let b = buono();
    for (name, value) in [
        ("point_count", -5i32),
        ("point_count", 1 << 30),
        ("scan_index", 1 << 30),
        ("scan_index", -1),
    ] {
        let i = pos(&b, name.as_bytes());
        let mut x = b.clone();
        // the data of the variable: its `begin` is in the header, found through the variable table
        let begin = u32::from_be_bytes(x[i + 36..i + 40].try_into().unwrap()) as usize;
        x[begin..begin + 4].copy_from_slice(&value.to_be_bytes());
        let _ = andi_error(x);
    }
}

/// Every real `.cdf` of the data repository: the scans counted, the points sorted and finite, the TIC of the file equal to the sum of the points, the times increasing.
#[test]
fn real_cdfs_open_and_the_counts_match() {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/netcdf_cdf") else {
        eprintln!("SKIP andi: no data repository (MZLAB_DATI)");
        return;
    };
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cdf")) {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_andi(std::fs::read(&p).unwrap());
        let n = l.samples()[0].scansioni;
        assert!(n > 100, "{name}");
        for (prefisso, attese) in [
            ("MTBLS7740", 7231),
            ("MTBLS8819", 14960),
            ("MTBLS13991", 4109),
        ] {
            if name.starts_with(prefisso) {
                assert_eq!(n, attese, "{name}");
            }
        }
        let index = l.index(0).unwrap().to_vec();
        assert!(index.windows(2).all(|w| w[0].rt <= w[1].rt), "{name}");
        for k in (0..n).step_by((n / 40).max(1)) {
            let s = l.scan(0, k).unwrap();
            assert!(
                !s.mz.is_empty() && s.mz.windows(2).all(|w| w[0] <= w[1]),
                "{name} scan {k}"
            );
            let summed: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
            assert!(
                (summed - index[k].tic).abs() <= 1e-3 * summed.max(1.0),
                "{name} scan {k}: {summed} {}",
                index[k].tic
            );
        }
        assert!(l.method(0).unwrap().contains("\"andi-ms\""), "{name}");
    }
}
