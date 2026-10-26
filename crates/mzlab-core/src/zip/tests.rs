use super::*;

#[test]
fn crc_known_values() {
    assert_eq!(crc32(b""), 0);
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
fn empty_archive_is_the_end_record_only() {
    let z = zip_stored(&[]).unwrap();
    assert_eq!(z.len(), 22);
    assert_eq!(&z[..4], &[0x50, 0x4B, 5, 6]);
}

#[test]
fn packed_round_trip_layout() {
    let z = zip_stored(&[("a.txt", b"hello"), ("d/vuoto", b"")]).unwrap();
    // 2 local headers (30 + name + data), 2 central entries (46 + name), end record 22
    assert_eq!(z.len(), (30 + 5 + 5) + (30 + 7) + (46 + 5) + (46 + 7) + 22);
    let mut packed = Vec::new();
    for (n, d) in [("a.txt", &b"hello"[..]), ("d/vuoto", &b""[..])] {
        packed.extend_from_slice(&(n.len() as u16).to_le_bytes());
        packed.extend_from_slice(&(d.len() as u32).to_le_bytes());
        packed.extend_from_slice(n.as_bytes());
        packed.extend_from_slice(d);
    }
    assert_eq!(zip_packed(&packed).unwrap(), z);
    assert!(unpack(&packed[..8]).is_err());
}

#[test]
fn reads_back_what_it_writes_and_refuses_the_rest() {
    let files: Vec<(&str, &[u8])> = vec![("a", b"xyz"), ("d/\u{e8}", b""), ("big", &[7u8; 70_000])];
    let z = zip_stored(&files).unwrap();
    let back = unzip_stored(&z).unwrap();
    assert_eq!(back.len(), 3);
    for ((n, d), (bn, bd)) in files.iter().zip(&back) {
        assert_eq!((*n, *d), (bn.as_str(), bd.as_slice()));
    }
    assert_eq!(unzip_stored(&zip_stored(&[]).unwrap()).unwrap().len(), 0);
    assert!(unzip_stored(b"not a zip at all, not even close.").is_err());
    assert!(unzip_stored(&zip_stored(&[("../x", b"1")]).unwrap()).is_err());
    assert!(unzip_stored(&zip_stored(&[("/abs", b"1")]).unwrap()).is_err());
    let mut crc = zip_stored(&[("a", b"xyz")]).unwrap();
    let last = 30 + 1 + 2;
    crc[last] ^= 1; // flip a data byte: CRC mismatch
    assert_eq!(unzip_stored(&crc).unwrap_err(), "zip: CRC mismatch");
}

/// With `MZLAB_ZIP_OUT=<dir>`: reads `py.zip`, an archive made by Python's `zipfile` (stored), written by `tests/test_zip_stored.py`.
#[test]
fn reads_an_archive_of_another_writer() {
    let Ok(dir) = std::env::var("MZLAB_ZIP_OUT") else {
        return;
    };
    let Ok(z) = std::fs::read(format!("{dir}/py.zip")) else {
        return;
    };
    let got = unzip_stored(&z).unwrap();
    assert_eq!(
        got,
        vec![
            ("a.txt".to_string(), b"hello".to_vec()),
            ("d/vuoto".to_string(), vec![]),
            ("d/\u{e8}.bin".to_string(), vec![0, 1, 2, 255])
        ]
    );
}

/// With `MZLAB_ZIP_OUT=<dir>`: writes the archives that `tests/test_zip_stored.py` reads back with Python's `zipfile`.
#[test]
fn write_for_the_independent_reader() {
    let Ok(dir) = std::env::var("MZLAB_ZIP_OUT") else {
        return;
    };
    let big: Vec<u8> = (0..70_000u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let files: Vec<(&str, &[u8])> = vec![
        ("manifesto.json", b"{\"v\":1}"),
        ("vuoto.txt", b""),
        ("sotto/\u{e8}.bin", &big),
        ("z", &[0, 255, 0]),
    ];
    std::fs::write(format!("{dir}/n.zip"), zip_stored(&files).unwrap()).unwrap();
    std::fs::write(format!("{dir}/zero.zip"), zip_stored(&[]).unwrap()).unwrap();
    std::fs::write(
        format!("{dir}/uno_vuoto.zip"),
        zip_stored(&[("v", b"")]).unwrap(),
    )
    .unwrap();
    std::fs::write(format!("{dir}/big.bin"), &big).unwrap();
}
