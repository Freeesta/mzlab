//! `file_sha`: the hash of a file read in pieces is the hash of its bytes.

use super::*;

struct Bytes(Vec<u8>);

impl ReadAt for Bytes {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        buf.copy_from_slice(&self.0[off as usize..off as usize + buf.len()]);
        Ok(())
    }
}

#[test]
fn the_sha_of_a_file_read_in_pieces_is_the_sha_of_its_bytes() {
    let data: Vec<u8> = (0..(2 << 20) + 17).map(|i| (i * 31 % 251) as u8).collect();
    let want = mzlab_core::budget::sha256_hex(&data);
    let size = data.len() as u64;
    assert_eq!(file_sha(&mut Bytes(data), size).unwrap(), want);
}
