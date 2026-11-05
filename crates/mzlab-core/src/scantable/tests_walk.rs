//! The walk of an mzML without an index list: the rows against the spectra placed in the text, the bytes it reads, and the tag search.

use super::*;
use crate::window::Windowed;

struct Counting {
    data: Vec<u8>,
    bytes: u64,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.bytes += buf.len() as u64;
        self.data.read_at(off, buf)
    }
}

/// A non-indexed mzML with one spectrum per entry of `sizes` (bytes of base64), and where each spectrum starts and ends.
fn mzml(sizes: &[usize]) -> (Vec<u8>, Vec<(u64, u64)>) {
    let mut s = String::from("<?xml version=\"1.0\"?><mzML><run id=\"r\"><spectrumList count=\"");
    s.push_str(&format!("{}\">\n", sizes.len()));
    let mut spans = Vec::new();
    for (i, &n) in sizes.iter().enumerate() {
        let a = s.len() as u64;
        s.push_str(&format!(
            "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{}\"/><scanList count=\"1\"><scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitName=\"minute\"/></scan></scanList><binaryDataArrayList count=\"1\"><binaryDataArray encodedLength=\"{n}\"><binary>",
            i + 1,
            1 + i % 2,
            i as f64 * 0.01
        ));
        s.push_str(&"QUJD".repeat(n / 4));
        s.push_str("</binary></binaryDataArray></binaryDataArrayList></spectrum>");
        spans.push((a, s.len() as u64));
        s.push('\n');
    }
    s.push_str("</spectrumList></run></mzML>\n");
    (s.into_bytes(), spans)
}

fn walk(data: &[u8]) -> (Table, u64) {
    let mut c = Counting {
        data: data.to_vec(),
        bytes: 0,
    };
    let size = data.len() as u64;
    let t = read_in(&mut Windowed::new(&mut c, size), size, None).unwrap();
    (t, c.bytes)
}

#[test]
fn walked_rows_match_the_spectra_whatever_their_size() {
    // small ones, ones that cross the 1 MB block, ones longer than a block, and a long one last
    let sizes = [
        5_000, 300_000, 700_000, 1_500_000, 40, 990_000, 3_000_000, 64, 500_000, 2_100_000,
    ];
    let (data, spans) = mzml(&sizes);
    let (t, _) = walk(&data);
    assert_eq!(t.rows.len(), spans.len());
    for (i, (r, &(a, b))) in t.rows.iter().zip(&spans).enumerate() {
        assert_eq!((r.start, r.end), (a, b), "spectrum {i}");
        assert_eq!(r.level, 1 + (i % 2) as u8);
        assert_eq!(r.rt, i as f64 * 0.01);
    }
}

#[test]
fn walk_reads_a_spectrum_that_crosses_a_block_once() {
    // 400 spectra of ~50 KB: each 1 MB block ends inside a spectrum; reading it again from its start must also serve the spectra after it
    let (data, spans) = mzml(&[50_000; 400]);
    let (t, bytes) = walk(&data);
    assert_eq!(t.rows.len(), spans.len());
    let size = data.len() as u64;
    // the whole file once, plus the first read of the header (at most 4 MB) and a few blocks
    assert!(
        bytes <= size + (6 << 20),
        "{bytes} bytes read for a file of {size}"
    );
}

#[test]
fn find_matches_a_plain_search() {
    let mut x = 7u64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let naive = |h: &[u8], p: &[u8]| {
        if p.is_empty() {
            return None;
        }
        h.windows(p.len()).position(|w| w == p)
    };
    for len in [0usize, 1, 5, 31, 32, 33, 63, 64, 65, 100, 1000] {
        for _ in 0..50 {
            // few letters, so the first byte and partial matches are frequent, across the 32-byte blocks
            let h: Vec<u8> = (0..len).map(|_| b"<ab"[(next() % 3) as usize]).collect();
            for p in [
                &b"<"[..],
                b"<a",
                b"<ab<",
                b"ab",
                b"",
                b"<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
            ] {
                assert_eq!(find(&h, p), naive(&h, p), "{h:?} {p:?}");
            }
        }
    }
}
