//! The scan packets of the synthetic Orbitrap `.raw`: an MS1 in profile with its centroids, resolutions and noise nodes, a dependent MS2 in centroid.
use super::{Opt, B_COEF, DF, HIGH, LOW};

/// SplitMix64: the same file for the same options on every machine.
pub(crate) struct Rng(pub(crate) u64);
impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub(crate) fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn freq(mz: f64) -> f64 {
    (B_COEF / mz).sqrt()
}
fn f_first() -> f64 {
    freq(LOW)
}
fn bin_mz(bin: u32) -> f64 {
    let f = f_first() - f64::from(bin) * DF;
    B_COEF / (f * f)
}

/// What the index entry of a scan says, and the packet.
pub(crate) struct Scan {
    pub(crate) bytes: Vec<u8>,
    pub(crate) tic: f64,
    pub(crate) base_int: f64,
    pub(crate) base_mz: f64,
    pub(crate) ms2: bool,
    pub(crate) precursor: f64,
    /// The centroids of an MS1 (empty for an MS2): `--dda` picks its precursors among them.
    pub(crate) cents: Vec<(f32, f32)>,
}

pub(crate) fn put_u32(b: &mut Vec<u8>, x: u32) {
    b.extend(x.to_le_bytes());
}
pub(crate) fn put_f32(b: &mut Vec<u8>, x: f32) {
    b.extend(x.to_le_bytes());
}
pub(crate) fn put_f64(b: &mut Vec<u8>, x: f64) {
    b.extend(x.to_le_bytes());
}
pub(crate) fn put_u64(b: &mut Vec<u8>, x: u64) {
    b.extend(x.to_le_bytes());
}

/// The centroid list (count, then f32 m/z and intensity pairs); returns the TIC, the base peak intensity and its m/z.
fn put_cents(b: &mut Vec<u8>, cents: &[(f32, f32)]) -> (f64, f64, f64) {
    put_u32(b, cents.len() as u32);
    let (mut tic, mut bi, mut bm) = (0f64, 0f64, 0f64);
    for &(m, i) in cents {
        put_f32(b, m);
        put_f32(b, i);
        tic += f64::from(i);
        if f64::from(i) > bi {
            bi = f64::from(i);
            bm = f64::from(m);
        }
    }
    (tic, bi, bm)
}

/// One Gaussian chunk of `bins` bins per apex (overlapping ones skipped) and its centroid.
#[allow(clippy::type_complexity)]
fn peaks(o: &Opt, rng: &mut Rng, apex: Vec<u32>) -> (Vec<(u32, Vec<f32>)>, Vec<(f32, f32)>) {
    let mut chunks: Vec<(u32, Vec<f32>)> = Vec::new();
    let mut cents: Vec<(f32, f32)> = Vec::new();
    let mut next_free = 0u32;
    for a in apex {
        let first = a.saturating_sub(o.bins / 2);
        if first < next_free {
            continue;
        }
        let height = (10f64.powf(3.0 + 4.0 * rng.unit())) as f32;
        let mid = (o.bins as f32 - 1.0) / 2.0;
        let sig: Vec<f32> = (0..o.bins)
            .map(|i| {
                let d = (i as f32 - mid) / (o.bins as f32 / 6.0);
                height * (-0.5 * d * d).exp()
            })
            .collect();
        cents.push((bin_mz(first + o.bins / 2) as f32, height));
        chunks.push((first, sig));
        next_free = first + o.bins + 1;
    }
    (chunks, cents)
}

/// An MS1 packet: profile chunks of `bins` bins around each peak, the centroid list (f32 m/z, f32 intensity), one resolution per peak, four noise nodes.
pub(crate) fn ms1(o: &Opt, rng: &mut Rng) -> Scan {
    let nbins_total = ((f_first() - freq(HIGH)) / DF) as u32;
    let mut apex: Vec<u32> = (0..o.peaks)
        .map(|_| (rng.unit() * f64::from(nbins_total - o.bins - 1)) as u32)
        .collect();
    apex.sort_unstable();
    let (chunks, cents) = peaks(o, rng, apex);
    let mut b = Vec::new();
    let profile_words = 6 + chunks.iter().map(|c| 3 + c.1.len() as u32).sum::<u32>();
    let n = cents.len() as u32;
    put_u32(&mut b, 1); // windows
    put_u32(&mut b, profile_words);
    put_u32(&mut b, 1 + 2 * n); // peak list: count + pairs
    put_u32(&mut b, 1); // layout: one fudge per chunk
    put_u32(&mut b, 0); // descriptors
    put_u32(&mut b, n + 1); // resolutions
    put_u32(&mut b, 12); // four noise nodes
    put_u32(&mut b, 0);
    put_f32(&mut b, LOW as f32);
    put_f32(&mut b, HIGH as f32);
    put_f64(&mut b, f_first());
    put_f64(&mut b, -DF);
    put_u32(&mut b, chunks.len() as u32);
    put_u32(&mut b, nbins_total);
    for (first, sig) in &chunks {
        put_u32(&mut b, *first);
        put_u32(&mut b, sig.len() as u32);
        put_f32(&mut b, 0.0);
        for &s in sig {
            put_f32(&mut b, s);
        }
    }
    let (tic, bi, bm) = put_cents(&mut b, &cents);
    put_u32(&mut b, n);
    for &(m, _) in &cents {
        put_f32(&mut b, (120_000.0 * (200.0 / f64::from(m)).sqrt()) as f32);
    }
    for k in 0..4 {
        put_f32(&mut b, (LOW + (HIGH - LOW) * k as f64 / 3.0) as f32);
        put_f32(&mut b, 150.0 + 10.0 * k as f32);
        put_f32(&mut b, 20.0);
    }
    Scan {
        bytes: b,
        tic,
        base_int: bi,
        base_mz: bm,
        ms2: false,
        precursor: 0.0,
        cents,
    }
}

/// A dependent MS2 packet with centroids only.
pub(crate) fn ms2(o: &Opt, rng: &mut Rng, precursor: f64) -> Scan {
    let mut cents: Vec<(f32, f32)> = (0..o.ms2_peaks)
        .map(|_| {
            (
                (LOW + (precursor - LOW) * rng.unit()) as f32,
                (10f64.powf(2.0 + 4.0 * rng.unit())) as f32,
            )
        })
        .collect();
    cents.sort_by(|a, b| a.0.total_cmp(&b.0));
    let n = cents.len() as u32;
    let mut b = Vec::new();
    for x in [1, 0, 1 + 2 * n, 0, 0, n + 1, 12, 0] {
        put_u32(&mut b, x);
    }
    put_f32(&mut b, LOW as f32);
    put_f32(&mut b, precursor as f32 + 10.0);
    let (tic, bi, bm) = put_cents(&mut b, &cents);
    put_u32(&mut b, n);
    for _ in 0..n {
        put_f32(&mut b, 30_000.0);
    }
    for k in 0..4 {
        put_f32(&mut b, (LOW + (precursor - LOW) * k as f64 / 3.0) as f32);
        put_f32(&mut b, 50.0);
        put_f32(&mut b, 5.0);
    }
    Scan {
        bytes: b,
        tic,
        base_int: bi,
        base_mz: bm,
        ms2: true,
        precursor,
        cents: Vec::new(),
    }
}
