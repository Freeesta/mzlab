//! A synthetic DDA-PASEF folder (invented numbers, no lab data): MS1 frames and PASEF frames of 12 precursors each, codec 2, for the bench and the tests of the Bruker reader.

use super::V;

const SCANS: u32 = 918;
const PASEF_PER_CYCLE: usize = 10;
const PRECURSORS_PER_FRAME: usize = 12;
/// `IsolationMz` minus `MonoisotopicMz` of every precursor.
pub const ISO_OFFSET: f64 = 0.25;

/// xorshift: the same numbers on every machine.
pub struct Case(pub u64);
impl Case {
    pub fn n(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// A codec-2 block: `per_scan[s]` sorted TOF indices with their intensities.
pub fn block(per_scan: &[Vec<(u32, u32)>]) -> Vec<u8> {
    let ns = per_scan.len();
    let mut words: Vec<u32> = vec![0; ns];
    for s in 0..ns.saturating_sub(1) {
        words[s + 1] = 2 * per_scan[s].len() as u32;
    }
    for p in per_scan {
        let mut before = u32::MAX;
        for &(t, y) in p {
            words.push(t.wrapping_sub(before));
            words.push(y);
            before = t;
        }
    }
    let mut planes = Vec::with_capacity(words.len() * 4);
    for k in 0..4 {
        planes.extend(words.iter().map(|w| (w >> (8 * k)) as u8));
    }
    let z =
        ruzstd::encoding::compress_to_vec(&planes[..], ruzstd::encoding::CompressionLevel::Fastest);
    let mut b = ((z.len() + 8) as u32).to_le_bytes().to_vec();
    b.extend((ns as u32).to_le_bytes());
    b.extend(z);
    b
}

/// The peaks of a frame: `n` points spread over the scans `da..a`, TOF indices drawn around a few hundred ion lines.
fn peaks(c: &mut Case, n: usize, windows: &[(u32, u32)]) -> (Vec<Vec<(u32, u32)>>, u64) {
    let mut per_scan: Vec<Vec<(u32, u32)>> = vec![Vec::new(); SCANS as usize];
    for i in 0..n {
        let (a, b) = windows[i % windows.len()];
        let s = a + (c.n() % u64::from(b - a)) as u32;
        let line = (c.n() % 400) as u32 * 1000 + 20_000;
        let t = line + (c.n() % 40) as u32;
        let y = 10 + (c.n() % 500) as u32;
        per_scan[s as usize].push((t, y));
    }
    for p in &mut per_scan {
        p.sort_unstable();
        p.dedup_by_key(|x| x.0);
    }
    let tot = per_scan.iter().map(Vec::len).sum::<usize>() as u64;
    (per_scan, tot)
}

/// The `Frames` row of frame `id`.
fn frame_row(id: i64, ms1: bool, pos: i64, summed: u64, np: u64) -> Vec<V<'static>> {
    vec![
        V::N,
        V::R(0.1 * id as f64),
        V::T("+"),
        V::I(8),
        V::I(if ms1 { 0 } else { 8 }),
        V::I(pos),
        V::I(summed as i64),
        V::I(500),
        V::I(i64::from(SCANS)),
        V::I(np as i64),
        V::I(1),
        V::R(25.0),
        V::R(30.0),
        V::R(100.0),
    ]
}

/// `analysis.tdf`: the rows of the frames, precursors and PASEF windows, one calibration, codec 2.
fn database(
    frames: Vec<Vec<V<'static>>>,
    prec: Vec<Vec<V<'static>>>,
    pasef: Vec<Vec<V<'static>>>,
) -> Vec<u8> {
    let cal = vec![vec![
        V::N,
        V::I(1),
        V::R(0.125),
        V::R(18502.75),
        V::R(280.6435408329972),
        V::R(150581.44261048065),
        V::R(0.0),
        V::R(0.0),
        V::R(0.0),
        V::R(25.0),
        V::R(30.0),
        V::R(0.0),
        V::R(0.0),
    ]];
    let meta = vec![vec![V::T("TimsCompressionType"), V::T("2")]];
    super::db_rows(&[
        ("GlobalMetadata", "CREATE TABLE GlobalMetadata (Key TEXT, Value TEXT)", meta),
        ("Frames", "CREATE TABLE Frames (Id INTEGER PRIMARY KEY, Time REAL, Polarity TEXT, ScanMode INTEGER, MsMsType INTEGER, TimsId INTEGER, SummedIntensities INTEGER, MaxIntensity INTEGER, NumScans INTEGER, NumPeaks INTEGER, MzCalibration INTEGER, T1 REAL, T2 REAL, AccumulationTime REAL)", frames),
        ("Precursors", "CREATE TABLE Precursors (Id INTEGER PRIMARY KEY, MonoisotopicMz REAL, LargestPeakMz REAL, Charge INTEGER, Intensity REAL)", prec),
        ("PasefFrameMsMsInfo", "CREATE TABLE PasefFrameMsMsInfo (Frame INTEGER, Precursor INTEGER, ScanNumBegin INTEGER, ScanNumEnd INTEGER, IsolationMz REAL, IsolationWidth REAL, CollisionEnergy REAL)", pasef),
        ("MzCalibration", "CREATE TABLE MzCalibration (Id INTEGER PRIMARY KEY, ModelType INTEGER, DigitizerTimebase REAL, DigitizerDelay REAL, C0 REAL, C1 REAL, C2 REAL, C3 REAL, C4 REAL, T1 REAL, T2 REAL, dC1 REAL, dC2 REAL)", cal),
    ])
}

/// `analysis.tdf` and `analysis.tdf_bin` of `cicli` cycles of one MS1 frame (`n1` points) and 10 PASEF frames (`n2` points).
pub fn folder(cycles: usize, n1: usize, n2: usize) -> (Vec<u8>, Vec<u8>) {
    let mut c = Case(0x9E37_79B9_7F4A_7C15);
    let mut bin = Vec::new();
    let (mut frames, mut prec, mut pasef) = (Vec::new(), Vec::new(), Vec::new());
    let mut id = 0i64;
    for cycle in 0..cycles {
        for j in 0..=PASEF_PER_CYCLE {
            id += 1;
            let ms1 = j == 0;
            let fin: Vec<(u32, u32)> = if ms1 {
                vec![(0, SCANS)]
            } else {
                (0..PRECURSORS_PER_FRAME)
                    .map(|p| {
                        let a = 20 + p as u32 * 70;
                        (a, a + 30)
                    })
                    .collect()
            };
            let (per_scan, np) = peaks(&mut c, if ms1 { n1 } else { n2 }, &fin);
            let summed: u64 = per_scan.iter().flatten().map(|x| u64::from(x.1)).sum();
            let pos = bin.len() as i64;
            bin.extend(block(&per_scan));
            frames.push(frame_row(id, ms1, pos, summed, np));
            if !ms1 {
                for (p, &(a, b)) in fin.iter().enumerate() {
                    let pid = (cycle * PASEF_PER_CYCLE * PRECURSORS_PER_FRAME
                        + (j - 1) * PRECURSORS_PER_FRAME
                        + p) as i64
                        + 1;
                    let mz = 300.0 + (pid % 900) as f64 * 1.37;
                    prec.push(vec![V::N, V::R(mz), V::R(mz), V::I(2), V::R(1e4)]);
                    pasef.push(vec![
                        V::I(id),
                        V::I(pid),
                        V::I(i64::from(a)),
                        V::I(i64::from(b)),
                        // the window is centred off the monoisotopic ion, as an instrument centres it on the isotope cluster
                        V::R(mz + ISO_OFFSET),
                        V::R(2.0),
                        V::R(30.0),
                    ]);
                }
            }
        }
    }
    (database(frames, prec, pasef), bin)
}
