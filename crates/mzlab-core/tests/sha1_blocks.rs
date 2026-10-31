//! `metodo::sha1_hex` reads in blocks of at most 1 MB: the hash is that of the whole buffer around the block size, and for an empty member.

use mzlab_core::metodo::sha1_hex;
use sha1::{Digest as _, Sha1};

#[test]
fn sha1_of_blocks_at_and_around_one_megabyte() {
    let data: Vec<u8> = (0..(2 << 20) + 5).map(|i| (i * 31 % 251) as u8).collect();
    for len in [
        0usize,
        1,
        (1 << 20) - 1,
        1 << 20,
        (1 << 20) + 1,
        (2 << 20) + 5,
    ] {
        let want: String = Sha1::digest(&data[..len])
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            sha1_hex(&mut data.clone(), len as u64).unwrap(),
            want,
            "{len}"
        );
    }
}
