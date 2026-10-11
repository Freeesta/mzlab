//! Reader of the offline suspect index `.mzsus` v1 (format: `docs/agenti/sospetti.md`; reference reader: `tools/genera_sospetti.py::leggi_sospetti`).
//! `Suspects::open` parses the whole file into rows ordered by mass; `search` is a binary search of the window `|m - mass| <= mass * ppm * 1e-6`.
//! A hit of mass is a candidate with its evidence, never a name for a feature.

const MAGIC: &[u8; 4] = b"MZSU";
const HEAD: usize = 32;
const VERSION: u16 = 1;

/// Sources of a row (bits of `Hit::sources`).
pub const SRC_SUSDAT: u8 = 1;
pub const SRC_PUBCHEMLITE: u8 = 2;
pub const SRC_TRANSPUBCHEM: u8 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub mass: f64,
    /// InChIKey first block (14 ASCII letters).
    pub key: [u8; 14],
    /// Index into the formula table, see `Suspects::formula`.
    pub formula: u32,
    pub sources: u8,
    pub pubmed: u16,
    pub patents: u16,
    /// 1-based row of the first reaction in Transformations in PubChem; 0 = none.
    pub reaction: u32,
}

#[derive(Debug, PartialEq)]
pub enum SuspectsError {
    /// Not a `.mzsus` (magic) or truncated/inconsistent.
    Corrupt(&'static str),
    /// The file needs a newer reader.
    Version { found: u16, need: u16 },
}

pub struct Suspects {
    rows: Vec<Hit>,
    formulas: Vec<String>,
    manifest: String,
}

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    /// The next `size` bytes; the cursor then moves to the next multiple of 8.
    fn take(&mut self, size: usize) -> Result<&'a [u8], SuspectsError> {
        let end = self
            .p
            .checked_add(size)
            .filter(|&e| e <= self.b.len())
            .ok_or(SuspectsError::Corrupt("truncated"))?;
        let s = &self.b[self.p..end];
        self.p = end + (8 - size % 8) % 8;
        Ok(s)
    }
}

impl Suspects {
    pub fn open(data: &[u8]) -> Result<Suspects, SuspectsError> {
        parse(data)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The manifest JSON (sources with licence, DOI and SHA-256).
    pub fn manifest(&self) -> &str {
        &self.manifest
    }

    pub fn formula(&self, h: &Hit) -> &str {
        &self.formulas[h.formula as usize]
    }

    /// Rows with `|m - mass| <= mass * ppm * 1e-6`, in order of mass then InChIKey-14.
    pub fn search(&self, mass: f64, ppm: f64) -> &[Hit] {
        let d = mass * ppm * 1e-6;
        let (lo, hi) = (mass - d, mass + d);
        let a = self.rows.partition_point(|h| h.mass < lo);
        let b = self.rows.partition_point(|h| h.mass <= hi);
        &self.rows[a..b.max(a)]
    }
}

fn u16s(b: &[u8]) -> impl Iterator<Item = u16> + '_ {
    b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]))
}

fn u32s(b: &[u8]) -> impl Iterator<Item = u32> + '_ {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn parse(data: &[u8]) -> Result<Suspects, SuspectsError> {
    if data.len() < HEAD || &data[..4] != MAGIC {
        return Err(SuspectsError::Corrupt("not a .mzsus"));
    }
    let need = u16::from_le_bytes([data[6], data[7]]);
    if need > VERSION {
        return Err(SuspectsError::Version {
            found: need,
            need: VERSION,
        });
    }
    let (n, nf, nr, nman) = (
        le32(data, 8) as usize,
        le32(data, 12) as usize,
        le32(data, 16) as usize,
        le32(data, 20) as usize,
    );
    let mut c = Cur { b: data, p: HEAD };
    let manifest = String::from_utf8_lossy(c.take(nman)?).into_owned();
    let masses = c.take(8 * n)?;
    let keys = c.take(14 * n)?;
    let fidx = c.take(4 * n)?;
    let src = c.take(n)?;
    let lit = c.take(2 * n)?;
    let pat = c.take(2 * n)?;
    let react = c.take(8 * nr)?;
    let offs: Vec<u32> = u32s(c.take(4 * (nf + 1))?).collect();
    let blob = c.take(offs[nf] as usize)?;
    let mut formulas = Vec::with_capacity(nf);
    for i in 0..nf {
        let (a, b) = (offs[i] as usize, offs[i + 1] as usize);
        if a > b || b > blob.len() {
            return Err(SuspectsError::Corrupt("formula offsets"));
        }
        formulas.push(String::from_utf8_lossy(&blob[a..b]).into_owned());
    }
    let mut rows: Vec<Hit> = Vec::with_capacity(n);
    let (mut lit, mut pat, mut fidx) = (u16s(lit), u16s(pat), u32s(fidx));
    for i in 0..n {
        let mut key = [0u8; 14];
        key.copy_from_slice(&keys[14 * i..14 * i + 14]);
        let mass = f64::from_le_bytes(masses[8 * i..8 * i + 8].try_into().unwrap());
        let formula = fidx.next().unwrap();
        if formula as usize >= nf {
            return Err(SuspectsError::Corrupt("formula index"));
        }
        rows.push(Hit {
            mass,
            key,
            formula,
            sources: src[i],
            pubmed: lit.next().unwrap(),
            patents: pat.next().unwrap(),
            reaction: 0,
        });
    }
    for r in react.chunks_exact(8) {
        let (row, id) = (le32(r, 0) as usize, le32(r, 4));
        rows.get_mut(row)
            .ok_or(SuspectsError::Corrupt("reaction row"))?
            .reaction = id;
    }
    Ok(Suspects {
        rows,
        formulas,
        manifest,
    })
}
