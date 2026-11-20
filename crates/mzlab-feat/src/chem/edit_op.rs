//! One operation of an edit patch as it is read: its fields with their types, ranges and units, each failure naming the operation.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

/// Angstrom per bond length of the drawing.
const BOND_A: f64 = 1.5;

/// The failure of operation `index` (0-based): `op` is its name, `reason` a stable word, `detail` what it was about.
pub(super) fn fail(index: usize, op: &str, reason: &'static str, detail: impl ToString) -> Error {
    Error::new("err.mol.draw.edit")
        .with("index", index)
        .with("op", op)
        .with("reason", reason)
        .with("detail", detail)
}

/// One operation with the means to fail with its name and index.
pub(super) struct Op<'a> {
    pub(super) i: usize,
    pub(super) name: &'a str,
    pub(super) v: &'a Value,
}

impl Op<'_> {
    pub(super) fn err(&self, reason: &'static str, detail: impl ToString) -> Error {
        fail(self.i, self.name, reason, detail)
    }

    /// Only the keys in `ok` (and `op`) may be there: a misspelt key is an error, never ignored.
    pub(super) fn only(&self, ok: &[&str]) -> Result<()> {
        match self
            .v
            .entries()
            .iter()
            .find(|(k, _)| k != "op" && !ok.contains(&k.as_str()))
        {
            Some((k, _)) => Err(self.err("unknown_field", k)),
            None => Ok(()),
        }
    }

    pub(super) fn num(&self, k: &str) -> Result<Option<f64>> {
        match self.v.get(k) {
            None => Ok(None),
            Some(Value::Num(n)) if n.is_finite() => Ok(Some(*n)),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    /// An integer from `lo` to `hi`.
    pub(super) fn int(&self, k: &str, lo: i64, hi: i64) -> Result<Option<i64>> {
        match self.num(k)? {
            None => Ok(None),
            Some(n) if n.fract() == 0.0 && n >= lo as f64 && n <= hi as f64 => Ok(Some(n as i64)),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    pub(super) fn need(&self, k: &str, lo: i64, hi: i64) -> Result<i64> {
        self.int(k, lo, hi)?
            .ok_or_else(|| self.err("missing_field", k))
    }

    pub(super) fn id(&self, k: &str) -> Result<u32> {
        Ok(self.need(k, 0, i64::from(u32::MAX >> 1))? as u32)
    }

    pub(super) fn ids(&self, k: &str) -> Result<Vec<u32>> {
        match self.v.get(k) {
            None => Ok(Vec::new()),
            Some(Value::Arr(a)) => a
                .iter()
                .map(|x| match x {
                    Value::Num(n) if n.fract() == 0.0 && *n >= 0.0 && *n < 2e9 => Ok(*n as u32),
                    _ => Err(self.err("bad_field", k)),
                })
                .collect(),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    pub(super) fn text(&self, k: &str) -> Result<Option<&str>> {
        match self.v.get(k) {
            None => Ok(None),
            Some(Value::Str(s)) => Ok(Some(s)),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    /// A point or a shift as angstrom from `kx`,`ky` (bond lengths) or `kx_a`,`ky_a`; `None` when neither is there.
    pub(super) fn point(&self, kx: &str, ky: &str) -> Result<Option<[f64; 2]>> {
        let (ax, ay) = (format!("{kx}_a"), format!("{ky}_a"));
        let bl = [self.num(kx)?, self.num(ky)?];
        let an = [self.num(&ax)?, self.num(&ay)?];
        let (vals, unit, names) = match (
            bl.iter().any(Option::is_some),
            an.iter().any(Option::is_some),
        ) {
            (false, false) => return Ok(None),
            (true, true) => {
                return Err(self.err("units_mixed", format!("{kx}/{ky} with {ax}/{ay}")))
            }
            (true, false) => (bl, BOND_A, [kx, ky]),
            (false, true) => (an, 1.0, [ax.as_str(), ay.as_str()]),
        };
        match vals {
            [Some(x), Some(y)] => Ok(Some([r4(x * unit), r4(y * unit)])),
            [None, _] => Err(self.err("missing_field", names[0])),
            _ => Err(self.err("missing_field", names[1])),
        }
    }

    pub(super) fn wedge(&self) -> Result<Option<i8>> {
        Ok(match self.num("wedge")? {
            None => None,
            Some(w) if [-1.0, 0.0, 1.0, 2.0].contains(&w) => Some(w as i8),
            Some(_) => return Err(self.err("bad_field", "wedge")),
        })
    }
}

pub(super) fn r4(v: f64) -> f64 {
    (v * 1e4).round() / 1e4 + 0.0
}
