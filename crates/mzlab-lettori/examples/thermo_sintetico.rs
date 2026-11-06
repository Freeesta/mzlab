//! A synthetic Orbitrap `.raw` (format v66, invented numbers) of any size, for the speed benches of the Thermo reader: MS1 scans in profile with the
//! centroid list, resolutions and noise nodes (`FTMS + p`), each followed by `--ms2` dependent HCD scans with centroids only (`FTMS + c`), a scan trailer
//! of `--trailer` bytes per scan. Laid out the way `vendor/opentfraw` reads a Q Exactive file (uniform events of 288 bytes). Written as a stream (bounded memory).
//! `cargo run --profile cli -p mzlab-lettori --features thermo --example thermo_sintetico -- OUT.raw [--scans N] [--peaks P] [--bins B] [--ms2 K] [--ms2-peaks Q] [--trailer T] [--seed S]`

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};

const VERSION: u32 = 66;
const EVENT: usize = 288; // preamble 136 + body 152
const PREAMBLE: usize = 136;
const ENTRY: usize = 88;
/// m/z = B / f² (Orbitrap coefficients 5: A, B, C at 2..4).
const B_COEF: f64 = 4.0e12;
const LOW: f64 = 100.0;
const HIGH: f64 = 1500.0;
/// Frequency step of a profile bin (Hz): about 0.0005 m/z at m/z 200.
const DF: f64 = 0.177;

struct Opt {
    out: String,
    scans: u32,
    peaks: u32,
    bins: u32,
    ms2: u32,
    ms2_peaks: u32,
    trailer: u32,
    seed: u64,
}

fn args() -> Opt {
    let mut a = std::env::args().skip(1);
    let mut o = Opt {
        out: String::new(),
        scans: 2000,
        peaks: 2000,
        bins: 16,
        ms2: 0,
        ms2_peaks: 200,
        trailer: 600,
        seed: 1,
    };
    while let Some(x) = a.next() {
        let mut num = || {
            a.next()
                .and_then(|v| v.parse::<u64>().ok())
                .expect("number after the option")
        };
        match x.as_str() {
            "--scans" => o.scans = num() as u32,
            "--peaks" => o.peaks = num() as u32,
            "--bins" => o.bins = num() as u32,
            "--ms2" => o.ms2 = num() as u32,
            "--ms2-peaks" => o.ms2_peaks = num() as u32,
            "--trailer" => o.trailer = num() as u32,
            "--seed" => o.seed = num(),
            _ if !x.starts_with("--") => o.out = x,
            _ => panic!("unknown option {x}"),
        }
    }
    assert!(
        !o.out.is_empty(),
        "usage: thermo_sintetico OUT.raw [--scans N] ..."
    );
    o
}

/// SplitMix64: the same file for the same options on every machine.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
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
struct Scan {
    bytes: Vec<u8>,
    tic: f64,
    base_int: f64,
    base_mz: f64,
    ms2: bool,
    precursor: f64,
}

fn put_u32(b: &mut Vec<u8>, x: u32) {
    b.extend(x.to_le_bytes());
}
fn put_f32(b: &mut Vec<u8>, x: f32) {
    b.extend(x.to_le_bytes());
}
fn put_f64(b: &mut Vec<u8>, x: f64) {
    b.extend(x.to_le_bytes());
}
fn put_u64(b: &mut Vec<u8>, x: u64) {
    b.extend(x.to_le_bytes());
}

/// An MS1 packet: profile chunks of `bins` bins around each peak, the centroid list (f32 m/z, f32 intensity), one resolution per peak, four noise nodes.
fn ms1(o: &Opt, rng: &mut Rng) -> Scan {
    let nbins_total = ((f_first() - freq(HIGH)) / DF) as u32;
    let mut apex: Vec<u32> = (0..o.peaks)
        .map(|_| (rng.unit() * f64::from(nbins_total - o.bins - 1)) as u32)
        .collect();
    apex.sort_unstable();
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
    put_u32(&mut b, n);
    let (mut tic, mut bi, mut bm) = (0f64, 0f64, 0f64);
    for &(m, i) in &cents {
        put_f32(&mut b, m);
        put_f32(&mut b, i);
        tic += f64::from(i);
        if f64::from(i) > bi {
            bi = f64::from(i);
            bm = f64::from(m);
        }
    }
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
    }
}

/// A dependent MS2 packet with centroids only.
fn ms2(o: &Opt, rng: &mut Rng, precursor: f64) -> Scan {
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
    put_u32(&mut b, n);
    let (mut tic, mut bi, mut bm) = (0f64, 0f64, 0f64);
    for &(m, i) in &cents {
        put_f32(&mut b, m);
        put_f32(&mut b, i);
        tic += f64::from(i);
        if f64::from(i) > bi {
            bi = f64::from(i);
            bm = f64::from(m);
        }
    }
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
    }
}

fn utf16(s: &str, bytes: usize) -> Vec<u8> {
    let mut b: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
    b.resize(bytes, 0);
    b
}
fn pascal(s: &str) -> Vec<u8> {
    let u: Vec<u16> = s.encode_utf16().collect();
    let mut b = (u.len() as u32).to_le_bytes().to_vec();
    b.extend(u.iter().flat_map(|x| x.to_le_bytes()));
    b
}

fn header() -> Vec<u8> {
    let mut b = 0xa101u16.to_le_bytes().to_vec();
    b.extend(utf16("Finnigan", 18));
    b.extend([0u8; 16]);
    put_u32(&mut b, VERSION);
    b.extend(vec![0u8; 112 * 2 + 4 + 60]);
    b.extend(utf16("synthetic", 1028));
    assert_eq!(b.len(), 1356);
    b
}

/// SeqRow, ASInfo and RawFileInfo of a v66 file with one controller.
fn middle(data_addr: u64, rh_addr: u64) -> Vec<u8> {
    let mut b = Vec::new();
    for x in [0, 1, 0] {
        put_u32(&mut b, x);
    }
    b.extend(utf16("1", 12));
    b.extend([0u8; 40]);
    b.extend(vec![0u8; (2 + 2 + 5 + 5 + 2 + 15) * 4 + 4]);
    b.extend([0u8; 24 + 4]);
    put_u32(&mut b, 0);
    for x in [2026u16, 1, 3, 1, 10, 0, 0, 0] {
        b.extend(x.to_le_bytes());
    }
    for x in [0, 0, 1, 0, 0, 0, 0] {
        put_u32(&mut b, x);
    }
    b.extend(vec![0u8; 760]);
    let mut region = Vec::new();
    put_u64(&mut region, data_addr);
    region.extend([0u8; 8]);
    put_u64(&mut region, rh_addr);
    region.extend([0u8; 8]);
    region.resize(1048, 0);
    b.extend(region);
    b.extend(vec![0u8; 6 * 4]);
    b
}

struct Addr {
    index: u64,
    data: u64,
    inst_log: u64,
    error_log: u64,
    trailer: u64,
    params: u64,
    own: u64,
}

fn run_header(n: u32, end_time: f64, a: &Addr) -> Vec<u8> {
    let mut b = Vec::new();
    for x in [0, 0, 1, n, 0, 0, 0, 0, 0, 0, 0, 0] {
        put_u32(&mut b, x);
    }
    for x in [1e9, LOW, HIGH, 0.0, end_time] {
        put_f64(&mut b, x);
    }
    b.extend(vec![0u8; 56 + 88 + 40 + 320]);
    assert_eq!(b.len(), 592);
    for i in 0..13 {
        if i == 6 {
            b.extend([0u8; 16]);
        }
        b.extend(vec![0u8; 520]);
    }
    for x in [0, 0, n, 0, 1, 0, 0, 0, 0, 0] {
        put_u32(&mut b, x);
    }
    for x in [
        a.index,
        a.data,
        a.inst_log,
        a.error_log,
        0,
        a.trailer,
        a.params,
    ] {
        put_u64(&mut b, x);
    }
    b.extend([0u8; 8]);
    put_u64(&mut b, a.own);
    b.extend(vec![0u8; 96]);
    for s in [
        "Orbitrap Exploris 480",
        "Orbitrap Exploris 480",
        "SN00000",
        "4.0",
        "1.0",
    ] {
        b.extend(pascal(s));
    }
    b
}

/// The trailer schema: the fields the reader names, then Float32 fillers up to `size` bytes per record.
fn trailer_fields(size: u32) -> Vec<(u32, &'static str, String)> {
    let mut f: Vec<(u32, &'static str, String)> = vec![
        (0x8, "", "Charge State:".into()),
        (0xB, "", "Monoisotopic M/Z:".into()),
        (0xA, "", "Ion Injection Time (ms):".into()),
        (0x8, "", "Master Scan Number:".into()),
        (0xA, "", "MS2 Isolation Width:".into()),
        (0x8, "", "FT Resolution:".into()),
        (0xA, "", "Elapsed Scan Time (sec):".into()),
    ];
    let used: u32 = 4 + 8 + 4 + 4 + 4 + 4 + 4;
    for k in 0..size.saturating_sub(used) / 4 {
        f.push((0xA, "", format!("Filler {k}:")));
    }
    f
}

/// What the index and the trailer need of a written packet.
struct Row {
    offset: u64,
    len: u32,
    rt: f64,
    tic: f64,
    base_int: f64,
    base_mz: f64,
    ms2: bool,
    precursor: f64,
}

fn main() -> std::io::Result<()> {
    let o = args();
    let mut rng = Rng(o.seed);
    let head = header();
    let mid_len = middle(0, 0).len() as u64;
    let data_addr = head.len() as u64 + mid_len;
    let mut w = BufWriter::with_capacity(1 << 20, File::create(&o.out)?);
    w.write_all(&head)?;
    w.write_all(&middle(0, 0))?;
    // packets, then what the index and the trailer need
    let mut entries: Vec<Row> = Vec::with_capacity(o.scans as usize);
    let mut offset = 0u64;
    let mut k = 0u32;
    let mut last_base = 500.0;
    while k < o.scans {
        let s = if k.is_multiple_of(o.ms2 + 1) {
            let s = ms1(&o, &mut rng);
            last_base = s.base_mz.max(LOW + 50.0);
            s
        } else {
            let p = LOW + 50.0 + (last_base - LOW - 50.0) * rng.unit();
            ms2(&o, &mut rng, p.max(LOW + 50.0))
        };
        let t = 0.002 * f64::from(k);
        w.write_all(&s.bytes)?;
        entries.push(Row {
            offset,
            len: s.bytes.len() as u32,
            rt: t,
            tic: s.tic,
            base_int: s.base_int,
            base_mz: s.base_mz,
            ms2: s.ms2,
            precursor: s.precursor,
        });
        offset += s.bytes.len() as u64;
        k += 1;
    }
    let inst_log = data_addr + offset;
    let error_log = inst_log + 4;
    let fields = trailer_fields(o.trailer);
    let mut gdh = Vec::new();
    put_u32(&mut gdh, fields.len() as u32);
    for (t, _, label) in &fields {
        put_u32(&mut gdh, *t);
        put_u32(&mut gdh, 0);
        gdh.extend(pascal(label));
    }
    let rec_size: u32 = 4 + 8 + 4 + 4 + 4 + 4 + 4 + 4 * (fields.len() as u32 - 7);
    let index_addr = error_log + gdh.len() as u64;
    let rh_addr = index_addr + (ENTRY as u64) * u64::from(o.scans);
    let end_time = entries.last().map_or(0.0, |e| e.rt);
    let rh_len = run_header(
        o.scans,
        end_time,
        &Addr {
            index: 0,
            data: 0,
            inst_log: 0,
            error_log: 0,
            trailer: 0,
            params: 0,
            own: 0,
        },
    )
    .len() as u64;
    let trailer = rh_addr + rh_len;
    let params = trailer + 4 + (EVENT as u64) * u64::from(o.scans);
    let a = Addr {
        index: index_addr,
        data: data_addr,
        inst_log,
        error_log,
        trailer,
        params,
        own: rh_addr,
    };
    w.write_all(&0u32.to_le_bytes())?;
    w.write_all(&gdh)?;
    for (i, e) in entries.iter().enumerate() {
        let mut b = Vec::with_capacity(ENTRY);
        put_u32(&mut b, e.offset as u32);
        put_u32(&mut b, i as u32);
        b.extend(((i as u32 % (o.ms2 + 1)) as u16 + 1).to_le_bytes()); // event number in the cycle (1 = MS1)
        b.extend(0u16.to_le_bytes());
        put_u32(&mut b, i as u32 + 1);
        put_u32(&mut b, 0);
        put_u32(&mut b, e.len);
        for x in [
            e.rt,
            e.tic,
            e.base_int,
            e.base_mz,
            LOW,
            if e.ms2 { e.precursor + 10.0 } else { HIGH },
        ] {
            put_f64(&mut b, x);
        }
        put_u64(&mut b, e.offset);
        b.extend([0u8; 8]);
        w.write_all(&b)?;
    }
    w.write_all(&run_header(o.scans, end_time, &a))?;
    w.write_all(&0u32.to_le_bytes())?;
    for e in &entries {
        let mut p = vec![0u8; PREAMBLE];
        p[4] = 1; // positive
        p[5] = u8::from(!e.ms2); // profile for MS1, centroid for MS2
        p[6] = if e.ms2 { 2 } else { 1 };
        p[10] = u8::from(e.ms2);
        p[11] = 3; // ESI
        p[24] = if e.ms2 { 4 } else { 0 };
        p[40] = 4; // FTMS
        let mut body = vec![0u8; EVENT - PREAMBLE];
        if e.ms2 {
            body[4..8].copy_from_slice(&1u32.to_le_bytes());
            body[8..16].copy_from_slice(&e.precursor.to_le_bytes());
            body[16..24].copy_from_slice(&1.0f64.to_le_bytes());
            body[24..32].copy_from_slice(&30.0f64.to_le_bytes());
        }
        let hi = if e.ms2 { e.precursor + 10.0 } else { HIGH };
        body[64..72].copy_from_slice(&LOW.to_le_bytes());
        body[72..80].copy_from_slice(&hi.to_le_bytes());
        body[80..84].copy_from_slice(&5u32.to_le_bytes());
        for (j, c) in [0.0, 0.0, 0.0, B_COEF, 0.0].iter().enumerate() {
            body[84 + 8 * j..92 + 8 * j].copy_from_slice(&c.to_le_bytes());
        }
        w.write_all(&p)?;
        w.write_all(&body)?;
    }
    let mut master = 1u32;
    for (i, e) in entries.iter().enumerate() {
        if !e.ms2 {
            master = i as u32 + 1;
        }
        let mut b = Vec::with_capacity(rec_size as usize);
        put_u32(&mut b, if e.ms2 { 2 } else { 0 });
        put_f64(&mut b, if e.ms2 { e.precursor } else { 0.0 });
        put_f32(&mut b, if e.ms2 { 22.0 } else { 3.5 });
        put_u32(&mut b, if e.ms2 { master } else { 0 });
        put_f32(&mut b, if e.ms2 { 1.4 } else { 0.0 });
        put_u32(&mut b, if e.ms2 { 30_000 } else { 120_000 });
        put_f32(&mut b, 0.25);
        b.resize(rec_size as usize, 0);
        w.write_all(&b)?;
    }
    let mut f = w.into_inner().map_err(|e| e.into_error())?;
    f.seek(SeekFrom::Start(head.len() as u64))?;
    f.write_all(&middle(data_addr, rh_addr))?;
    let len = f.seek(SeekFrom::End(0))?;
    println!(
        "{{\"out\":{:?},\"bytes\":{len},\"scans\":{},\"data_bytes\":{offset}}}",
        o.out, o.scans
    );
    Ok(())
}
