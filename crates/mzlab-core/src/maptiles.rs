//! The pyramid of tiles of the ion map (RT × m/z) of one file, built once from its index in a single pass over the peaks (already sorted by m/z)
//! and answered by `api/maptiles`: every zoom or pan then costs the cells of the view, not the peaks of the file.
//!
//! Level 0 has one row per scan and cells of `BASE_DMZ` Da starting at m/z 0; level L merges 2^L scans and 2^L cells, taking the maximum, so a cell of
//! any level is exactly the maximum (as float32) of the peaks that `region_map` puts in the same cell of an aligned region. Cells live in sparse
//! tiles of `TILE` × `TILE`. The pyramid serialises to bytes (`to_bytes` / `from_bytes`, checked against a stamp of the index) to be kept beside the `.mzidx`.

use std::collections::HashMap;

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::routes::Routes;

pub const TILE: usize = 256;
/// Width in Da of a cell of level 0 (a power of two: every level edge is exact in binary).
pub const BASE_DMZ: f64 = 0.25;
const MAX_LEVEL: usize = 14;
const MAGIC: &[u8; 8] = b"MZTILE01";

impl Pyramid {
    fn flush(level: &mut Level, s: &mut Strip) {
        for (ty, t) in s.tiles.iter_mut().enumerate() {
            let cells: Tile = t
                .iter()
                .enumerate()
                .filter(|(_, v)| **v > 0.0)
                .map(|(i, v)| (i as u16, *v))
                .collect();
            if !cells.is_empty() {
                level.insert((ty as u32, s.tx), cells);
            }
            t.fill(0.0);
        }
    }

    /// Builds the pyramid in one pass: `each` feeds `(scan position, m/z, intensity)` with m/z ascending.
    fn build(
        n_scans: usize,
        mut each: impl FnMut(&mut dyn FnMut(usize, f64, f64)) -> Result<()>,
    ) -> Result<Self> {
        let nty = n_scans.div_ceil(TILE).max(1);
        let mut l0: Level = HashMap::new();
        let mut strip = Strip {
            tx: 0,
            tiles: vec![vec![0.0; TILE * TILE]; nty],
        };
        let mut any = false;
        each(&mut |p, mz, y| {
            let c = (mz / BASE_DMZ).floor();
            if !(c >= 0.0 && c < u32::MAX as f64) || p >= n_scans {
                return;
            }
            let (c, tx) = (c as usize, (c as usize / TILE) as u32);
            if any && tx != strip.tx {
                Self::flush(&mut l0, &mut strip);
            }
            any = true;
            strip.tx = tx;
            let v = &mut strip.tiles[p / TILE][(p % TILE) * TILE + c % TILE];
            if y as f32 > *v {
                *v = y as f32;
            }
        })?;
        if any {
            Self::flush(&mut l0, &mut strip);
        }
        let mut levels = vec![l0];
        let mut dense = vec![0.0f32; TILE * TILE];
        while levels.len() <= MAX_LEVEL {
            let Some(prev) = levels.last() else { break };
            if (1usize << (levels.len() - 1)) >= n_scans && prev.keys().all(|k| *k == (0, 0)) {
                break;
            }
            let mut next: Level = HashMap::new();
            let mut keys: Vec<(u32, u32)> = prev.keys().map(|&(ty, tx)| (ty / 2, tx / 2)).collect();
            // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
            #[allow(clippy::disallowed_methods)]
            keys.sort_unstable();
            keys.dedup();
            for (ty, tx) in keys {
                dense.fill(0.0);
                for (dy, dx) in [(0u32, 0u32), (0, 1), (1, 0), (1, 1)] {
                    let Some(child) = prev.get(&(ty * 2 + dy, tx * 2 + dx)) else {
                        continue;
                    };
                    let (oy, ox) = (dy as usize * TILE / 2, dx as usize * TILE / 2);
                    for &(i, v) in child {
                        let (r, c) = (i as usize / TILE / 2 + oy, i as usize % TILE / 2 + ox);
                        let d = &mut dense[r * TILE + c];
                        if v > *d {
                            *d = v;
                        }
                    }
                }
                let cells: Tile = dense
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v > 0.0)
                    .map(|(i, v)| (i as u16, *v))
                    .collect();
                if !cells.is_empty() {
                    next.insert((ty, tx), cells);
                }
            }
            levels.push(next);
        }
        Ok(Pyramid { n_scans, levels })
    }

    /// Bytes the pyramid holds in memory (a cell is 8 bytes with its position).
    pub fn bytes(&self) -> usize {
        self.levels
            .iter()
            .flat_map(|l| l.values())
            .map(|t| t.len() * 8 + 64)
            .sum()
    }

    pub fn max_level(&self) -> usize {
        self.levels.len() - 1
    }

    /// Rows of level `l` (groups of 2^l scans).
    pub fn rows(&self, l: usize) -> usize {
        self.n_scans.div_ceil(1 << l)
    }

    /// The cells `rows r0..r1`, `cols c0..c1` of level `l` as a dense `(r1 - r0) × (c1 - c0)` array (row-major).
    pub fn rect(&self, l: usize, (r0, r1): (usize, usize), (c0, c1): (usize, usize)) -> Vec<f32> {
        let w = c1.saturating_sub(c0);
        let mut out = vec![0.0f32; r1.saturating_sub(r0) * w];
        let Some(level) = self.levels.get(l) else {
            return out;
        };
        if r1 <= r0 || c1 <= c0 {
            return out;
        }
        for ty in r0 / TILE..=(r1 - 1) / TILE {
            for tx in c0 / TILE..=(c1 - 1) / TILE {
                let Some(t) = level.get(&(ty as u32, tx as u32)) else {
                    continue;
                };
                for &(i, v) in t {
                    let (r, c) = (ty * TILE + i as usize / TILE, tx * TILE + i as usize % TILE);
                    if r >= r0 && r < r1 && c >= c0 && c < c1 {
                        out[(r - r0) * w + (c - c0)] = v;
                    }
                }
            }
        }
        out
    }

    /// Bytes to keep beside the index; `stamp` ties them to the index they come from.
    pub fn to_bytes(&self, stamp: u64) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&stamp.to_le_bytes());
        b.extend_from_slice(&(self.n_scans as u64).to_le_bytes());
        b.extend_from_slice(&(self.levels.len() as u32).to_le_bytes());
        for lv in &self.levels {
            let mut keys: Vec<_> = lv.keys().copied().collect();
            // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
            #[allow(clippy::disallowed_methods)]
            keys.sort_unstable();
            b.extend_from_slice(&(keys.len() as u32).to_le_bytes());
            for k in keys {
                let t = &lv[&k];
                b.extend_from_slice(&k.0.to_le_bytes());
                b.extend_from_slice(&k.1.to_le_bytes());
                b.extend_from_slice(&(t.len() as u32).to_le_bytes());
                for &(i, v) in t {
                    b.extend_from_slice(&i.to_le_bytes());
                    b.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        b
    }

    /// The pyramid of `to_bytes`, or `None` when the bytes are damaged or belong to another index.
    pub fn from_bytes(b: &[u8], stamp: u64) -> Option<Self> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Option<&[u8]> {
            let s = b.get(at..at.checked_add(n)?)?;
            at += n;
            Some(s)
        };
        if take(8)? != MAGIC {
            return None;
        }
        let u32_ = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap_or([0; 4]));
        let u64_ = |s: &[u8]| u64::from_le_bytes(s.try_into().unwrap_or([0; 8]));
        if u64_(take(8)?) != stamp {
            return None;
        }
        let n_scans = u64_(take(8)?) as usize;
        let n_levels = u32_(take(4)?) as usize;
        if n_levels == 0 || n_levels > MAX_LEVEL + 1 {
            return None;
        }
        let mut levels = Vec::with_capacity(n_levels);
        for _ in 0..n_levels {
            let n_tiles = u32_(take(4)?) as usize;
            let mut lv: Level = HashMap::new();
            for _ in 0..n_tiles {
                let (ty, tx, n) = (u32_(take(4)?), u32_(take(4)?), u32_(take(4)?) as usize);
                let raw = take(n.checked_mul(6)?)?;
                let t: Tile = raw
                    .chunks_exact(6)
                    .map(|c| {
                        (
                            u16::from_le_bytes([c[0], c[1]]),
                            f32::from_le_bytes([c[2], c[3], c[4], c[5]]),
                        )
                    })
                    .collect();
                lv.insert((ty, tx), t);
            }
            levels.push(lv);
        }
        (at == b.len()).then_some(Pyramid { n_scans, levels })
    }
}

impl<R: ReadAt> Routes<R> {
    /// The pyramid of a level (built on first use), `None` when the level has no peaks or its scans are not in time order.
    pub(crate) fn pyramid(&mut self, level: u8) -> Result<Option<std::sync::Arc<Pyramid>>> {
        if let Some(t) = self.tiles.iter_mut().find(|t| t.0 == level) {
            t.2 = super::routes::next_tick();
            return Ok(t.1.clone());
        }
        let p = self.build_pyramid(level)?.map(std::sync::Arc::new);
        self.tiles
            .push((level, p.clone(), super::routes::next_tick()));
        Ok(p)
    }

    /// Stamp that ties the saved bytes of a pyramid to the index it comes from (shape of the section and the first and last RT).
    fn tiles_stamp(&mut self, level: u8) -> Result<Option<u64>> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(None);
        };
        let s = &self.idx.sections[si];
        let mut h = [
            s.n_peaks,
            s.n_scans as u64,
            s.n_fence as u64,
            level as u64,
            s.f4 as u64,
        ]
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, v| {
            (h ^ v).wrapping_mul(0x100_0000_01b3)
        });
        let rt = &self.idx.arrays(si)?.rt;
        for v in [rt.first(), rt.last()].into_iter().flatten() {
            h = (h ^ v.to_bits()).wrapping_mul(0x100_0000_01b3);
        }
        Ok(Some(h))
    }

    /// True when the pyramid of the level is in memory (the cache may have dropped it: the page then loads it again from OPFS).
    pub fn tiles_held(&self, level: u8) -> bool {
        self.tiles.iter().any(|t| t.0 == level && t.1.is_some())
    }

    /// The bytes of the pyramid of a level (built if need be) to keep beside the index; empty when the level has none.
    pub fn tiles_export(&mut self, level: u8) -> Result<Vec<u8>> {
        let (Some(p), Some(stamp)) = (self.pyramid(level)?, self.tiles_stamp(level)?) else {
            return Ok(Vec::new());
        };
        Ok(p.to_bytes(stamp))
    }

    /// Takes saved bytes as the pyramid of a level; false when they are damaged or belong to another index (the pyramid is then built from the index).
    pub fn tiles_import(&mut self, level: u8, bytes: &[u8]) -> bool {
        let Ok(Some(stamp)) = self.tiles_stamp(level) else {
            return false;
        };
        let Some(p) = Pyramid::from_bytes(bytes, stamp) else {
            return false;
        };
        self.tiles.retain(|t| t.0 != level);
        self.tiles.push((
            level,
            Some(std::sync::Arc::new(p)),
            super::routes::next_tick(),
        ));
        true
    }

    /// The pyramid of a level for tests of other crates (`pyramid` is the engine's own entry).
    pub fn pyramid_for_test(&mut self, level: u8) -> Option<std::sync::Arc<Pyramid>> {
        self.pyramid(level).ok().flatten()
    }

    fn build_pyramid(&mut self, level: u8) -> Result<Option<Pyramid>> {
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(None);
        };
        let (n_peaks, n_scans) = (self.idx.sections[si].n_peaks, self.idx.sections[si].n_scans);
        if n_peaks == 0 || n_scans == 0 {
            return Ok(None);
        }
        if self.idx.arrays(si)?.rt.windows(2).any(|w| w[1] < w[0]) {
            return Ok(None);
        }
        let idx = &mut self.idx;
        Ok(Some(Pyramid::build(n_scans, |f| {
            idx.each_peak(si, 0, n_peaks, &mut *f)
        })?))
    }

    /// The cells of the view `[rt0, rt1] × [mz0, mz1]` at the finest level whose aligned rectangle holds at most `nrt × nmz` cells.
    pub(crate) fn tile_view(
        &mut self,
        level: u8,
        (rt0, rt1): (f64, f64),
        (mz0, mz1): (f64, f64),
        nrt: usize,
        nmz: usize,
    ) -> Result<Option<TileView>> {
        let Some(p) = self.pyramid(level)? else {
            return Ok(None);
        };
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(None);
        };
        let rt = self.idx.arrays(si)?.rt.clone();
        let n = rt.len();
        let first = rt.partition_point(|&r| r < rt0).min(n.saturating_sub(1));
        let last = rt.partition_point(|&r| r <= rt1).max(first + 1).min(n);
        let l = (0..=p.max_level())
            .find(|&l| {
                let cols = ((mz1 / cell_width(l)).ceil() - (mz0.max(0.0) / cell_width(l)).floor())
                    as usize;
                ((last - 1) >> l) + 1 - (first >> l) <= nrt && cols <= nmz
            })
            .unwrap_or(p.max_level());
        let (g0, g1) = (first >> l, ((last - 1) >> l) + 1);
        let w = cell_width(l);
        let (c0, c1) = (
            (mz0.max(0.0) / w).floor() as usize,
            (mz1.max(0.0) / w).ceil() as usize,
        );
        let data = p.rect(l, (g0, g1), (c0, c1.max(c0)));
        let mut edges: Vec<f64> = (g0..g1).map(|g| rt[g << l]).collect();
        let step = if n > 1 {
            (rt[n - 1] - rt[0]) / (n - 1) as f64
        } else {
            0.0
        };
        edges.push(if g1 << l < n {
            rt[g1 << l]
        } else {
            rt[n - 1] + step
        });
        Ok(Some(TileView {
            tile_level: l,
            row0: g0 << l,
            rows: g1 - g0,
            col0: c0,
            cols: c1.max(c0) - c0,
            data,
            rt: edges,
        }))
    }
}

/// The cells of one tile that are not zero: (`row * TILE + col`, value), in increasing order of position.
type Tile = Vec<(u16, f32)>;
type Level = HashMap<(u32, u32), Tile>;

pub struct Pyramid {
    pub n_scans: usize,
    levels: Vec<Level>,
}

/// Width in Da of a cell of level `l`.
pub fn cell_width(l: usize) -> f64 {
    BASE_DMZ * (1u64 << l) as f64
}

/// What `api/maptiles` returns: the level, the aligned cells, and where they are.
pub struct TileView {
    pub tile_level: usize,
    /// First and last scan position (exclusive) of the rows, in scans.
    pub row0: usize,
    pub rows: usize,
    pub col0: usize,
    pub cols: usize,
    pub data: Vec<f32>,
    /// RT of the start of every row, plus the end of the last one.
    pub rt: Vec<f64>,
}

/// Dense strip of tiles of one column of tiles (`tx`), the rows of the file in tiles of `TILE`.
struct Strip {
    tx: u32,
    tiles: Vec<Vec<f32>>,
}
