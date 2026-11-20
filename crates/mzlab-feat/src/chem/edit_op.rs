//! One operation of an edit patch as it is read: its fields with their types, ranges and units, each failure naming the operation.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use super::draw::MAX_ATOMS;

/// Angstrom per bond length of the drawing.
const BOND_A: f64 = 1.5;

/// The largest id an atom can have in a patch.
pub(super) const MAX_ID: u32 = u32::MAX >> 1;

/// The text of a patch echoed in an error, at most 64 bytes.
pub(super) fn clip(s: &str) -> &str {
    let mut n = s.len().min(64);
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    &s[..n]
}

/// `a-b`, the name of a bond in a failure or a choice.
#[inline(never)]
pub(super) fn pair(a: u32, b: u32) -> String {
    format!("{a}-{b}")
}

/// The failure of operation `index` (0-based): `op` is its name, `reason` a stable word, `detail` what it was about.
pub(super) fn fail(index: usize, op: &str, reason: &'static str, detail: &str) -> Error {
    Error::new("err.mol.draw.edit")
        .with("index", index)
        .with("op", clip(op))
        .with("reason", reason)
        .with("detail", clip(detail))
}

/// One operation with the means to fail with its name and index.
pub(super) struct Op<'a> {
    pub(super) i: usize,
    pub(super) name: &'a str,
    pub(super) v: &'a Value,
}

impl Op<'_> {
    pub(super) fn err(&self, reason: &'static str, detail: &str) -> Error {
        fail(self.i, self.name, reason, detail)
    }

    /// The failure about atom `id`.
    pub(super) fn err_id(&self, reason: &'static str, id: u32) -> Error {
        self.err(reason, &id.to_string())
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
        Ok(self.need(k, 0, i64::from(MAX_ID))? as u32)
    }

    /// An id (a whole number from 0 to `MAX_ID`) taken from the list `k`.
    pub(super) fn id_in(&self, x: &Value, k: &str) -> Result<u32> {
        match x {
            Value::Num(n) if n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(MAX_ID) => {
                Ok(*n as u32)
            }
            _ => Err(self.err("bad_field", k)),
        }
    }

    /// The list `k`, empty when it is absent; longer than `max` is a bad field (a patch never needs more).
    pub(super) fn list(&self, k: &str, max: usize) -> Result<&[Value]> {
        match self.v.get(k) {
            None => Ok(&[]),
            Some(Value::Arr(a)) if a.len() <= max => Ok(a),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    pub(super) fn ids(&self, k: &str) -> Result<Vec<u32>> {
        self.list(k, MAX_ATOMS)?
            .iter()
            .map(|x| self.id_in(x, k))
            .collect()
    }

    pub(super) fn text(&self, k: &str) -> Result<Option<&str>> {
        match self.v.get(k) {
            None => Ok(None),
            Some(Value::Str(s)) => Ok(Some(s)),
            Some(_) => Err(self.err("bad_field", k)),
        }
    }

    /// A point or a shift as angstrom from `n[0]`,`n[1]` (bond lengths) or `n[2]`,`n[3]` (angstrom); `None` when none of them is there.
    pub(super) fn point(&self, n: [&str; 4]) -> Result<Option<[f64; 2]>> {
        let bl = [self.num(n[0])?, self.num(n[1])?];
        let an = [self.num(n[2])?, self.num(n[3])?];
        let (vals, unit, k) = match (
            bl.iter().any(Option::is_some),
            an.iter().any(Option::is_some),
        ) {
            (false, false) => return Ok(None),
            (true, true) => return Err(self.err("units_mixed", n[0])),
            (true, false) => (bl, BOND_A, 0),
            (false, true) => (an, 1.0, 2),
        };
        match vals {
            [Some(x), Some(y)] => Ok(Some([r4(x * unit), r4(y * unit)])),
            [None, _] => Err(self.err("missing_field", n[k])),
            _ => Err(self.err("missing_field", n[k + 1])),
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
