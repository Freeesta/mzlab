//! The dense synthetic case of the `.raw` → mzML bench: deterministic Thermo FT scan data packets (profile in chunks with fudge, centroid list,
//! descriptors, resolutions, noise nodes, in the layout `opentfraw::scan_data` reads) written to one file, and a `Lettore` that serves them back from
//! offsets with the decoders of the real reader (`opentfraw::pacchetto`). The `.raw` container is not rebuilt: its cost is measured on real files.
//! Same parameters, same bytes: only integer generator steps and IEEE `+ - * /` are used, no libm.
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use opentfraw::scan_data::freq_to_mz;
use sha2::{Digest, Sha256};

/// Orbitrap conversion `m/z = A + B / f² + C / f⁴` as the 5-value array of the scan event (two unknown prefix values, then A, B, C).
pub const COEFF: [f64; 5] = [0.0, 0.0, -0.25, 1.0e13, 3.0e6];
/// The frequency of the first bin (Hz) and the step: negative, so that the m/z grow with the bin number.
pub const F0: f64 = 258_000.0;
pub const STEP: f64 = -0.25;
/// Layout word: m/z of the centroid list as f64 (bit 16) and a fudge per profile chunk (low byte).
const LAYOUT: u32 = 0x10001;
pub const NODES: usize = 40;

/// What the case is made of.
#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    pub scansioni: usize,
    /// Centroid peaks per scan.
    pub peaks: usize,
    pub seme: u64,
    /// The packet carries the profile (chunks of `bin.0..=bin.1` bins around each peak); without it the packet is a centroid list only.
    pub with_profile: bool,
    pub bin: (u32, u32),
}

impl Parameters {
    /// At least 200 000 profile points per scan (20 500 peaks of 8-12 bins).
    pub fn denso(scansioni: usize) -> Self {
        Parameters {
            scansioni,
            peaks: 20_500,
            seme: 0x6d7a_4c61_6221_0001,
            with_profile: true,
            bin: (8, 12),
        }
    }

    /// The shape of a typical `Q Exactive` run at 17 500 resolution: centroid list only (about 530 peaks per scan).
    pub fn lab_shape_a(scansioni: usize) -> Self {
        Parameters {
            scansioni,
            peaks: 526,
            seme: 11,
            with_profile: false,
            bin: (8, 12),
        }
    }

    /// The shape of a Fusion run with profile: about 64 peaks of 4-8 bins per scan.
    pub fn lab_shape_b(scansioni: usize) -> Self {
        Parameters {
            scansioni,
            peaks: 64,
            seme: 13,
            with_profile: true,
            bin: (4, 8),
        }
    }
}

/// SplitMix64: a step is integer arithmetic only.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// What a packet holds, for a test to compare with what the decoders give back.
#[derive(Default)]
pub struct Verita {
    /// (frequency bin, intensity) of every profile point, and the fudge of its chunk.
    pub profilo: Vec<(u32, f32, f32)>,
    pub picchi: Vec<(f64, f32)>,
    pub risoluzioni: Vec<f32>,
}

fn u32s(v: &mut Vec<u8>, x: &[u32]) {
    x.iter().for_each(|x| v.extend_from_slice(&x.to_le_bytes()));
}

/// The signal of one profile chunk, a Lorentzian squared plus noise; `(first bin, length, amplitude, fudge)`.
fn points(
    profilo: &mut Vec<u8>,
    rng: &mut Rng,
    (bin, len, amp, fudge): (u32, u32, f64, f32),
    mut verita: Option<&mut Verita>,
) {
    let w = len as f64 * 0.25;
    let c = (len - 1) as f64 * 0.5;
    for j in 0..len {
        let x = (j as f64 - c) / w;
        let q = 1.0 + x * x;
        let noise = (rng.next() % 1000) as f64 * 0.5;
        let s = (amp / (q * q) + noise) as f32;
        profilo.extend_from_slice(&s.to_le_bytes());
        if let Some(v) = verita.as_deref_mut() {
            v.profilo.push((bin + j, s, fudge));
        }
    }
}

/// The noise nodes (m/z, noise, baseline) spread over the scan range.
fn noise_nodes(rng: &mut Rng, first_mz: f64, last_mz: f64) -> Vec<u8> {
    let mut nodes = Vec::with_capacity(NODES * 12);
    for i in 0..NODES {
        let mz = first_mz + (last_mz - first_mz) * i as f64 / (NODES - 1) as f64;
        nodes.extend_from_slice(&(mz as f32).to_le_bytes());
        nodes.extend_from_slice(&(1.0 + (rng.next() % 100) as f32 * 0.1).to_le_bytes());
        nodes.extend_from_slice(&(100.0 + (rng.next() % 100) as f32).to_le_bytes());
    }
    nodes
}

/// The 40-byte header of the packet (sizes in words, layout, low and high m/z).
fn scan_header(
    out: &mut Vec<u8>,
    [profilo, lista]: &[&Vec<u8>; 2],
    n: usize,
    (primo_mz, ultimo_mz): (f64, f64),
) {
    let words = |b: &Vec<u8>| (b.len() / 4) as u32;
    u32s(
        out,
        &[
            0,
            words(profilo),
            words(lista),
            LAYOUT,
            n as u32,
            n as u32 + 1,
            (NODES * 3) as u32,
            0,
        ],
    );
    out.extend_from_slice(&(primo_mz as f32).to_le_bytes());
    out.extend_from_slice(&(ultimo_mz as f32).to_le_bytes());
}

/// Appends packet `k` of the case to `out`; `verita` collects the values written when a test wants them.
pub fn packet(out: &mut Vec<u8>, p: &Parameters, k: usize, mut verita: Option<&mut Verita>) {
    let mut rng = Rng(p.seme ^ (k as u64).wrapping_mul(0xd6e8_feb8_6659_fd93));
    let n = p.peaks;
    let mut profilo = Vec::with_capacity(if p.with_profile { n * 44 + 24 } else { 0 });
    profilo.extend_from_slice(&F0.to_le_bytes());
    profilo.extend_from_slice(&STEP.to_le_bytes());
    u32s(&mut profilo, &[n as u32, 0]);
    let mut list = Vec::with_capacity(4 + n * 12);
    u32s(&mut list, &[n as u32]);
    let mut bin = 1 + (rng.next() % 40) as u32;
    let (mut descr, mut risol) = (Vec::with_capacity(n * 4), Vec::with_capacity(n * 4 + 4));
    u32s(&mut risol, &[n as u32]);
    let mut bin_total = 0u32;
    let (mut first_mz, mut last_mz) = (0.0f64, 0.0f64);
    for i in 0..n {
        let len = p.bin.0 + (rng.next() % u64::from(p.bin.1 - p.bin.0 + 1)) as u32;
        let amp = 1.0e3 + (rng.next() >> 40) as f64 * 0.01;
        let fudge = ((rng.next() >> 50) as f32) * 1.0e-8;
        let c = (len - 1) as f64 * 0.5;
        if p.with_profile {
            profilo.extend_from_slice(&bin.to_le_bytes());
            profilo.extend_from_slice(&len.to_le_bytes());
            profilo.extend_from_slice(&fudge.to_le_bytes());
            points(
                &mut profilo,
                &mut rng,
                (bin, len, amp, fudge),
                verita.as_deref_mut(),
            );
        }
        let centre = bin as f64 + c;
        let mz = freq_to_mz(F0 + centre * STEP, &COEFF) + f64::from(fudge);
        if i == 0 {
            first_mz = mz;
        }
        last_mz = mz;
        let abb = amp as f32;
        list.extend_from_slice(&mz.to_le_bytes());
        list.extend_from_slice(&abb.to_le_bytes());
        // the index in the low 16 bits, a charge and a flag byte above it
        u32s(
            &mut descr,
            &[(i as u32 & 0xffff)
                | (((rng.next() % 4) as u32) << 24)
                | (((rng.next() % 2) as u32) << 16)],
        );
        let r = 60_000.0 + (rng.next() % 40_000) as f32;
        risol.extend_from_slice(&r.to_le_bytes());
        if let Some(v) = verita.as_deref_mut() {
            v.picchi.push((mz, abb));
            v.risoluzioni.push(r);
        }
        bin += len + 1 + (rng.next() % 50) as u32;
        bin_total = bin;
    }
    if p.with_profile {
        profilo[20..24].copy_from_slice(&bin_total.to_le_bytes());
    } else {
        profilo.clear();
    }
    let nodes = noise_nodes(&mut rng, first_mz, last_mz);
    scan_header(out, &[&profilo, &list], n, (first_mz, last_mz));
    out.extend_from_slice(&profilo);
    out.extend_from_slice(&list);
    out.extend_from_slice(&descr);
    out.extend_from_slice(&risol);
    out.extend_from_slice(&nodes);
}

/// The offset table (offset, length), the bytes written and the SHA-256 of the file when asked.
pub type Generated = (Vec<(u64, u32)>, u64, Option<String>);

/// Writes the case to `path`: a table of (offset, length) of the packets, then the packets. Returns the table, the bytes and the SHA-256 of the file
/// (`with_sha`: it costs a pass of the file).
pub fn generate(path: &Path, p: &Parameters, with_sha: bool) -> std::io::Result<Generated> {
    let mut f = BufWriter::with_capacity(4 << 20, File::create(path)?);
    let mut sha = Sha256::new();
    let mut write = |f: &mut BufWriter<File>, b: &[u8]| -> std::io::Result<()> {
        if with_sha {
            sha.update(b);
        }
        f.write_all(b)
    };
    let testa = 8 + 12 * p.scansioni as u64;
    let mut table = Vec::with_capacity(p.scansioni);
    let mut buf = Vec::new();
    let mut pos = testa;
    let mut corpo = Vec::new();
    // the table needs the lengths, so it is written after the packets and the file is assembled in two passes over the same generator
    for k in 0..p.scansioni {
        buf.clear();
        packet(&mut buf, p, k, None);
        table.push((pos, buf.len() as u32));
        pos += buf.len() as u64;
    }
    corpo.extend_from_slice(&(p.scansioni as u64).to_le_bytes());
    for (o, l) in &table {
        corpo.extend_from_slice(&o.to_le_bytes());
        corpo.extend_from_slice(&l.to_le_bytes());
    }
    write(&mut f, &corpo)?;
    for k in 0..p.scansioni {
        buf.clear();
        packet(&mut buf, p, k, None);
        write(&mut f, &buf)?;
    }
    f.flush()?;
    Ok((table, pos, with_sha.then(|| hex(&sha.finalize()))))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
