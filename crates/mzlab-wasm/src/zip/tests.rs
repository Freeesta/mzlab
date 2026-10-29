use super::*;

// The two-step code the exports had before (size, then write into a buffer prepared by the page), kept as the reference.
fn old_zip(packed: &[u8]) -> Result<Vec<u8>, String> {
    let z = mzlab_core::zip::zip_packed(packed)?;
    let mut out = vec![0u8; z.len()];
    out.get_mut(..z.len())
        .ok_or("zip: output too small")?
        .copy_from_slice(&z);
    Ok(out)
}
fn old_unzip(zip: &[u8]) -> Result<Vec<u8>, String> {
    let p = mzlab_core::zip::unzip_packed(zip)?;
    let mut out = vec![0u8; p.len()];
    out.get_mut(..p.len())
        .ok_or("zip: output too small")?
        .copy_from_slice(&p);
    Ok(out)
}

fn sets() -> Vec<Vec<(String, Vec<u8>)>> {
    let many = (0..300)
        .map(|i| (format!("d/f{i}.json"), vec![(i % 251) as u8; i * 7 % 500]))
        .collect();
    let big = vec![(
        String::from("opfs/big.bin"),
        (0..20_000_000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 24) as u8)
            .collect(),
    )];
    vec![
        vec![],
        vec![(String::from("manifesto.json"), b"{}".to_vec())],
        many,
        big,
        vec![(String::from("opfs/librerie/àè–日本.msp"), b"x".to_vec())],
    ]
}

#[test]
fn one_pass_equals_the_old_two_step_and_round_trips() {
    for set in sets() {
        let files: Vec<(&str, &[u8])> = set
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let packed = mzlab_core::zip::pack(&files);
        let z = zip_bytes(&packed).unwrap();
        assert_eq!(z, old_zip(&packed).unwrap());
        let back = unzip_bytes(&z).unwrap();
        assert_eq!(back, old_unzip(&z).unwrap());
        assert_eq!(back, packed);
    }
}

#[test]
fn errors_keep_their_messages() {
    let packed = mzlab_core::zip::pack(&[("a.txt", b"hello world".as_slice())]);
    let z = zip_bytes(&packed).unwrap();
    // truncated ZIP and truncated packed input
    let cut = &z[..z.len() / 2];
    assert!(unzip_bytes(cut).is_err());
    assert_eq!(unzip_bytes(cut), old_unzip(cut));
    let cut = &packed[..packed.len() - 3];
    assert!(zip_bytes(cut).is_err());
    assert_eq!(zip_bytes(cut), old_zip(cut));
    // bad CRC: flip a data byte ("hello world" sits right after the local header)
    let mut bad = z.clone();
    let at = bad.windows(11).position(|w| w == b"hello world").unwrap();
    bad[at] ^= 0xff;
    assert!(unzip_bytes(&bad).is_err());
    assert_eq!(unzip_bytes(&bad), old_unzip(&bad));
    // unsafe name
    let evil = mzlab_core::zip::zip_stored(&[("../evil", b"x".as_slice())]).unwrap();
    assert!(unzip_bytes(&evil).is_err());
    assert_eq!(unzip_bytes(&evil), old_unzip(&evil));
}
