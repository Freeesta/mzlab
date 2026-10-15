//! A netCDF-3 file, read only (classic `CDF\x01` and 64-bit offset `CDF\x02`; the format of the Unidata documentation, "The NetCDF File Format"): the header (`numrecs`, dimensions, global
//! attributes, variables with their attributes) and the data of a variable, big-endian, record variables included. Every number read from the file (name lengths, counts, dimensions, offsets)
//! is checked against `Limiti` and against the size of the file before it allocates or reads. netCDF-4 (HDF5) is `err.file.netcdf4`.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limiti;

/// The bytes of the window the header is read through.
const FINESTRA: u64 = 1 << 16;
/// The longest name of a dimension, attribute or variable.
const NOME_MAX: u64 = 1 << 12;
const TAG_DIM: u32 = 0x0A;

const TAG_VAR: u32 = 0x0B;
const TAG_ATT: u32 = 0x0C;

impl Tipo {
    fn da(n: u32) -> Result<Tipo> {
        Ok(match n {
            1 => Tipo::Byte,
            2 => Tipo::Char,
            3 => Tipo::Short,
            4 => Tipo::Int,
            5 => Tipo::Float,
            6 => Tipo::Double,
            _ => return Err(guasto("unknown nc_type")),
        })
    }
    pub fn larghezza(self) -> u64 {
        match self {
            Tipo::Byte | Tipo::Char => 1,
            Tipo::Short => 2,
            Tipo::Int | Tipo::Float => 4,
            Tipo::Double => 8,
        }
    }
    /// One big-endian value of this type.
    fn valore(self, b: &[u8]) -> f64 {
        let a = |n: usize| -> [u8; 8] {
            let mut x = [0u8; 8];
            x[..n].copy_from_slice(&b[..n]);
            x
        };
        match self {
            Tipo::Byte => f64::from(b[0] as i8),
            Tipo::Char => f64::from(b[0]),
            Tipo::Short => f64::from(i16::from_be_bytes([b[0], b[1]])),
            Tipo::Int => f64::from(i32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            Tipo::Float => f64::from(f32::from_be_bytes([b[0], b[1], b[2], b[3]])),
            Tipo::Double => f64::from_be_bytes(a(8)),
        }
    }
}

impl Cur<'_> {
    fn bytes(&mut self, n: u64) -> Result<Vec<u8>> {
        if n > self.limiti.byte || self.pos.checked_add(n).is_none_or(|f| f > self.dim) {
            return Err(guasto("header runs past the end of the file"));
        }
        let (a, f) = (self.pos, self.pos + n);
        if a < self.base || f > self.base + self.buf.len() as u64 {
            let l = n.max(FINESTRA).min(self.dim - a);
            self.buf = vec![0u8; usize::try_from(l).map_err(|_| Error::memory_budget())?];
            self.dati.read_at(a, &mut self.buf)?;
            self.base = a;
        }
        self.pos = f;
        let i = (a - self.base) as usize;
        Ok(self.buf[i..i + n as usize].to_vec())
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    /// An offset: 32 bits in the classic format, 64 in the other.
    fn offset(&mut self, v64: bool) -> Result<u64> {
        if v64 {
            let b = self.bytes(8)?;
            Ok(u64::from_be_bytes([
                b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
            ]))
        } else {
            self.u32().map(u64::from)
        }
    }
    fn riempi(&mut self, n: u64) -> Result<()> {
        self.bytes((4 - n % 4) % 4).map(drop)
    }
    fn nome(&mut self) -> Result<String> {
        let n = u64::from(self.u32()?);
        if n > NOME_MAX {
            return Err(guasto("name too long"));
        }
        let b = self.bytes(n)?;
        self.riempi(n)?;
        Ok(String::from_utf8_lossy(&b).into_owned())
    }
    /// The tag and the count of a list (`ABSENT` is two zeros).
    fn lista(&mut self, tag: u32) -> Result<usize> {
        let (t, n) = (self.u32()?, self.u32()?);
        if t == 0 && n == 0 {
            return Ok(0);
        }
        if t != tag {
            return Err(guasto("unexpected tag in the header"));
        }
        self.limiti.voci_ok(u64::from(n))
    }
    fn attributi(&mut self) -> Result<Vec<Attr>> {
        let n = self.lista(TAG_ATT)?;
        let mut out = Vec::new();
        for _ in 0..n {
            let nome = self.nome()?;
            let tipo = Tipo::da(self.u32()?)?;
            let c = u64::from(self.u32()?);
            let byte = c
                .checked_mul(tipo.larghezza())
                .ok_or_else(Error::memory_budget)?;
            let b = self.bytes(byte)?;
            self.riempi(byte)?;
            let val = if tipo == Tipo::Char {
                Val::Testo(
                    String::from_utf8_lossy(&b)
                        .trim_end_matches('\0')
                        .to_string(),
                )
            } else {
                let w = tipo.larghezza() as usize;
                Val::Num(b.chunks_exact(w).map(|x| tipo.valore(x)).collect())
            };
            out.push(Attr { nome, val });
        }
        Ok(out)
    }
}

impl Cdf {
    /// Parses the header of the file `dati` of `dim` bytes.
    pub fn apri(dati: &mut dyn ReadAt, dim: u64, limiti: &Limiti) -> Result<Cdf> {
        let mut c = Cur {
            dati,
            dim,
            pos: 0,
            buf: Vec::new(),
            base: 0,
            limiti: *limiti,
        };
        let m = c.bytes(4.min(dim)).map_err(|_| guasto("too short"))?;
        if m.starts_with(b"\x89HDF") {
            return Err(Error::new("err.file.netcdf4"));
        }
        if m.len() < 4 || &m[..3] != b"CDF" || !matches!(m[3], 1 | 2) {
            return Err(guasto("not a netCDF-3 file"));
        }
        let v64 = m[3] == 2;
        let numrecs = u64::from(c.u32()?);
        let mut cdf = Cdf {
            numrecs,
            dim: Vec::new(),
            att: Vec::new(),
            var: Vec::new(),
            recsize: 0,
            dimensione: dim,
        };
        cdf.dimensioni(&mut c)?;
        cdf.att = c.attributi()?;
        cdf.variabili(&mut c, v64)?;
        cdf.record(numrecs);
        Ok(cdf)
    }

    fn dimensioni(&mut self, c: &mut Cur) -> Result<()> {
        for _ in 0..c.lista(TAG_DIM)? {
            c.nome()?;
            let lunghezza = u64::from(c.u32()?);
            self.dim.push(Dim { lunghezza });
        }
        Ok(())
    }

    fn variabili(&mut self, c: &mut Cur, v64: bool) -> Result<()> {
        for _ in 0..c.lista(TAG_VAR)? {
            let nome = c.nome()?;
            let n = u64::from(c.u32()?);
            let nd = c.limiti.voci_ok(n)?;
            let mut dims = Vec::new();
            for _ in 0..nd {
                let d = c.u32()? as usize;
                if d >= self.dim.len() {
                    return Err(guasto("dimension id out of range"));
                }
                dims.push(d);
            }
            let att = c.attributi()?;
            let tipo = Tipo::da(c.u32()?)?;
            let vsize = u64::from(c.u32()?);
            let begin = c.offset(v64)?;
            let record = dims.first().is_some_and(|&d| self.dim[d].lunghezza == 0);
            if begin > self.dimensione && !record {
                return Err(guasto("variable starts past the end of the file"));
            }
            self.var.push(Var {
                nome,
                dims,
                att,
                tipo,
                vsize,
                begin,
                record,
            });
        }
        Ok(())
    }

    /// `recsize` (a single record variable is not padded), and `numrecs` when the file says "streaming".
    fn record(&mut self, numrecs: u64) {
        let rec: Vec<&Var> = self.var.iter().filter(|v| v.record).collect();
        self.recsize = match rec.as_slice() {
            [v] => per_record(&self.dim, v).saturating_mul(v.tipo.larghezza()),
            _ => rec.iter().map(|v| v.vsize).sum(),
        };
        if numrecs == 0xFFFF_FFFF {
            let inizio = rec.iter().map(|v| v.begin).min().unwrap_or(0);
            self.numrecs = (self.dimensione.saturating_sub(inizio))
                .checked_div(self.recsize)
                .unwrap_or(0);
        }
    }

    pub fn trova(&self, nome: &str) -> Option<&Var> {
        self.var.iter().find(|v| v.nome == nome)
    }

    /// A global attribute as text.
    pub fn testo(&self, nome: &str) -> Option<&str> {
        self.att.iter().find_map(|a| match &a.val {
            Val::Testo(t) if a.nome == nome => Some(t.trim()).filter(|t| !t.is_empty()),
            _ => None,
        })
    }

    /// A global attribute as a number.
    pub fn numero(&self, nome: &str) -> Option<f64> {
        self.att.iter().find_map(|a| match &a.val {
            Val::Num(n) if a.nome == nome => n.first().copied(),
            _ => None,
        })
    }

    /// The length of a dimension (the number of records for the unlimited one).
    pub fn lunghezza_dim(&self, d: usize) -> u64 {
        match self.dim.get(d) {
            Some(x) if x.lunghezza == 0 => self.numrecs,
            Some(x) => x.lunghezza,
            None => 0,
        }
    }

    /// The number of elements of a variable.
    pub fn elementi(&self, v: &Var) -> u64 {
        v.dims
            .iter()
            .fold(1u64, |a, &d| a.saturating_mul(self.lunghezza_dim(d)))
    }

    /// The bytes of elements `da..da + n` of `v`.
    fn grezzi(
        &self,
        dati: &mut dyn ReadAt,
        limiti: &Limiti,
        v: &Var,
        da: u64,
        n: u64,
    ) -> Result<Vec<u8>> {
        let w = v.tipo.larghezza();
        let fine = da.checked_add(n).ok_or_else(Error::memory_budget)?;
        if fine > self.elementi(v) {
            return Err(guasto("read past the end of a variable"));
        }
        let totale = n
            .checked_mul(w)
            .filter(|&t| t <= limiti.byte)
            .ok_or_else(Error::memory_budget)?;
        let mut out = vec![0u8; usize::try_from(totale).map_err(|_| Error::memory_budget())?];
        let (mut e, mut k) = (da, 0usize);
        let epr = if v.record {
            per_record(&self.dim, v).max(1)
        } else {
            u64::MAX
        };
        while e < fine {
            let (pos, blocco) = if v.record {
                let (r, i) = (e / epr, e % epr);
                (
                    v.begin
                        .saturating_add(r.saturating_mul(self.recsize))
                        .saturating_add(i * w),
                    (epr - i).min(fine - e),
                )
            } else {
                (v.begin.saturating_add(e * w), fine - e)
            };
            let len = (blocco * w) as usize;
            if pos
                .checked_add(len as u64)
                .is_none_or(|f| f > self.dimensione)
            {
                return Err(guasto("variable data past the end of the file"));
            }
            dati.read_at(pos, &mut out[k..k + len])?;
            k += len;
            e += blocco;
        }
        Ok(out)
    }

    /// Elements `da..da + n` of `v` as numbers (`scale_factor` and `add_offset` are not applied here).
    pub fn leggi(
        &self,
        dati: &mut dyn ReadAt,
        limiti: &Limiti,
        v: &Var,
        da: u64,
        n: u64,
    ) -> Result<Vec<f64>> {
        limiti.voci_ok(n)?;
        let b = self.grezzi(dati, limiti, v, da, n)?;
        Ok(b.chunks_exact(v.tipo.larghezza() as usize)
            .map(|x| v.tipo.valore(x))
            .collect())
    }

    /// The strings of a `char` variable (rows of its last dimension, trailing NULs and spaces cut).
    pub fn stringhe(&self, dati: &mut dyn ReadAt, limiti: &Limiti, v: &Var) -> Result<Vec<String>> {
        let riga = v.dims.last().map_or(1, |&d| self.lunghezza_dim(d)).max(1);
        let n = self.elementi(v);
        let b = self.grezzi(dati, limiti, v, 0, n)?;
        Ok(b.chunks(riga as usize)
            .map(|r| {
                String::from_utf8_lossy(r)
                    .trim_end_matches(['\0', ' '])
                    .trim()
                    .to_string()
            })
            .collect())
    }
}

impl Var {
    /// An attribute of the variable as a number.
    pub fn numero(&self, nome: &str) -> Option<f64> {
        self.att.iter().find_map(|a| match &a.val {
            Val::Num(n) if a.nome == nome => n.first().copied(),
            _ => None,
        })
    }
    pub fn testo(&self, nome: &str) -> Option<&str> {
        self.att.iter().find_map(|a| match &a.val {
            Val::Testo(t) if a.nome == nome => Some(t.trim()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Byte,
    Char,
    Short,
    Int,
    Float,
    Double,
}

pub struct Dim {
    /// 0: the unlimited (record) dimension.
    pub lunghezza: u64,
}

pub enum Val {
    Testo(String),
    Num(Vec<f64>),
}

pub struct Attr {
    pub nome: String,
    pub val: Val,
}

pub struct Var {
    pub nome: String,
    pub dims: Vec<usize>,
    pub att: Vec<Attr>,
    pub tipo: Tipo,
    pub vsize: u64,
    pub begin: u64,
    /// Its first dimension is the unlimited one.
    pub record: bool,
}

pub struct Cdf {
    pub numrecs: u64,
    pub dim: Vec<Dim>,
    pub att: Vec<Attr>,
    pub var: Vec<Var>,
    /// Bytes of one record (all the record variables).
    pub recsize: u64,
    pub dimensione: u64,
}

/// The header read through a window over the file.
struct Cur<'a> {
    dati: &'a mut dyn ReadAt,
    dim: u64,
    pos: u64,
    buf: Vec<u8>,
    base: u64,
    limiti: Limiti,
}

fn guasto(why: &str) -> Error {
    Error::new("err.dam.unreadable").with("message", why)
}

/// Elements of a variable per record (the product of its dimensions after the unlimited one).
fn per_record(dim: &[Dim], v: &Var) -> u64 {
    v.dims[1..]
        .iter()
        .fold(1u64, |a, &d| a.saturating_mul(dim[d].lunghezza))
}
