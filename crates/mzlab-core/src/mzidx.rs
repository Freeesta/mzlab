//! `.mzidx` builder (format: docs/agenti/mzidx.md): the peaks of an mzML or `.raw` stream or of a `Lettore`, sorted by m/z, written to any `Output`.
//! The result is byte for byte what `mzlab/reader/mzidx.py::build` writes. The scan table (JSON rows), the source and run
//! descriptions and the per-scan level, polarity, RT and TIC come from the caller (`ScanTable`): in the browser from the
//! Python reader, which has already read the headers; this crate only decodes the peaks.
//! Memory is bounded by `budget` bytes of peaks: if all of them fit they are kept from the first pass; otherwise a first pass
//! counts the peaks of every 1-unit m/z bucket and the buckets are written in rounds, each one decoding the stream again.

use std::cmp::Ordering;
use std::io::{Cursor, Seek, SeekFrom, Write};

use crate::error::{Error, Result};
use crate::mzidx_spill::{leggi_parti, Parti, Spill};
use crate::mzml::{self, Item, Spectrum};

pub const BLOCK: u64 = 16384;
const ALIGN: u64 = 64;
const HEADER: usize = 64;
pub(crate) const NB: usize = 10000;
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
    /// Values rounded to 32 bits (the buckets are those of the rounded value).
    const F32: bool;
    fn from64(x: f64) -> Self;
    fn to64(self) -> f64;
    fn put(self, out: &mut Vec<u8>);
}
impl Num for f32 {
    const SIZE: usize = 4;
    const F32: bool = true;
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
    const F32: bool = false;
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

/// `floor(x)` clamped to `0..NB` (NaN in the last): for `x > 0` the cast truncates, which is the floor.
#[inline]
#[allow(clippy::cast_possible_truncation)] // `x` is in 0..NB here
pub(crate) fn bucket(x: f64) -> usize {
    if x.is_nan() || x >= (NB - 1) as f64 {
        NB - 1
    } else if x <= 0.0 {
        0
    } else {
        x as usize
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
    /// Adds what pass 1 counted for a group of scans of this section.
    fn assorbi(&mut self, g: &Gruppo);
    /// Takes the kept peaks of the groups of this section (`rank`: the number of each scan of the file in this section) into the buckets of a round
    /// that spans all of them, ready for `flush`.
    fn carica(&mut self, da: &[&Gruppo], rank: &[u32]) -> Result<()>;
    fn n_peaks(&self) -> u64;
    /// Bytes per peak while a round holds it.
    fn cost(&self, b: usize) -> usize;
    fn begin_round(&mut self, lo: usize, hi: usize) -> Result<()>;
    fn fill(&mut self, sp: &Spectrum, pos: u32) -> Result<()>;
    /// The same for one peak (a peak read back from the scratch file).
    fn fill_pk(&mut self, mz: f64, it: f64, pos: u32) -> Result<()>;
    /// Sorts what the section holds (the round `[lo, hi)`) and writes it.
    fn flush(&mut self, out: &mut dyn Output, off: &Offsets) -> Result<()>;
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
    /// The round holds peaks of more than one group (`carica`): equal m/z are then ordered by scan.
    per_scansione: bool,
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
            per_scansione: false,
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
    fn assorbi(&mut self, g: &Gruppo) {
        let counts = if T::F32 { &g.counts32 } else { &g.counts };
        for (c, x) in self.counts.iter_mut().zip(counts) {
            *c += x;
        }
        self.n += g.n;
    }

    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)] // the indexes come from the table, checked against `n_scans`
    fn carica(&mut self, da: &[&Gruppo], rank: &[u32]) -> Result<()> {
        self.begin_round(0, NB)?;
        for g in da {
            for p in &g.kept {
                let q = Peak {
                    mz: T::from64(p.mz),
                    it: T::from64(p.it),
                    pos: rank[p.pos as usize],
                };
                let b = bucket(q.mz.to64());
                self.round[self.cursor[b]] = q;
                self.cursor[b] += 1;
            }
        }
        self.per_scansione = da.len() > 1;
        Ok(())
    }

    fn n_peaks(&self) -> u64 {
        self.n
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
            self.fill_pk(sp.mz[j], sp.intensity_f64(j), pos)?;
        }
        Ok(())
    }

    #[inline]
    fn fill_pk(&mut self, mz: f64, it: f64, pos: u32) -> Result<()> {
        let p = Peak {
            mz: T::from64(mz),
            it: T::from64(it),
            pos,
        };
        let b = bucket(p.mz.to64());
        if b >= self.lo && b < self.hi {
            self.round[self.cursor[b]] = p;
            self.cursor[b] += 1;
        }
        Ok(())
    }

    fn flush(&mut self, out: &mut dyn Output, off: &Offsets) -> Result<()> {
        let mut round = std::mem::take(&mut self.round);
        for b in self.lo..self.hi {
            let (a, z) = (self.base[b], self.base[b] + self.counts[b] as usize);
            // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
            // (a round filled by passes is in scan order, so the stable sort keeps equal m/z so; `carica` joins groups, `pos` orders them)
            #[allow(clippy::disallowed_methods)]
            if self.per_scansione {
                round[a..z].sort_by(|x, y| by_mz(x, y).then(x.pos.cmp(&y.pos)));
            } else {
                round[a..z].sort_by(by_mz);
            }
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
    /// The key that made the section: 0 `all`, 1 `+`, 2 `-` (`ammette`).
    kind: u8,
    ids: Vec<usize>,
    f32: bool,
    n_peaks: u64,
    off: Offsets,
}

/// Whether the scans of polarity `pol` belong to a section made by key `kind`.
fn ammette(kind: u8, pol: i8) -> bool {
    match kind {
        0 => true,
        1 => pol == 1 || pol == 0,
        _ => pol == -1 || pol == 0,
    }
}

/// The scans of one level and polarity, as pass 1 of the builder sees them: the peaks per 1-unit bucket of m/z and, while they fit, the peaks themselves.
/// A section is the union of the groups it admits, so pass 1 can run before the table (and so the sections) is known.
struct Gruppo {
    level: u8,
    polarity: i8,
    counts: Vec<u32>,
    /// The same as seen after rounding the m/z to 32 bits (where a section holds `f32`: only level 1).
    counts32: Vec<u32>,
    n: u64,
    /// In scan order; `pos` is the number of the scan in the file.
    kept: Vec<Peak<f64>>,
}

impl Gruppo {
    fn nuovo(level: u8, polarity: i8) -> Gruppo {
        Gruppo {
            level,
            polarity,
            counts: vec![0; NB],
            counts32: if level == 1 { vec![0; NB] } else { Vec::new() },
            n: 0,
            kept: Vec::new(),
        }
    }
}

/// Pass 1 of the builder: the groups, filled one scan at a time. `budget / 2` bytes of peaks are kept; past that, with a scratch file (`spill`) they go to it
/// split by m/z (`mzidx_spill`) and the rounds read them from there; without one they are dropped and the rounds decode again.
pub(crate) struct Prima<'a> {
    gruppi: Vec<Gruppo>,
    keep: bool,
    budget: usize,
    visti: usize,
    spill: Option<&'a mut dyn Spill>,
    parti: Option<Parti>,
}

impl<'a> Prima<'a> {
    pub(crate) fn new(budget: usize, spill: Option<&'a mut dyn Spill>) -> Prima<'a> {
        Prima {
            gruppi: Vec::new(),
            keep: true,
            budget,
            visti: 0,
            spill,
            parti: None,
        }
    }

    /// The next scan of the file, with the level and polarity (+1, -1, 0) of its row.
    #[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)] // the indexes come from the table, checked against `n_scans`
    pub(crate) fn add(&mut self, level: u8, polarity: i8, sp: &Spectrum) -> Result<()> {
        let gi = match self
            .gruppi
            .iter()
            .position(|g| g.level == level && g.polarity == polarity)
        {
            Some(i) => i,
            None => {
                self.gruppi.push(Gruppo::nuovo(level, polarity));
                self.gruppi.len() - 1
            }
        };
        let g = &mut self.gruppi[gi];
        let n = sp.mz.len();
        let pos = self.visti as u32;
        if self.keep {
            reserve(&mut g.kept, n)?;
        }
        for j in 0..n {
            let (mz, it) = (sp.mz[j], sp.intensity_f64(j));
            g.counts[bucket(mz)] += 1;
            if level == 1 {
                g.counts32[bucket(f64::from(mz as f32))] += 1;
            }
            if self.keep {
                g.kept.push(Peak { mz, it, pos });
            } else if let (Some(parti), Some(spill)) = (&mut self.parti, self.spill.as_deref_mut())
            {
                parti.push(spill, mz, it, pos)?;
            }
        }
        g.n += n as u64;
        self.visti += 1;
        if self.keep {
            let held: usize = self
                .gruppi
                .iter()
                .map(|g| g.kept.len() * std::mem::size_of::<Peak<f64>>())
                .sum();
            if held > self.budget / 2 {
                self.keep = false;
                if let Some(sp) = self.spill.as_deref_mut() {
                    self.parti = Some(avvia(&mut self.gruppi, sp)?);
                }
                self.gruppi.iter_mut().for_each(|g| g.kept = Vec::new());
            }
        }
        Ok(())
    }
}

/// The budget ran out with a scratch file: what is kept goes to it in the order of the scans (the groups merged by scan number), and the next scans follow it.
#[allow(clippy::indexing_slicing)] // the indexes come from the groups just counted
fn avvia(gruppi: &mut [Gruppo], sp: &mut dyn Spill) -> Result<Parti> {
    let mut counts = vec![0u64; NB];
    for g in gruppi.iter() {
        for (c, &x) in counts.iter_mut().zip(&g.counts) {
            *c += u64::from(x);
        }
    }
    let mut parti = Parti::new(&counts);
    let mut next = vec![0usize; gruppi.len()];
    loop {
        let mut best: Option<usize> = None;
        for (i, g) in gruppi.iter().enumerate() {
            if let Some(p) = g.kept.get(next[i]) {
                if best.is_none_or(|b| p.pos < gruppi[b].kept[next[b]].pos) {
                    best = Some(i);
                }
            }
        }
        let Some(i) = best else { break };
        let p = gruppi[i].kept[next[i]];
        next[i] += 1;
        parti.push(sp, p.mz, p.it, p.pos)?;
    }
    Ok(parti)
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
#[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)] // the indexes come from the table, checked against `n_scans`
pub fn build(
    table: &ScanTable,
    pass: &mut Pass,
    out: &mut dyn Output,
    budget: usize,
    nativo: bool,
) -> Result<Stats> {
    let n_scans = table.level.len();
    if table.polarity.len() != n_scans || table.rt.len() != n_scans || table.tic.len() != n_scans {
        return Err(Error::new("err.file.index").with("detail", "scan table"));
    }
    // -- pass 1: count (and keep, while it fits)
    let mut prima = Prima::new(budget, None);
    let mut failure: Option<Error> = None;
    {
        let mut on = |sp: Spectrum| {
            if failure.is_some() {
                return;
            }
            let seen = prima.visti;
            if seen >= n_scans {
                failure =
                    Some(Error::new("err.file.index").with("detail", "more spectra than scans"));
                return;
            }
            if let Err(e) = prima.add(table.level[seen], table.polarity[seen], &sp) {
                failure = Some(e);
            }
        };
        pass(&mut on)?;
    }
    if let Some(e) = failure {
        return Err(e);
    }
    build_dopo(table, prima, pass, out, budget, nativo)
}

/// The rest of the build once pass 1 has run (`prima`, which counted `table.level.len()` scans): the layout, the arrays of the scans and the peaks,
/// in rounds of buckets decoded again by `pass` when they did not fit.
#[allow(clippy::indexing_slicing, clippy::cast_possible_truncation)] // the indexes come from the table, checked against `n_scans`
pub(crate) fn build_dopo(
    table: &ScanTable,
    mut prima: Prima,
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
    if prima.visti != n_scans {
        return Err(Error::new("err.file.index").with(
            "detail",
            format!("{} spectra, {n_scans} scans", prima.visti),
        ));
    }
    let keep = prima.keep;
    let mut passes = 1usize;
    // -- the groups of scans (level x polarity), those with the same members share a section
    let mut levels: Vec<u8> = table.level.clone();
    // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
    #[allow(clippy::disallowed_methods)]
    levels.sort_unstable();
    levels.dedup();
    let mut infos: Vec<Info> = Vec::new();
    let mut keys: Vec<(String, usize)> = Vec::new();
    for &level in &levels {
        for (kind, (pol, name)) in [(None, "all"), (Some(1i8), "+"), (Some(-1i8), "-")]
            .into_iter()
            .enumerate()
        {
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
                        kind: kind as u8,
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
            let mut sec: Box<dyn Sect> = if s.f32 {
                Box::new(Sec::<f32>::new(s.ids.len()))
            } else {
                Box::new(Sec::<f64>::new(s.ids.len()))
            };
            for g in prima
                .gruppi
                .iter()
                .filter(|g| g.level == s.level && ammette(s.kind, g.polarity))
            {
                sec.assorbi(g);
            }
            sec
        })
        .collect();

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
            let da: Vec<&Gruppo> = prima
                .gruppi
                .iter()
                .filter(|g| g.level == s.level && ammette(s.kind, g.polarity))
                .collect();
            let mut rank = vec![0u32; n_scans];
            for (p, &i) in s.ids.iter().enumerate() {
                rank[i] = p as u32;
            }
            sec.carica(&da, &rank)?;
            sec.flush(out, &s.off)?;
        }
    } else {
        let (mut parti, mut spill) = (prima.parti.take(), prima.spill.take());
        if let (Some(p), Some(sp)) = (parti.as_mut(), spill.as_deref_mut()) {
            p.fine(sp)?;
        }
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
                if let (Some(p), Some(sp)) = (parti.as_ref(), spill.as_deref_mut()) {
                    // the peaks are in the scratch file: no decoding
                    leggi_parti(p, sp, lo, hi, &mut |mz, it, pos| {
                        for &(si, q) in &member[pos as usize] {
                            secs[si].fill_pk(mz, it, q)?;
                        }
                        Ok(())
                    })?;
                } else {
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
                    sec.flush(out, &s.off)?;
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
