//! Every count of an ANDI-MS header is confirmed by a second source and a fill value is no value: a file that lies about its counts is `err.file.cdfDamaged`,
//! never one read with other scans or points and no word. Synthetic files of `common/andi.rs`; the last test changes each byte of the header in turn.
#![cfg(feature = "andi")]
mod common;
use common::andi::*;
use mzlab_core::readers::Limits;

const NC_FILL_DOUBLE: f64 = 9.969_209_968_386_869e36;

/// What a reader gives of a file: per scan the RT, TIC and the bits of every point; `None` for an error, with `missing` when scans are counted as lost.
fn result(b: Vec<u8>) -> Result<(usize, Vec<String>), &'static str> {
    let mut l =
        mzlab_lettori::apri(common::file_set("run.cdf", b), &Limits::sito()).map_err(|e| e.key)?;
    let ix = l.index(0).map_err(|e| e.key)?.to_vec();
    let mut out = Vec::new();
    for (k, e) in ix.iter().enumerate() {
        let s = l.scan(0, k).map_err(|e| e.key)?;
        let pts: Vec<(u64, u64)> = (0..s.mz.len())
            .map(|i| (s.mz[i].to_bits(), s.intensity_f64(i).to_bits()))
            .collect();
        out.push(format!("{} {} {:?}", e.rt.to_bits(), e.tic.to_bits(), pts));
    }
    Ok((l.missing(0), out))
}

fn edited(edit: impl Fn(&mut Vec<Vb>)) -> Vec<u8> {
    andi_ms_edited(&FLAT_RUN, SCANS, points, true, edit)
}

fn set(name: &'static str, k: usize, x: f64) -> Vec<u8> {
    edited(move |vs| vs.iter_mut().find(|v| v.name == name).unwrap().data[k] = x)
}

fn pos(b: &[u8], name: &[u8]) -> usize {
    b.windows(name.len()).position(|w| w == name).unwrap()
}

#[test]
fn starts_and_counts_that_lie_are_damage() {
    for (name, k, x) in [
        ("scan_index", 2, -3.0),
        ("scan_index", 2, 1.0),
        ("scan_index", 2, -2_147_483_647.0),
        ("point_count", 1, 9.0),
        ("point_count", 1, -9999.0),
        ("point_count", 5, 40.0),
    ] {
        assert_eq!(
            result(set(name, k, x)),
            Err("err.file.cdfDamaged"),
            "{name}[{k}] = {x}"
        );
    }
    // a gap between scans and an empty scan are not damage
    let gap = edited(|vs| {
        let c = vs.iter_mut().find(|v| v.name == "point_count").unwrap();
        (c.data[1], c.data[2]) = (c.data[1] - 1.0, 0.0);
    });
    let (_, scans) = result(gap).unwrap();
    assert_eq!(scans.len(), SCANS);
}

#[test]
fn dimension_and_record_counts_are_checked_against_the_bytes() {
    let good = andi_ms(&FLAT_RUN);
    let dim = pos(&good, b"scan_number") + 12;
    for n in [5u32, 7] {
        let mut b = good.clone();
        b[dim..dim + 4].copy_from_slice(&n.to_be_bytes());
        assert_eq!(result(b), Err("err.file.cdfDamaged"), "scan_number = {n}");
    }
    let numrecs = u32::from_be_bytes(good[4..8].try_into().unwrap());
    let mut b = good.clone();
    b[4..8].copy_from_slice(&(numrecs - 3).to_be_bytes());
    assert_eq!(result(b), Err("err.file.cdfDamaged"));
}

#[test]
fn a_fill_value_is_no_value() {
    let (_, good) = result(andi_ms(&FLAT_RUN)).unwrap();
    // a TIC that is a fill is summed from the points: here the same as the file's
    for x in [-9999.0, NC_FILL_DOUBLE] {
        assert_eq!(result(set("total_intensity", 1, x)).unwrap().1, good);
    }
    // a scan without a time and no `time_values` to take it from
    for x in [-9999.0, NC_FILL_DOUBLE] {
        assert_eq!(
            result(set("scan_acquisition_time", 1, x)),
            Err("err.file.cdfDamaged")
        );
    }
    // a point whose m/z is a fill (the default of a float, or the variable's `_FillValue`) is dropped
    let fill = f64::from(9.969_21e36_f32);
    let (_, s) = result(set("mass_values", 0, fill)).unwrap();
    assert_eq!(s[1..], good[1..]);
    assert_ne!(s[0], good[0]);
    let own = edited(|vs| {
        let m = vs.iter_mut().find(|v| v.name == "mass_values").unwrap();
        m.att.push(("_FillValue".into(), At::N(5, vec![-1.0])));
        m.data[0] = -1.0;
    });
    assert_eq!(result(own).unwrap().1, s);
}

/// Each byte of the header changed in three ways: the file opens with the same scans, warns of lost scans, or is an error (classic and 64-bit offsets).
#[test]
fn no_header_byte_changes_the_result_without_a_word() {
    for c in [
        FLAT_RUN,
        Run {
            v2: true,
            ..FLAT_RUN
        },
    ] {
        let good = andi_ms(&c);
        let expected = result(good.clone()).unwrap();
        // the first variable is the time, whose data start right after the header: its `begin` follows the padded name, the dimension, the absent attributes, type and vsize
        let at = pos(&good, b"scan_acquisition_time") + 48;
        let header = if c.v2 {
            u64::from_be_bytes(good[at..at + 8].try_into().unwrap()) as usize
        } else {
            u32::from_be_bytes(good[at..at + 4].try_into().unwrap()) as usize
        };
        let mut silent = Vec::new();
        for i in 0..header {
            for x in [0x01u8, 0x80, 0xff] {
                let mut b = good.clone();
                b[i] ^= x;
                if let Ok((0, scans)) = result(b)
                    && scans != expected.1
                {
                    silent.push((i, x));
                }
            }
        }
        assert!(
            silent.is_empty(),
            "v2 {}: header bytes that change the scans silently: {silent:?}",
            c.v2
        );
    }
}
