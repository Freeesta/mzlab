//! The rows of the scan table of an index (`SCAN_COLUMNS`, `docs/agenti/mzidx.md`) as `Scan`.

use crate::idxread::{Scan, KINDS};
use crate::json::Value;
use crate::lettori::Posizione;

/// `[membro, offset, lunghezza]` of a row of version 2, or `null`.
fn posizione(v: &Value) -> Option<Option<Posizione>> {
    if v.is_null() {
        return Some(None);
    }
    let n = |i: usize| v.at(i).and_then(Value::f64).filter(|x| *x >= 0.0);
    Some(Some(Posizione {
        membro: u16::try_from(n(0)? as u64).ok()?,
        offset: n(1)? as u64,
        lunghezza: u32::try_from(n(2)? as u64).ok()?,
    }))
}

/// The scan of a row of the index: `start,end` (version 1) or `[membro,offset,lunghezza] | null, k` (version 2, `nativo`) in its first two columns.
pub(crate) fn scan_of_row(r: &Value, nativo: bool) -> Option<Scan> {
    let f = |i: usize| r.at(i).and_then(Value::f64);
    let s = |i: usize| r.at(i).and_then(Value::str).map(String::from);
    let iso = r.at(11).filter(|v| !v.is_null()).map(|v| {
        (
            v.at(0).and_then(Value::f64).unwrap_or(0.0),
            v.at(1).and_then(Value::f64).unwrap_or(0.0),
        )
    });
    let path = r.at(16).filter(|v| !v.is_null()).map(|v| {
        v.arr()
            .iter()
            .map(|x| {
                (
                    x.at(0).and_then(Value::f64).unwrap_or(0.0),
                    x.at(1).and_then(Value::str).unwrap_or("").to_string(),
                    x.at(2).and_then(Value::f64).unwrap_or(0.0),
                )
            })
            .collect()
    });
    let (start, end, pos, k) = if nativo {
        (0, 0, posizione(r.at(0)?)?, Some(f(1)? as usize))
    } else {
        (f(0)? as u64, f(1)? as u64, None, None)
    };
    Some(Scan {
        start,
        end,
        posizione: pos,
        k,
        level: f(2)? as u8,
        rt: f(3)?,
        polarity: f(4)? as i8,
        tic: f(5)?,
        precursor: f(6),
        collision_energy: f(7),
        filter: s(8).unwrap_or_default(),
        profile: r.at(9).and_then(Value::bool).unwrap_or(false),
        parent: f(10).map(|v| v as i64),
        iso,
        act: s(12),
        res: f(13),
        an: s(14),
        pint: f(15),
        path,
        charge: f(18).map(|v| v as i64),
        native: s(19).unwrap_or_default(),
        kind: r
            .at(20)
            .and_then(Value::str)
            .and_then(|k| KINDS.iter().position(|x| *x == k))
            .map_or(0, |i| i as u8 + 1),
    })
}
