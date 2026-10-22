//! A synthetic Thermo `.raw` for the tests: invented numbers, laid out the way `vendor/opentfraw` reads a TSQ run of format v57-v63 (one flat peak list per scan,
//! no scan events). It is the only way to build the cut, the undocumented-version and the unreadable-stream cases without a real file.
#![allow(dead_code)]

/// Where the scan data sits relative to the streams that describe it.
#[derive(Clone, Copy, PartialEq)]
pub enum Layout {
    /// header, data, scan index, run header (a cut in the data loses the index too).
    DataFirst,
    /// header, run header, scan index, data (a cut in the data leaves the index whole).
    DataLast,
}

pub struct Raw {
    /// Version of the layout written (57, 60, 62 or 63).
    pub layout_version: u32,
    /// Number written in the header (differs from `layout_version` for the undocumented-version cases).
    pub declared_version: u32,
    pub scans: u32,
    pub peaks: u32,
    pub layout: Layout,
    /// Error-log entries declared in the run header, whatever the file holds.
    pub error_entries: u32,
    /// Scan-event count written at the scan-trailer address.
    pub events: u32,
}

impl Raw {
    pub fn tsq(version: u32, scans: u32) -> Self {
        Self {
            layout_version: version,
            declared_version: version,
            scans,
            peaks: 3,
            layout: Layout::DataFirst,
            error_entries: 0,
            events: 0,
        }
    }

    fn u16s(v: &[u16]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn f64s(v: &[f64]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }
    fn utf16(s: &str, bytes: usize) -> Vec<u8> {
        let mut b: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
        b.resize(bytes, 0);
        b
    }

    fn header(&self) -> Vec<u8> {
        let mut b = Self::u16s(&[0xa101]);
        b.extend(Self::utf16("Finnigan", 18));
        b.extend([0u8; 16]);
        b.extend(Self::u32s(&[self.declared_version]));
        b.extend(vec![0u8; 112 * 2 + 4 + 60]); // two audit tags, unknown_long, unknown_area
        b.extend(Self::utf16("tag", 1028));
        assert_eq!(b.len(), 1356);
        b
    }

    /// SeqRow, ASInfo and RawFileInfo: only their size matters, so the strings are empty.
    fn middle(&self, data_addr: u32, run_header_addr: u32) -> Vec<u8> {
        let v = self.layout_version;
        let mut b = Self::u32s(&[0, 1, 0]);
        b.extend(Self::utf16("1", 12));
        b.extend(Self::f64s(&[0.0; 5]));
        let mut strings = 2 + 2 + 5 + 5; // unknown x2, id, comment, labels, methods, file name, path, vial
        if v >= 57 {
            strings += 2;
        }
        if v >= 60 {
            strings += 15;
        }
        b.extend(vec![0u8; strings * 4]);
        if v >= 57 {
            b.extend([0u8; 4]);
        }
        b.extend([0u8; 24 + 4]); // ASInfo preamble and empty string
        b.extend(Self::u32s(&[0])); // RawFileInfo: method file present
        b.extend(Self::u16s(&[2020, 1, 3, 1, 10, 0, 0, 0]));
        b.extend(Self::u32s(&[
            0,
            data_addr,
            1,
            1,
            0,
            0,
            run_header_addr,
            0,
            0,
            0,
        ]));
        b.extend(vec![0u8; 744]);
        b.extend(vec![0u8; 6 * 4]);
        b
    }

    fn run_header(&self, at: u32, index_addr: u32, data_addr: u32, trailer_addr: u32) -> Vec<u8> {
        let mut b = Self::u32s(&[0, 0, 1, self.scans, 0, self.error_entries, 0]);
        b.extend(Self::u32s(&[index_addr, data_addr, index_addr, index_addr]));
        b.extend(Self::u32s(&[0]));
        b.extend(Self::f64s(&[
            1e6,
            100.0,
            900.0,
            0.0,
            f64::from(self.scans) / 10.0,
        ]));
        b.extend(vec![0u8; 56 + 88 + 40 + 320]);
        assert_eq!(b.len(), 592);
        for i in 0..13 {
            if i == 6 {
                b.extend(Self::f64s(&[0.0, 0.0]));
            }
            b.extend(vec![0u8; 520]);
        }
        // scan trailer address, scan params address, ntrailer, nparams, nsegs, unknown x2, own address, unknown x2
        b.extend(Self::u32s(&[
            trailer_addr,
            trailer_addr + 4,
            0,
            0,
            1,
            0,
            0,
            at,
            0,
            0,
        ]));
        b
    }

    fn record(&self, k: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for p in 0..self.peaks {
            b.extend((300.0f32 + 10.0 * p as f32 + k as f32).to_le_bytes());
            b.extend((1000.0f32 * (p + 1) as f32).to_le_bytes());
        }
        b.extend(vec![0u8; self.peaks as usize]);
        b
    }

    fn index(&self, data_len: &[u32]) -> Vec<u8> {
        let mut b = Vec::new();
        let mut end = 0u32;
        for (k, len) in data_len.iter().enumerate() {
            end += len;
            b.extend(Self::u32s(&[end, k as u32 + 1]));
            b.extend(Self::u16s(&[1, 1]));
            b.extend(Self::u32s(&[0, 0, self.peaks + 1]));
            b.extend(Self::f64s(&[
                0.1 * (k as f64 + 1.0),
                5000.0,
                3000.0,
                320.0,
                100.0,
                900.0,
            ]));
        }
        b
    }

    pub fn bytes(&self) -> Vec<u8> {
        let records: Vec<Vec<u8>> = (0..self.scans).map(|k| self.record(k)).collect();
        let lens: Vec<u32> = records.iter().map(|r| r.len() as u32).collect();
        let index = self.index(&lens);
        let data: Vec<u8> = records.concat();
        let head = self.header();
        let mid_len = self.middle(0, 0).len();
        let after_mid = (head.len() + mid_len) as u32;
        let rh_len = self.run_header(0, 0, 0, 0).len() as u32;
        let (data_addr, rh_addr, index_addr) = match self.layout {
            Layout::DataFirst => {
                let rh = after_mid + data.len() as u32 + index.len() as u32 + 4;
                (after_mid, rh, after_mid + data.len() as u32)
            }
            Layout::DataLast => {
                let rh = after_mid;
                let ix = rh + rh_len;
                (ix + index.len() as u32 + 4, rh, ix)
            }
        };
        let trailer_addr = index_addr + index.len() as u32;
        let mut out = head;
        out.extend(self.middle(data_addr, rh_addr));
        let mut stream_events = Self::u32s(&[self.events]);
        stream_events.truncate(4);
        match self.layout {
            Layout::DataFirst => {
                out.extend(&data);
                out.extend(&index);
                out.extend(&stream_events);
                out.extend(self.run_header(rh_addr, index_addr, data_addr, trailer_addr));
            }
            Layout::DataLast => {
                out.extend(self.run_header(rh_addr, index_addr, data_addr, trailer_addr));
                out.extend(&index);
                out.extend(&stream_events);
                out.extend(&data);
            }
        }
        out
    }
}
