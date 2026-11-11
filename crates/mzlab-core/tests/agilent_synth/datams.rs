//! The synthetic `data.ms` encoder of the Agilent tests (ChemStation GC/MS records, big-endian).

/// A `data.ms` written here (big-endian records of whole twentieths and 2-bit-exponent abundances): a run with only that file opens.
pub fn encode_data_ms(scans: &[(u32, Vec<(u16, u16)>)]) -> Vec<u8> {
    let start = 0x140usize;
    let mut b = vec![0u8; start];
    b[0] = 1;
    b[1] = 0x32;
    for (o, s) in [
        (0x04usize, "GC / MS Data File"),
        (0x18, "run one"),
        (0xB2, "1 Jan 25  01:00 am"),
    ] {
        b[o] = s.len() as u8;
        b[o + 1..o + 1 + s.len()].copy_from_slice(s.as_bytes());
    }
    b[0x10A..0x10C].copy_from_slice(&(((start + 2) / 2) as u16).to_be_bytes());
    b[0x116..0x11A].copy_from_slice(&(scans.len() as u32).to_be_bytes());
    for (ms, peaks) in scans {
        let words = (24 + 4 * (peaks.len() + 1)) / 2;
        b.extend((words as u16).to_be_bytes());
        b.extend(ms.to_be_bytes());
        b.extend([0, 12, 0, 1, 0, 0]);
        b.extend((peaks.len() as u16).to_be_bytes());
        let top = peaks
            .iter()
            .max_by_key(|p| p.1 & 0x3fff)
            .copied()
            .unwrap_or((0, 0));
        b.extend(top.0.to_be_bytes());
        b.extend(top.1.to_be_bytes());
        for p in peaks.iter().rev() {
            b.extend(p.0.to_be_bytes());
            b.extend(p.1.to_be_bytes());
        }
        b.extend([0, 5, 0, 4, 0, 0]);
        b.extend(
            peaks
                .iter()
                .map(|p| ((p.1 & 0x3fff) as u32) << (3 * (p.1 >> 14)))
                .sum::<u32>()
                .to_be_bytes(),
        );
    }
    b.extend([9u8; 20]);
    b
}
