//! `.mzidx` builder (format: docs/agenti/mzidx.md): the peaks of an mzML or `.raw` stream or of a `Lettore`, sorted by m/z, written to any `Output`.
//! The result is byte for byte what `mzlab/reader/mzidx.py::build` writes. The scan table (JSON rows), the source and run
//! descriptions and the per-scan level, polarity, RT and TIC come from the caller (`ScanTable`): in the browser from the
//! Python reader, which has already read the headers; this crate only decodes the peaks.
//! Memory is bounded by `budget` bytes of peaks: if all of them fit they are kept from the first pass; otherwise a first pass
//! counts the peaks of every 1-unit m/z bucket and the buckets are written in rounds, each one decoding the stream again.

use std::cmp::Ordering;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::error::{Error, Result};
use crate::mzml::{self, Item, Spectrum};

pub const BLOCK: u64 = 16384;
const ALIGN: u64 = 64;
const HEADER: usize = 64;
const NB: usize = 10000;
/// Default bytes of peaks held in memory (the WASM memory is 256 MB at most).
pub const DEFAULT_BUDGET: usize = 64 << 20;

/// Where the file goes: writes at absolute offsets (gaps read as zeros). Files, `Cursor<Vec<u8>>` and the browser's callback implement it.
pub trait Output {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()>;
}

fn io_error(e: std::io::Error) -> Error {
    Error::new("err.file.index").with("detail", e)
}

impl<W: Write + Seek> Output for W {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        self.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        self.write_all(data).map_err(io_error)
    }
}

/// What the caller knows about the scans before the peaks are read.
#[derive(Debug, Clone, Default)]
pub struct ScanTable {
    pub level: Vec<u8>,
    /// +1, -1 or 0.
    pub polarity: Vec<i8>,
    pub rt: Vec<f64>,
    pub tic: Vec<f64>,
    /// The rows of `SCAN_COLUMNS` as compact JSON, written as they are.
    pub scans_json: Vec<u8>,
    /// JSON objects for `source` and `run` of the metadata, written as they are.
    pub source_json: String,
    pub run_json: String,
    /// MS1 peaks as `f32` (high-resolution file).
    pub hr1: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub scans: usize,
    pub peaks: u64,
    pub sections: usize,
    /// Times the stream was decoded.
    pub passes: usize,
    pub bytes: u64,
}

trait Num: Copy + Default {
    const SIZE: usize;
    fn from64(x: f64) -> Self;
    fn to64(self) -> f64;
    fn put(self, out: &mut Vec<u8>);
}
impl Num for f32 {
    const SIZE: usize = 4;
    fn from64(x: f64) -> Self {
        x as f32
    }
    fn to64(self) -> f64 {
        self as f64
    }
    fn put(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
impl Num for f64 {
    const SIZE: usize = 8;
    fn from64(x: f64) -> Self {
        x
    }
    fn to64(self) -> f64 {
        self
    }
    fn put(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}

#[derive(Clone, Copy, Default)]
struct Peak<T> {
    mz: T,
    it: T,
    pos: u32,
}

/// Ascending, NaN last (as `np.argsort`).
fn by_mz<T: Num>(a: &Peak<T>, b: &Peak<T>) -> Ordering {
    let (x, y) = (a.mz.to64(), b.mz.to64());
    match (x.is_nan(), y.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        _ => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
    }
}

fn bucket(x: f64) -> usize {
    if x.is_nan() {
        NB - 1
    } else {
        x.floor().clamp(0.0, (NB - 1) as f64) as usize
    }
}

fn reserve<T>(v: &mut Vec<T>, extra: usize) -> Result<()> {
    v.try_reserve(extra).map_err(|_| Error::memory_budget())
}

/// The sums and base peaks of a section, taken from its peaks in table order.
struct Totals {
    acc: Vec<f64>,
    bpm: Vec<f64>,
    bpy: Vec<f64>,
    have: Vec<bool>,
}

impl Totals {
    fn new(n: usize) -> Self {
        Totals {
            acc: vec![0.0; n],
            bpm: vec![0.0; n],
            bpy: vec![0.0; n],
            have: vec![false; n],
        }
    }
    fn add(&mut self, pos: usize, mz: f64, it: f64) {
        self.acc[pos] += it;
        if !self.have[pos] || it >= self.bpy[pos] {
            self.bpm[pos] = mz;
            self.bpy[pos] = it;
            self.have[pos] = true;
        }
    }
}

trait Sect {
    /// Pass 1: count the peaks of one scan of this section (and keep them when `keep`).
    fn count(&mut self, sp: &Spectrum, pos: u32, keep: bool) -> Result<()>;
    fn forget(&mut self);
    fn n_peaks(&self) -> u64;
    fn held(&self) -> usize;
    /// Bytes per peak while a round holds it.
    fn cost(&self, b: usize) -> usize;
    fn begin_round(&mut self, lo: usize, hi: usize) -> Result<()>;
    fn fill(&mut self, sp: &Spectrum, pos: u32) -> Result<()>;
    /// Sorts what the section holds and writes it (retained peaks, or the round `[lo, hi)`).
    fn flush(&mut self, out: &mut dyn Output, off: &Offsets, retained: bool) -> Result<()>;
    fn finish(&mut self, out: &mut dyn Output, off: &Offsets) -> Result<()>;
}

#[derive(Clone, Default)]
struct Offsets {
    rt: u64,
    tic: u64,
    scan_ids: u64,
    sumint: u64,
    bp_mz: u64,
    bp_inten: u64,
    fence: u64,
    mz: u64,
    inten: u64,
    pos: u64,
}

struct Sec<T: Num> {
    counts: Vec<u32>,
    before: Vec<u64>,
    n: u64,
    kept: Vec<Peak<T>>,
    // round state
    lo: usize,
    hi: usize,
    round: Vec<Peak<T>>,
    cursor: Vec<usize>,
    base: Vec<usize>,
    done: u64,
    totals: Totals,
    fence: Vec<T>,
    bytes: Vec<u8>,
}

impl<T: Num> Sec<T> {
    fn new(n_scans: usize) -> Self {
        Sec {
            counts: vec![0; NB],
            before: Vec::new(),
            n: 0,
            kept: Vec::new(),
            lo: 0,
            hi: 0,
            round: Vec::new(),
            cursor: vec![0; NB],
            base: vec![0; NB],
            done: 0,
            totals: Totals::new(n_scans),
            fence: Vec::new(),
            bytes: Vec::new(),
        }
    }

    fn peak(sp: &Spectrum, j: usize, pos: u32) -> Peak<T> {
        Peak {
            mz: T::from64(sp.mz[j]),
            it: T::from64(sp.intensity_f64(j)),
            pos,
        }
    }

    /// Writes `peaks` (sorted, following the ones already written) and updates the sums, base peaks and fence values.
    fn emit(&mut self, out: &mut dyn Output, off: &Offsets, peaks: &[Peak<T>]) -> Result<()> {
        const CHUNK: usize = 1 << 16;
        let mut g = (BLOCK - self.done % BLOCK) % BLOCK;
        while (g as usize) < peaks.len() {
            self.fence.push(peaks[g as usize].mz);
            g += BLOCK;
        }
        for chunk in peaks.chunks(CHUNK) {
            let at = self.done;
            for (field, base, width) in [
                (0, off.mz, T::SIZE),
                (1, off.inten, T::SIZE),
                (2, off.pos, 4),
            ] {
                self.bytes.clear();
                reserve(&mut self.bytes, chunk.len() * width)?;
                for p in chunk {
                    match field {
                        0 => p.mz.put(&mut self.bytes),
                        1 => p.it.put(&mut self.bytes),
                        _ => self.bytes.extend_from_slice(&(p.pos as i32).to_le_bytes()),
                    }
                }
                out.write_at(base + at * width as u64, &self.bytes)?;
            }
            for p in chunk {
                self.totals.add(p.pos as usize, p.mz.to64(), p.it.to64());
            }
            self.done += chunk.len() as u64;
        }
        Ok(())
    }
}

impl<T: Num> Sect for Sec<T> {
    fn count(&mut self, sp: &Spectrum, pos: u32, keep: bool) -> Result<()> {
        let n = sp.mz.len();
        if keep {
            reserve(&mut self.kept, n)?;
        }
        for j in 0..n {
            let p = Self::peak(sp, j, pos);
            self.counts[bucket(p.mz.to64())] += 1;
            if keep {
                self.kept.push(p);
            }
        }
        self.n += n as u64;
        Ok(())
    }

    fn forget(&mut self) {
        self.kept = Vec::new();
    }

    fn n_peaks(&self) -> u64 {
        self.n
    }

    fn held(&self) -> usize {
        self.kept.len() * std::mem::size_of::<Peak<T>>()
    }

    fn cost(&self, b: usize) -> usize {
        self.counts[b] as usize * std::mem::size_of::<Peak<T>>()
    }

    fn begin_round(&mut self, lo: usize, hi: usize) -> Result<()> {
        if self.before.is_empty() {
            let mut acc = 0u64;
            self.before = (0..NB)
                .map(|b| {
                    let v = acc;
                    acc += self.counts[b] as u64;
                    v
                })
                .collect();
        }
        self.lo = lo;
        self.hi = hi;
        let mut at = 0usize;
        for b in lo..hi {
            self.base[b] = at;
            self.cursor[b] = at;
            at += self.counts[b] as usize;
        }
        self.round = Vec::new();
        reserve(&mut self.round, at)?;
        self.round.resize(at, Peak::default());
        self.done = self.before[lo];
        Ok(())
    }

    fn fill(&mut self, sp: &Spectrum, pos: u32) -> Result<()> {
        for j in 0..sp.mz.len() {
            let p = Self::peak(sp, j, pos);
            let b = bucket(p.mz.to64());
            if b >= self.lo && b < self.hi {
                self.round[self.cursor[b]] = p;
                self.cursor[b] += 1;
            }
        }
        Ok(())
    }

    fn flush(&mut self, out: &mut dyn Output, off: &Offsets, retained: bool) -> Result<()> {
        if retained {
            let mut all = std::mem::take(&mut self.kept);
            // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
            #[allow(clippy::disallowed_methods)]
            all.sort_by(by_mz);
            self.done = 0;
            let r = self.emit(out, off, &all);
            self.kept = all;
            return r;
        }
        let mut round = std::mem::take(&mut self.round);
        for b in self.lo..self.hi {
            let (a, z) = (self.base[b], self.base[b] + self.counts[b] as usize);
            // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
            #[allow(clippy::disallowed_methods)]
            round[a..z].sort_by(by_mz);
        }
        let r = self.emit(out, off, &round);
        self.round = Vec::new();
        r
    }

    fn finish(&mut self, out: &mut dyn Output, off: &Offsets) -> Result<()> {
        let mut b = Vec::new();
        for v in &self.fence {
            v.put(&mut b);
        }
        out.write_at(off.fence, &b)?;
        for (at, arr) in [
            (off.sumint, &self.totals.acc),
            (off.bp_mz, &self.totals.bpm),
            (off.bp_inten, &self.totals.bpy),
        ] {
            let mut bytes = Vec::with_capacity(arr.len() * 8);
            for v in arr.iter() {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            out.write_at(at, &bytes)?;
        }
        Ok(())
    }
}

struct Info {
    level: u8,
    ids: Vec<usize>,
    f32: bool,
    n_peaks: u64,
    off: Offsets,
}

fn pad(n: u64) -> u64 {
    (ALIGN - n % ALIGN) % ALIGN
}

fn f64_bytes(v: &[f64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A pass over the stream: calls the function for every spectrum in file order.
pub type Pass<'a> = dyn FnMut(&mut dyn FnMut(Spectrum)) -> Result<()> + 'a;

/// Builds the index. `pass` decodes the whole stream once per call (it must start from the beginning every time).
/// `nativo`: the rows locate the scans in a vendor file set (version 2, `idxread::FLAG_LETTORE`), otherwise in an mzML (version 1).
pub fn build(
    table: &ScanTable,
    pass: &mut Pass,
    out: &mut dyn Output,
    budget: usize,
    nativo: bool,
) -> Result<Stats> {
    let (version, flags) = if nativo {
        (2u32, crate::idxread::FLAG_LETTORE)
    } else {
        (1, 0)
    };
    let n_scans = table.level.len();
    if table.polarity.len() != n_scans || table.rt.len() != n_scans || table.tic.len() != n_scans {
        return Err(Error::new("err.file.index").with("detail", "scan table"));
    }
    // -- the groups of scans (level x polarity), those with the same members share a section
    let mut levels: Vec<u8> = table.level.clone();
    // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
    #[allow(clippy::disallowed_methods)]
    levels.sort_unstable();
    levels.dedup();
    let mut infos: Vec<Info> = Vec::new();
    let mut keys: Vec<(String, usize)> = Vec::new();
    for &level in &levels {
        for (pol, name) in [(None, "all"), (Some(1i8), "+"), (Some(-1i8), "-")] {
            let ids: Vec<usize> = (0..n_scans)
                .filter(|&i| {
                    table.level[i] == level
                        && pol.is_none_or(|p| table.polarity[i] == p || table.polarity[i] == 0)
                })
                .collect();
            let si = match infos.iter().position(|s| s.level == level && s.ids == ids) {
                Some(si) => si,
                None => {
                    infos.push(Info {
                        level,
                        ids,
                        f32: level == 1 && table.hr1,
                        n_peaks: 0,
                        off: Offsets::default(),
                    });
                    infos.len() - 1
                }
            };
            keys.push((format!("{level}/{name}"), si));
        }
    }
    let mut member: Vec<Vec<(usize, u32)>> = vec![Vec::new(); n_scans];
    for (si, s) in infos.iter().enumerate() {
        for (p, &i) in s.ids.iter().enumerate() {
            member[i].push((si, p as u32));
        }
    }
    let mut secs: Vec<Box<dyn Sect>> = infos
        .iter()
        .map(|s| -> Box<dyn Sect> {
            if s.f32 {
                Box::new(Sec::<f32>::new(s.ids.len()))
            } else {
                Box::new(Sec::<f64>::new(s.ids.len()))
            }
        })
        .collect();

    // -- pass 1: count (and keep, while it fits)
    let mut passes = 0usize;
    let mut keep = true;
    let mut seen = 0usize;
    let mut failure: Option<Error> = None;
    {
        let mut on = |sp: Spectrum| {
            if failure.is_some() {
                return;
            }
            if seen >= n_scans {
                failure =
                    Some(Error::new("err.file.index").with("detail", "more spectra than scans"));
                return;
            }
            for &(si, pos) in &member[seen] {
                if let Err(e) = secs[si].count(&sp, pos, keep) {
                    failure = Some(e);
                    return;
                }
            }
            seen += 1;
            if keep && secs.iter().map(|s| s.held()).sum::<usize>() > budget / 2 {
                keep = false;
                secs.iter_mut().for_each(|s| s.forget());
            }
        };
        pass(&mut on)?;
        passes += 1;
    }
    if let Some(e) = failure {
        return Err(e);
    }
    if seen != n_scans {
        return Err(
            Error::new("err.file.index").with("detail", format!("{seen} spectra, {n_scans} scans"))
        );
    }

    // -- layout (as the Python builder)
    let mut pos = HEADER as u64;
    for (s, sec) in infos.iter_mut().zip(&secs) {
        let n = s.ids.len() as u64;
        s.n_peaks = sec.n_peaks();
        let isz = if s.f32 { 4 } else { 8 };
        let nf = s.n_peaks.div_ceil(BLOCK);
        let mut at = |size: u64| {
            pos += pad(pos);
            let o = pos;
            pos += size;
            o
        };
        s.off = Offsets {
            rt: at(8 * n),
            tic: at(8 * n),
            scan_ids: at(8 * n),
            sumint: at(8 * n),
            bp_mz: at(8 * n),
            bp_inten: at(8 * n),
            fence: at(isz * nf),
            mz: at(isz * s.n_peaks),
            inten: at(isz * s.n_peaks),
            pos: at(4 * s.n_peaks),
        };
    }
    pos += pad(pos);

    // -- the arrays of the scans
    for s in &infos {
        let rt: Vec<f64> = s.ids.iter().map(|&i| table.rt[i]).collect();
        let tic: Vec<f64> = s.ids.iter().map(|&i| table.tic[i]).collect();
        let ids: Vec<u8> = s
            .ids
            .iter()
            .flat_map(|&i| (i as i64).to_le_bytes())
            .collect();
        out.write_at(s.off.rt, &f64_bytes(&rt))?;
        out.write_at(s.off.tic, &f64_bytes(&tic))?;
        out.write_at(s.off.scan_ids, &ids)?;
    }

    // -- the peaks, in rounds of buckets
    if keep {
        for (s, sec) in infos.iter().zip(secs.iter_mut()) {
            sec.flush(out, &s.off, true)?;
            sec.forget();
        }
    } else {
        let costs: Vec<usize> = (0..NB)
            .map(|b| secs.iter().map(|s| s.cost(b)).sum())
            .collect();
        let cost = |b: usize| costs[b];
        let mut lo = 0usize;
        while lo < NB {
            let (mut hi, mut used) = (lo, 0usize);
            while hi < NB && (used + cost(hi) <= budget || hi == lo && cost(hi) <= budget * 2) {
                used += cost(hi);
                hi += 1;
            }
            if hi == lo {
                return Err(Error::memory_budget());
            }
            if used > 0 {
                for sec in secs.iter_mut() {
                    sec.begin_round(lo, hi)?;
                }
                let mut seen = 0usize;
                let mut failure: Option<Error> = None;
                {
                    let mut on = |sp: Spectrum| {
                        if failure.is_some() || seen >= n_scans {
                            return;
                        }
                        for &(si, pos) in &member[seen] {
                            if let Err(e) = secs[si].fill(&sp, pos) {
                                failure = Some(e);
                                return;
                            }
                        }
                        seen += 1;
                    };
                    pass(&mut on)?;
                    passes += 1;
                }
                if let Some(e) = failure {
                    return Err(e);
                }
                for (s, sec) in infos.iter().zip(secs.iter_mut()) {
                    sec.flush(out, &s.off, false)?;
                }
            }
            lo = hi;
        }
    }
    for (s, sec) in infos.iter().zip(secs.iter_mut()) {
        sec.finish(out, &s.off)?;
    }

    // -- the tables, the metadata and last the header
    let scans_off = pos;
    out.write_at(scans_off, &table.scans_json)?;
    let mut meta =
        format!("{{\"format\":\"mzidx\",\"version\":{version},\"block\":16384,\"source\":");
    meta.push_str(&table.source_json);
    meta.push_str(",\"run\":");
    meta.push_str(&table.run_json);
    meta.push_str(",\"keys\":{");
    for (i, (k, si)) in keys.iter().enumerate() {
        if i > 0 {
            meta.push(',');
        }
        meta.push_str(&format!("\"{k}\":{si}"));
    }
    meta.push_str("},\"sections\":[");
    for (i, s) in infos.iter().enumerate() {
        if i > 0 {
            meta.push(',');
        }
        let o = &s.off;
        meta.push_str(&format!(
            "{{\"level\":{},\"dtype\":\"{}\",\"n_scans\":{},\"n_peaks\":{},\"n_fence\":{},\"off\":{{\"rt\":{},\"tic\":{},\"scan_ids\":{},\"sumint\":{},\"bp_mz\":{},\"bp_inten\":{},\"fence\":{},\"mz\":{},\"inten\":{},\"pos\":{}}}}}",
            s.level,
            if s.f32 { "<f4" } else { "<f8" },
            s.ids.len(),
            s.n_peaks,
            s.n_peaks.div_ceil(BLOCK),
            o.rt, o.tic, o.scan_ids, o.sumint, o.bp_mz, o.bp_inten, o.fence, o.mz, o.inten, o.pos
        ));
    }
    meta.push_str("]}");
    let meta_off = scans_off + table.scans_json.len() as u64;
    out.write_at(meta_off, meta.as_bytes())?;
    let total = meta_off + meta.len() as u64;
    let mut head = Vec::with_capacity(HEADER);
    head.extend_from_slice(b"MZIDX001");
    head.extend_from_slice(&version.to_le_bytes());
    head.extend_from_slice(&flags.to_le_bytes());
    for v in [
        meta_off,
        meta.len() as u64,
        scans_off,
        table.scans_json.len() as u64,
        total,
    ] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    head.resize(HEADER, 0);
    out.write_at(0, &head)?;
    Ok(Stats {
        scans: n_scans,
        peaks: infos.iter().map(|s| s.n_peaks).sum(),
        sections: infos.len(),
        passes,
        bytes: total,
    })
}

/// Builds from an mzML read through `Read + Seek` (the file is read again from its start for every pass).
pub fn build_mzml<R: std::io::Read + Seek>(
    table: &ScanTable,
    mut src: R,
    out: &mut dyn Output,
    budget: usize,
) -> Result<Stats> {
    build(
        table,
        &mut |on| {
            src.seek(SeekFrom::Start(0)).map_err(io_error)?;
            mzml::parse(&mut src, |item| {
                if let Item::Spectrum(s) = item {
                    on(s)
                }
            })
        },
        out,
        budget,
        false,
    )
}

/// The same for a Thermo `.raw` file.
#[cfg(feature = "thermo")]
pub fn build_raw<R: std::io::Read + Seek>(
    table: &ScanTable,
    mut src: R,
    out: &mut dyn Output,
    budget: usize,
) -> Result<Stats> {
    build(
        table,
        &mut |on| {
            src.seek(SeekFrom::Start(0)).map_err(io_error)?;
            crate::raw::parse(&mut src, |s| on(s))
        },
        out,
        budget,
        false,
    )
}

/// The same for sample `c` of a vendor file set (version 2): every pass reads the scans of `Lettore::indice(c)` in order (`mzidx_lettore`).
pub fn build_lettore(
    table: &ScanTable,
    l: &mut dyn crate::lettori::Lettore,
    c: usize,
    out: &mut dyn Output,
    budget: usize,
) -> Result<Stats> {
    let n = l.indice(c)?.len();
    build(
        table,
        &mut |on| (0..n).try_for_each(|k| crate::mzidx_lettore::spettro(l, c, k).map(&mut *on)),
        out,
        budget,
        true,
    )
}

/// Convenience for tests and small files: the whole index in memory.
pub fn build_mzml_to_vec(
    table: &ScanTable,
    mzml: &[u8],
    budget: usize,
) -> Result<(Vec<u8>, Stats)> {
    let mut out = Cursor::new(Vec::new());
    let st = build_mzml(table, Cursor::new(mzml), &mut out, budget)?;
    Ok((out.into_inner(), st))
}
