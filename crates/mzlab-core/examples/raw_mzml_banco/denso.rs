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
pub const PASSO: f64 = -0.25;
/// Layout word: m/z of the centroid list as f64 (bit 16) and a fudge per profile chunk (low byte).
const LAYOUT: u32 = 0x10001;
pub const NODI: usize = 40;

/// What the case is made of.
#[derive(Debug, Clone, Copy)]
pub struct Parametri {
    pub scansioni: usize,
    /// Centroid peaks per scan.
    pub picchi: usize,
    pub seme: u64,
    /// The packet carries the profile (chunks of `bin.0..=bin.1` bins around each peak); without it the packet is a centroid list only.
    pub con_profilo: bool,
    pub bin: (u32, u32),
}

impl Parametri {
    /// At least 200 000 profile points per scan (20 500 peaks of 8-12 bins).
    pub fn denso(scansioni: usize) -> Self {
        Parametri {
            scansioni,
            picchi: 20_500,
            seme: 0x6d7a_4c61_6221_0001,
            con_profilo: true,
            bin: (8, 12),
        }
    }

    /// The shape of a typical `Q Exactive` run at 17 500 resolution: centroid list only (about 530 peaks per scan).
    pub fn forma_lab_a(scansioni: usize) -> Self {
        Parametri {
            scansioni,
            picchi: 526,
            seme: 11,
            con_profilo: false,
            bin: (8, 12),
        }
    }

    /// The shape of a Fusion run with profile: about 64 peaks of 4-8 bins per scan.
    pub fn forma_lab_b(scansioni: usize) -> Self {
        Parametri {
            scansioni,
            picchi: 64,
            seme: 13,
            con_profilo: true,
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
fn punti(
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
        let rumore = (rng.next() % 1000) as f64 * 0.5;
        let s = (amp / (q * q) + rumore) as f32;
        profilo.extend_from_slice(&s.to_le_bytes());
        if let Some(v) = verita.as_deref_mut() {
            v.profilo.push((bin + j, s, fudge));
        }
    }
}

/// The noise nodes (m/z, noise, baseline) spread over the scan range.
fn nodi_di_rumore(rng: &mut Rng, primo_mz: f64, ultimo_mz: f64) -> Vec<u8> {
    let mut nodi = Vec::with_capacity(NODI * 12);
    for i in 0..NODI {
        let mz = primo_mz + (ultimo_mz - primo_mz) * i as f64 / (NODI - 1) as f64;
        nodi.extend_from_slice(&(mz as f32).to_le_bytes());
        nodi.extend_from_slice(&(1.0 + (rng.next() % 100) as f32 * 0.1).to_le_bytes());
        nodi.extend_from_slice(&(100.0 + (rng.next() % 100) as f32).to_le_bytes());
    }
    nodi
}

/// The 40-byte header of the packet (sizes in words, layout, low and high m/z).
fn intestazione(
    out: &mut Vec<u8>,
    [profilo, lista]: &[&Vec<u8>; 2],
    n: usize,
    (primo_mz, ultimo_mz): (f64, f64),
) {
    let parole = |b: &Vec<u8>| (b.len() / 4) as u32;
    u32s(
        out,
        &[
            0,
            parole(profilo),
            parole(lista),
            LAYOUT,
            n as u32,
            n as u32 + 1,
            (NODI * 3) as u32,
            0,
        ],
    );
    out.extend_from_slice(&(primo_mz as f32).to_le_bytes());
    out.extend_from_slice(&(ultimo_mz as f32).to_le_bytes());
}

/// Appends packet `k` of the case to `out`; `verita` collects the values written when a test wants them.
pub fn pacchetto(out: &mut Vec<u8>, p: &Parametri, k: usize, mut verita: Option<&mut Verita>) {
    let mut rng = Rng(p.seme ^ (k as u64).wrapping_mul(0xd6e8_feb8_6659_fd93));
    let n = p.picchi;
    let mut profilo = Vec::with_capacity(if p.con_profilo { n * 44 + 24 } else { 0 });
    profilo.extend_from_slice(&F0.to_le_bytes());
    profilo.extend_from_slice(&PASSO.to_le_bytes());
    u32s(&mut profilo, &[n as u32, 0]);
    let mut lista = Vec::with_capacity(4 + n * 12);
    u32s(&mut lista, &[n as u32]);
    let mut bin = 1 + (rng.next() % 40) as u32;
    let (mut descr, mut risol) = (Vec::with_capacity(n * 4), Vec::with_capacity(n * 4 + 4));
    u32s(&mut risol, &[n as u32]);
    let mut totale_bin = 0u32;
    let (mut primo_mz, mut ultimo_mz) = (0.0f64, 0.0f64);
    for i in 0..n {
        let len = p.bin.0 + (rng.next() % u64::from(p.bin.1 - p.bin.0 + 1)) as u32;
        let amp = 1.0e3 + (rng.next() >> 40) as f64 * 0.01;
        let fudge = ((rng.next() >> 50) as f32) * 1.0e-8;
        let c = (len - 1) as f64 * 0.5;
        if p.con_profilo {
            profilo.extend_from_slice(&bin.to_le_bytes());
            profilo.extend_from_slice(&len.to_le_bytes());
            profilo.extend_from_slice(&fudge.to_le_bytes());
            punti(
                &mut profilo,
                &mut rng,
                (bin, len, amp, fudge),
                verita.as_deref_mut(),
            );
        }
        let centro = bin as f64 + c;
        let mz = freq_to_mz(F0 + centro * PASSO, &COEFF) + f64::from(fudge);
        if i == 0 {
            primo_mz = mz;
        }
        ultimo_mz = mz;
        let abb = amp as f32;
        lista.extend_from_slice(&mz.to_le_bytes());
        lista.extend_from_slice(&abb.to_le_bytes());
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
        totale_bin = bin;
    }
    if p.con_profilo {
        profilo[20..24].copy_from_slice(&totale_bin.to_le_bytes());
    } else {
        profilo.clear();
    }
    let nodi = nodi_di_rumore(&mut rng, primo_mz, ultimo_mz);
    intestazione(out, &[&profilo, &lista], n, (primo_mz, ultimo_mz));
    out.extend_from_slice(&profilo);
    out.extend_from_slice(&lista);
    out.extend_from_slice(&descr);
    out.extend_from_slice(&risol);
    out.extend_from_slice(&nodi);
}

/// The offset table (offset, length), the bytes written and the SHA-256 of the file when asked.
pub type Generato = (Vec<(u64, u32)>, u64, Option<String>);

/// Writes the case to `path`: a table of (offset, length) of the packets, then the packets. Returns the table, the bytes and the SHA-256 of the file
/// (`con_sha`: it costs a pass of the file).
pub fn genera(path: &Path, p: &Parametri, con_sha: bool) -> std::io::Result<Generato> {
    let mut f = BufWriter::with_capacity(4 << 20, File::create(path)?);
    let mut sha = Sha256::new();
    let mut scrivi = |f: &mut BufWriter<File>, b: &[u8]| -> std::io::Result<()> {
        if con_sha {
            sha.update(b);
        }
        f.write_all(b)
    };
    let testa = 8 + 12 * p.scansioni as u64;
    let mut tabella = Vec::with_capacity(p.scansioni);
    let mut buf = Vec::new();
    let mut pos = testa;
    let mut corpo = Vec::new();
    // the table needs the lengths, so it is written after the packets and the file is assembled in two passes over the same generator
    for k in 0..p.scansioni {
        buf.clear();
        pacchetto(&mut buf, p, k, None);
        tabella.push((pos, buf.len() as u32));
        pos += buf.len() as u64;
    }
    corpo.extend_from_slice(&(p.scansioni as u64).to_le_bytes());
    for (o, l) in &tabella {
        corpo.extend_from_slice(&o.to_le_bytes());
        corpo.extend_from_slice(&l.to_le_bytes());
    }
    scrivi(&mut f, &corpo)?;
    for k in 0..p.scansioni {
        buf.clear();
        pacchetto(&mut buf, p, k, None);
        scrivi(&mut f, &buf)?;
    }
    f.flush()?;
    Ok((tabella, pos, con_sha.then(|| hex(&sha.finalize()))))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
