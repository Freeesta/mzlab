//! Tests of `BlockReader` over an in-memory `Fetch`.
struct Mem(Vec<u8>, usize);
impl super::Fetch for Mem {
    fn fetch(&mut self, pos: u64, want: u64, into: &mut Vec<u8>) -> std::io::Result<()> {
        self.1 += 1;
        into.clear();
        into.extend_from_slice(self.0.get(pos as usize..(pos + want) as usize).unwrap_or_default());
        Ok(())
    }
}

#[test]
fn block_reader_is_bufread_without_a_second_copy() {
    use std::io::{BufRead, Read, Seek, SeekFrom};
    let data: Vec<u8> = (0..(super::BLOCK * 2 + 10))
        .map(|i| (i % 251) as u8)
        .collect();
    let mut r = super::BlockReader::with_fetch(Mem(data.clone(), 0), data.len() as u64);
    // the buffer handed out is the block itself: one block, not a 1 MiB copy of it
    assert_eq!(r.fill_buf().unwrap().len(), super::BLOCK);
    r.consume(5);
    assert_eq!(r.fill_buf().unwrap().len(), super::BLOCK - 5);
    r.seek(SeekFrom::Start(0)).unwrap();
    let mut all = Vec::new();
    r.read_to_end(&mut all).unwrap();
    assert_eq!(all, data);
    assert_eq!(r.src.1, 3);
    r.seek(SeekFrom::End(-4)).unwrap();
    assert_eq!(r.fill_buf().unwrap(), &data[data.len() - 4..]);
}

#[test]
fn reads_past_the_end_or_after_a_far_seek_return_nothing() {
    use std::io::{BufRead, Read, Seek, SeekFrom};
    let data: Vec<u8> = (0..100u8).collect();
    let mut r = super::BlockReader::with_fetch(Mem(data.clone(), 0), data.len() as u64);
    r.seek(SeekFrom::Start(1 << 40)).unwrap();
    assert!(r.fill_buf().unwrap().is_empty());
    let mut b = [0u8; 8];
    assert_eq!(r.read(&mut b).unwrap(), 0);
    assert!(r.seek(SeekFrom::Current(-(1i64 << 41))).is_err());
    // a fill with the cursor past the end asks for nothing instead of underflowing
    r.fill().unwrap();
    assert!(r.block.is_empty());
    assert_eq!(r.src.1, 1);
}
