//! `Mzml::chromatograms` read in windows against the search over the whole file in one buffer.
use super::super::{attr_id, find, find_tag, Mzml};
use super::WINDOW;
use crate::error::Result;
use crate::idxread::ReadAt;

/// A file in memory that counts its reads and remembers the longest.
struct Counting {
    data: Vec<u8>,
    calls: usize,
    longest: usize,
}

impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, out: &mut [u8]) -> Result<()> {
        self.calls += 1;
        self.longest = self.longest.max(out.len());
        self.data.read_at(off, out)
    }
}

/// (id, start, end) of every chromatogram, searched in the whole file at once.
fn whole(f: &[u8], from: usize) -> Vec<(String, u64, u64)> {
    let mut out = vec![];
    let mut pos = from;
    while let Some(a) = find_tag(f, b"<chromatogram", pos) {
        let Some(b) = find(f, b"</chromatogram>", a) else { break };
        let c = find(&f[..b], b"<binaryDataArrayList", a).unwrap_or(b);
        out.push((attr_id(&String::from_utf8_lossy(&f[a..c])).unwrap_or_default(), a as u64, (b + 15) as u64));
        pos = b + 1;
    }
    out
}

fn chrom(i: usize, body: usize) -> Vec<u8> {
    let mut c = format!("<chromatogram index=\"{i}\" id=\"SRM SIC Q1=100.{i} Q3=50.{i}\" defaultArrayLength=\"3\"><cvParam accession=\"MS:1001473\"/>").into_bytes();
    if i % 3 != 2 {
        c.extend_from_slice(b"<binaryDataArrayList count=\"2\"><binaryDataArray><binary>");
    }
    c.extend(std::iter::repeat_n(b'A', body));
    if i % 3 != 2 {
        c.extend_from_slice(b"</binary></binaryDataArray></binaryDataArrayList>");
    }
    c.extend_from_slice(b"</chromatogram>");
    c
}

#[test]
fn the_windowed_list_is_the_whole_file_list_across_window_edges() {
    let w = WINDOW as usize;
    for pad in [0, 3, w - 200, w - 120, w - 60, w - 14, w - 13, w - 1, w, w + 7] {
        let mut f = b"<mzML><run><chromatogramList>".to_vec();
        f.extend(std::iter::repeat_n(b' ', pad));
        // decoys: a longer tag name and a closing tag that is not one
        f.extend_from_slice(b"<chromatogramList2 x=\"1\"/>");
        for i in 0..12 {
            f.extend(chrom(i, if i == 4 { w + 333 } else { 40 + 9 * i }));
        }
        f.extend_from_slice(b"<chromatogram id=\"cut\"><binaryDataArrayList>");
        let want = whole(&f, 0);
        assert_eq!(want.len(), 12);
        let size = f.len() as u64;
        let mut m = Mzml::new(Counting { data: f, calls: 0, longest: 0 }, size);
        let got: Vec<(String, u64, u64)> = m.chromatograms(0).unwrap().into_iter().map(|c| (c.id, c.start, c.end)).collect();
        assert_eq!(got, want, "pad {pad}");
        assert!(m.r.longest <= w, "pad {pad}: a read of {} bytes", m.r.longest);
        let from = want[5].1 as usize;
        let size = m.size;
        let mut m2 = Mzml::new(m.r.data.clone(), size);
        let got: Vec<(String, u64, u64)> = m2.chromatograms(from as u64).unwrap().into_iter().map(|c| (c.id, c.start, c.end)).collect();
        assert_eq!(got, whole(&m.r.data, from), "pad {pad} from {from}");
    }
}
