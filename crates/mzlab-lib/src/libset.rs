//! Several libraries searched as one: each library keeps its own file and indexes, the set searches the ones selected by a mask with shared accumulators
//! and fuses the best hits by score. A sidecar (built once per combination of files) groups the same spectrum found in several libraries without merging them.

use crate::error::{Error, Result};
use crate::flash::{flash_top, FlashAcc, FlashMode, FlashScore};
use crate::frag_tol::FragTol;
use crate::io::Source;
use crate::read::Library;
use crate::search::{analogs_with, identity_with, AnalogHit, AnalogOpts, Hit, IdentityOpts, Query};
use crate::sortutil;
use crate::util::{crc32c, le_u16, le_u32};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;

/// A mask bit per library, so a set holds at most this many.
pub const MAX_LIBS: usize = 16;

const SIDECAR_MAGIC: [u8; 4] = *b"MZLS";
const SIDECAR_VERSION: u16 = 1;
/// How many of the highest peaks identify a spectrum in the duplicate key.
const KEY_PEAKS: usize = 6;

/// A hit and the library it came from (`group` and `libs` only with a sidecar: the duplicate group and the libraries, within the mask, that hold it).
#[derive(Debug, Clone)]
pub struct SetHit<T> {
    pub lib: u16,
    /// Spectrum number in its library.
    pub index: u32,
    pub hit: T,
    pub group: Option<u32>,
    pub libs: u16,
}

/// Duplicate groups of a set: for each spectrum (libraries one after the other) its group id and the mask of the libraries that contain the group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    fingerprints: Vec<([u8; 32], u32)>,
    group: Vec<u32>,
    libs: Vec<u16>,
}

impl Sidecar {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(16 + self.group.len() * 6 + self.fingerprints.len() * 36);
        b.extend_from_slice(&SIDECAR_MAGIC);
        b.extend_from_slice(&SIDECAR_VERSION.to_le_bytes());
        b.extend_from_slice(&(self.fingerprints.len() as u16).to_le_bytes());
        for (f, n) in &self.fingerprints {
            b.extend_from_slice(f);
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(&(self.group.len() as u32).to_le_bytes());
        for g in &self.group {
            b.extend_from_slice(&g.to_le_bytes());
        }
        for m in &self.libs {
            b.extend_from_slice(&m.to_le_bytes());
        }
        let c = crc32c(&b);
        b.extend_from_slice(&c.to_le_bytes());
        b
    }

    /// `None` when the bytes are damaged or were built for other files (the caller builds the sidecar again).
    pub fn from_bytes(b: &[u8], expect: &[([u8; 32], u32)]) -> Option<Sidecar> {
        let body = b.get(..b.len().checked_sub(4)?)?;
        if crc32c(body) != le_u32(b, b.len() - 4) || body.get(..4)? != SIDECAR_MAGIC {
            return None;
        }
        if le_u16(body, 4) != SIDECAR_VERSION || le_u16(body, 6) as usize != expect.len() {
            return None;
        }
        let mut at = 8;
        let mut fingerprints = Vec::with_capacity(expect.len());
        for e in expect {
            let f: [u8; 32] = body.get(at..at + 32)?.try_into().ok()?;
            let n = le_u32(body.get(..at + 36)?, at + 32);
            if (f, n) != *e {
                return None;
            }
            fingerprints.push((f, n));
            at += 36;
        }
        let total = le_u32(body.get(..at + 4)?, at) as usize;
        at += 4;
        let sum: u64 = expect.iter().map(|e| e.1 as u64).sum();
        if total as u64 != sum || body.len() != at + total * 6 {
            return None;
        }
        let group = (0..total).map(|i| le_u32(body, at + i * 4)).collect();
        at += total * 4;
        let libs = (0..total).map(|i| le_u16(body, at + i * 2)).collect();
        Some(Sidecar {
            fingerprints,
            group,
            libs,
        })
    }

    /// Number of distinct spectra once the duplicates are counted once.
    pub fn n_groups(&self) -> usize {
        self.group.iter().max().map_or(0, |m| *m as usize + 1)
    }
}

pub struct LibSet<S: Source> {
    libs: Vec<Library<S>>,
    /// First global spectrum number of each library (and the total at the end).
    base: Vec<u32>,
    acc: RefCell<FlashAcc>,
    sidecar: Option<Sidecar>,
}

/// Key of a duplicate: InChIKey hash, polarity, precursor to 0.001, the `KEY_PEAKS` highest peaks (by rank, m/z to 0.01 Da).
type DupKey = (u64, i8, i64, u64);
impl<S: Source> LibSet<S> {
    pub fn new(libs: Vec<Library<S>>) -> Result<Self> {
        if libs.len() > MAX_LIBS {
            return Err(Error::too_large("libraries"));
        }
        let mut base = vec![0u32];
        for l in &libs {
            let next = base[base.len() - 1]
                .checked_add(l.n_spectra())
                .ok_or_else(|| Error::too_large("spectra"))?;
            base.push(next);
        }
        Ok(LibSet {
            libs,
            base,
            acc: RefCell::new(FlashAcc::new()),
            sidecar: None,
        })
    }

    pub fn len(&self) -> usize {
        self.libs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.libs.is_empty()
    }

    pub fn lib(&self, k: usize) -> Option<&Library<S>> {
        self.libs.get(k)
    }

    /// Mask selecting every library of the set.
    pub fn all(&self) -> u16 {
        ((1u32 << self.libs.len()) - 1) as u16
    }

    /// Fingerprint and size of each library: what a sidecar is built for.
    pub fn fingerprints(&self) -> Vec<([u8; 32], u32)> {
        self.libs
            .iter()
            .map(|l| (l.fingerprint(), l.n_spectra()))
            .collect()
    }

    pub fn sidecar(&self) -> Option<&Sidecar> {
        self.sidecar.as_ref()
    }

    /// Uses a stored sidecar when it matches these files; `false` (and none attached) when it does not.
    pub fn attach_sidecar(&mut self, bytes: &[u8]) -> bool {
        self.sidecar = Sidecar::from_bytes(bytes, &self.fingerprints());
        self.sidecar.is_some()
    }

    pub fn set_sidecar(&mut self, s: Sidecar) {
        self.sidecar = Some(s);
    }
}

fn fnv(h: &mut u64, x: u64) {
    for b in x.to_le_bytes() {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

impl<S: Source> LibSet<S> {
    /// Finds the same spectrum in several libraries: every spectrum gets the id of its duplicate group and the mask of the libraries holding the group.
    pub fn dedup(&self) -> Result<Sidecar> {
        let total = *self.base.last().unwrap_or(&0) as usize;
        let mut group: Vec<u32> = Vec::with_capacity(total);
        let mut seen: HashMap<DupKey, u32> = HashMap::new();
        let mut libs_of: Vec<u16> = Vec::new();
        for (k, lib) in self.libs.iter().enumerate() {
            let ik = lib.inchikeys()?;
            for i in 0..lib.n_spectra() {
                let rec = lib.rec(i)?;
                let (mz, it) = lib.peaks_q(i)?;
                let key = (
                    ik[i as usize],
                    rec.pol,
                    (rec.prec * 1000.0).round() as i64,
                    top_peaks_hash(&mz, &it),
                );
                let next = seen.len() as u32;
                let g = *seen.entry(key).or_insert(next);
                if g as usize == libs_of.len() {
                    libs_of.push(0);
                }
                libs_of[g as usize] |= 1 << k;
                group.push(g);
            }
        }
        let libs = group.iter().map(|g| libs_of[*g as usize]).collect();
        Ok(Sidecar {
            fingerprints: self.fingerprints(),
            group,
            libs,
        })
    }
}

fn top_peaks_hash(mz: &[u32], it: &[u16]) -> u64 {
    let mut idx: Vec<usize> = (0..mz.len()).collect();
    sortutil::sort_by(&mut idx, |&a, &b| it[b].cmp(&it[a]).then(mz[a].cmp(&mz[b])));
    idx.truncate(KEY_PEAKS);
    sortutil::sort_by(&mut idx, |&a, &b| mz[a].cmp(&mz[b]));
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for i in idx {
        fnv(&mut h, ((mz[i] as u64) + 500) / 1000);
    }
    h
}

mod search;
