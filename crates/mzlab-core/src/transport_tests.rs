// The tests of transport.rs: what the module writes is what the engine reads, bit by bit.
use super::*;

fn entries() -> Vec<ScanEntry> {
    vec![
        ScanEntry {
            id: "scan=1 \"q\"\né".into(),
            rt: 0.1234567890123,
            livello: 2,
            polarita: -1,
            tic: 1e300,
            base_peak: None,
            filtro: "FTMS + c ESI Full ms2 445.12@hcd30.00 [50.00-500.00]".into(),
            precursore: Some(445.12),
            position: Some(Position {
                membro: 3,
                offset: 1 << 40,
                lunghezza: 77,
            }),
        },
        ScanEntry {
            id: String::new(),
            rt: f64::INFINITY,
            livello: 1,
            polarita: 0,
            tic: 0.1 + 0.2,
            base_peak: Some(200.25),
            filtro: String::new(),
            precursore: None,
            position: None,
        },
        ScanEntry {
            id: "scan=3".into(),
            rt: -0.0,
            livello: 255,
            polarita: 1,
            tic: 0.0,
            base_peak: Some(-1.5),
            filtro: "FTMS + c ESI Full ms2 445.12@hcd30.00 [50.00-500.00]".into(),
            precursore: Some(0.0),
            position: Some(Position {
                membro: 0,
                offset: 0,
                lunghezza: 0,
            }),
        },
    ]
}

#[test]
fn the_index_comes_back_equal() {
    let v = entries();
    let b = encode_index(&v);
    assert_eq!(decode_index(&b).unwrap(), v);
    // each filter text is written once
    assert_eq!(
        b.windows(4).filter(|w| *w == b"FTMS").count(),
        1,
        "the repeated filter is kept once"
    );
    assert_eq!(decode_index(&encode_index(&[])).unwrap(), vec![]);
}

#[test]
fn a_truncated_or_other_version_index_is_refused() {
    let b = encode_index(&entries());
    for n in [0, 3, 23, 24, b.len() / 2, b.len() - 1] {
        assert!(decode_index(&b[..n]).is_err(), "{n}");
    }
    let mut v1 = b.clone();
    v1[0] = 1;
    assert!(decode_index(&v1).is_err());
    // a count that no block could hold is refused before allocating
    let mut enorme = b.clone();
    enorme[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_index(&enorme).is_err());
}

fn scan(wide: Option<Vec<f64>>) -> Spectrum {
    Spectrum {
        index: 7,
        id: "ignored".into(),
        level: 2,
        rt: 12.5,
        polarity: -1,
        profile: true,
        filter: "ignored".into(),
        precursor: Some(445.5),
        mz: vec![100.0, 200.5, 300.25],
        intensity: vec![1.5, 2.0, f32::NAN],
        wide,
    }
}

#[test]
fn the_scan_comes_back_equal_without_id_and_filter() {
    let s = scan(None);
    let b = encode_scansione(&s);
    assert_eq!(b.len(), SCAN_HEAD + 3 * 12);
    let d = decode_scansione(7, &b).unwrap();
    assert_eq!((d.id.as_str(), d.filter.as_str()), ("", ""));
    assert_eq!(
        (d.index, d.level, d.polarity, d.profile, d.rt, d.precursor),
        (7, 2, -1, true, 12.5, Some(445.5))
    );
    assert_eq!(d.mz, s.mz);
    assert_eq!(d.intensity[..2], s.intensity[..2]);
    assert!(d.intensity[2].is_nan());
    assert!(d.wide.is_none());
    let without = Spectrum {
        precursor: None,
        profile: false,
        ..s
    };
    let d = decode_scansione(0, &encode_scansione(&without)).unwrap();
    assert_eq!((d.precursor, d.profile), (None, false));
}

#[test]
fn wide_intensities_stay_wide_only_when_needed() {
    let w = vec![1.5, 16777217.0, 3.0];
    let s = Spectrum {
        intensity: w.iter().map(|&x| x as f32).collect(),
        ..scan(Some(w.clone()))
    };
    let b = encode_scansione(&s);
    assert_eq!(b.len(), SCAN_HEAD + 3 * 16);
    assert_eq!(decode_scansione(0, &b).unwrap().wide, Some(w));
    // 64-bit values that a 32-bit one holds exactly are not kept wide
    let exact = vec![1.5, 2.0, 3.0];
    let s = Spectrum {
        intensity: exact.iter().map(|&x| x as f32).collect(),
        ..scan(Some(exact))
    };
    assert!(decode_scansione(0, &encode_scansione(&s))
        .unwrap()
        .wide
        .is_none());
}

#[test]
fn a_scan_of_wrong_size_is_refused() {
    let b = encode_scansione(&scan(None));
    assert!(decode_scansione(0, &b[..b.len() - 1]).is_err());
    assert!(decode_scansione(0, &b[..10]).is_err());
    let mut long = b.clone();
    long.push(0);
    assert!(decode_scansione(0, &long).is_err());
    let mut enorme = b;
    enorme[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_scansione(0, &enorme).is_err());
    let empty = Spectrum {
        mz: vec![],
        intensity: vec![],
        ..scan(None)
    };
    assert_eq!(
        decode_scansione(0, &encode_scansione(&empty))
            .unwrap()
            .mz
            .len(),
        0
    );
}

/// The buffer of the index is allocated once at its exact size (7 columns of f64 per row, not 6).
#[test]
fn the_index_buffer_is_sized_exactly() {
    let b = encode_index(&entries());
    assert_eq!(b.capacity(), b.len());
}

/// An index that declares `u32::MAX` filters is refused as corrupt (on wasm32 `nf + 1` wrapped to 0 before the offsets were read).
#[test]
fn an_index_with_a_filter_count_at_the_limit_is_refused() {
    let mut b = encode_index(&entries());
    b[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_index(&b).is_err());
}

/// The columns crossing apart (`scan_from_parts`) give the scan that the block gives, wide or not; a part of the wrong size is refused.
#[test]
fn the_scan_from_its_parts_is_the_one_from_the_block() {
    let w = vec![1.5, 16777217.0, 3.0];
    let wide = Spectrum {
        intensity: w.iter().map(|&x| x as f32).collect(),
        ..scan(Some(w.clone()))
    };
    for s in [
        scan(None),
        wide,
        Spectrum {
            precursor: None,
            profile: false,
            ..scan(None)
        },
    ] {
        let y = match &s.wide {
            Some(w) => Intensity::Larghe(w.clone()),
            None => Intensity::Strette(s.intensity.clone()),
        };
        let p = scan_from_parts(4, &testa_scansione(&s), s.mz.clone(), y).unwrap();
        let b = decode_scansione(4, &encode_scansione(&s)).unwrap();
        assert_eq!(format!("{p:?}"), format!("{b:?}"));
    }
    let s = scan(None);
    let t = testa_scansione(&s);
    assert!(scan_from_parts(0, &t, vec![1.0], Intensity::Strette(s.intensity.clone())).is_err());
    assert!(scan_from_parts(0, &t, s.mz.clone(), Intensity::Larghe(vec![1.0; 3])).is_err());
    assert!(scan_from_parts(0, &t[..10], s.mz.clone(), Intensity::Strette(s.intensity)).is_err());
}
