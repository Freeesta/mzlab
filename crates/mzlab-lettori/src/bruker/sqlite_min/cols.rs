//! `Sqlite::righe_cols` and the record decoder it shares with `record`: rows with only the wanted columns decoded.

use super::*;

impl Sqlite {
    /// Rows of `t` with only the columns `colonne` (positions in the table, `None` = a column the file lacks, read as NULL), in that order and with the same values `righe` gives for them: the
    /// `INTEGER PRIMARY KEY` filled in from the rowid and the key of a `WITHOUT ROWID` table put back in its place. The other columns of the record are skipped, not decoded, and `f` gets a vector
    /// that is reused from row to row.
    pub fn righe_cols<F>(&mut self, t: &Tabella, colonne: &[Option<usize>], mut f: F) -> Result<()>
    where
        F: FnMut(&[Val]) -> Result<bool>,
    {
        // where in the record each wanted column lies
        let pos: Vec<Option<usize>> = match &t.ordine {
            Some(o) => colonne
                .iter()
                .map(|c| c.and_then(|c| o.iter().rposition(|&x| x == c)))
                .collect(),
            None => colonne.to_vec(),
        };
        let mut mascherati = vec![false; pos.iter().flatten().max().map_or(0, |&m| m + 1)];
        for &p in pos.iter().flatten() {
            mascherati[p] = true;
        }
        // a position asked twice is cloned the first time and moved the last
        let ultima: Vec<bool> = (0..pos.len())
            .map(|j| pos[j].is_none() || !pos[j + 1..].contains(&pos[j]))
            .collect();
        let alias = t.alias_rowid.filter(|_| t.ordine.is_none());
        let mut out: Vec<Val> = Vec::with_capacity(colonne.len());
        self.scorri_con(t.radice, Some(&mascherati), |rowid, v| {
            out.clear();
            for (j, p) in pos.iter().enumerate() {
                out.push(match p.and_then(|p| v.get_mut(p).map(|x| (p, x))) {
                    Some((p, Val::Null)) if alias == Some(p) => Val::Int(rowid),
                    Some((_, x)) if ultima[j] => std::mem::replace(x, Val::Null),
                    Some((_, x)) => x.clone(),
                    None => Val::Null,
                });
            }
            f(&out)
        })
    }
}

/// `record` into `out` (cleared first). With `voglio`, only the positions it marks `true` are decoded (text and blobs are not copied for the others, which are `Val::Null`); the whole record
/// is still checked, so a damaged one is refused the same way.
pub(super) fn record_in(b: &[u8], limiti: &Limiti, voglio: Option<&[bool]>, out: &mut Vec<Val>) -> Result<()> {
    out.clear();
    let (hl, o0) = varint(b).ok_or_else(|| guasto("record header"))?;
    let hl = usize::try_from(hl).map_err(|_| guasto("record header"))?;
    if hl > b.len() || hl < o0 {
        return Err(guasto("record header"));
    }
    // first the count of the serial types (and its limit), then the types again with the bodies
    let (mut o, mut n) = (o0, 0u64);
    while o < hl {
        let (_, k) = varint(&b[o..hl]).ok_or_else(|| guasto("serial type"))?;
        o += k;
        n += 1;
        limiti.voci_ok(n)?;
    }
    out.reserve(n as usize);
    let (mut o, mut corpo) = (o0, hl);
    for pos in 0..n as usize {
        let (t, k) = varint(&b[o..hl]).ok_or_else(|| guasto("serial type"))?;
        o += k;
        let len = match t {
            0 | 8 | 9 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 6,
            6 | 7 => 8,
            10 | 11 => return Err(guasto("serial type")),
            n => ((n - 12) / 2) as usize,
        };
        let s = b
            .get(corpo..corpo.checked_add(len).ok_or_else(|| guasto("record"))?)
            .ok_or_else(|| guasto("record body"))?;
        corpo += len;
        if voglio.is_some_and(|m| !m.get(pos).copied().unwrap_or(false)) {
            out.push(Val::Null);
            continue;
        }
        out.push(match t {
            0 => Val::Null,
            8 => Val::Int(0),
            9 => Val::Int(1),
            1..=6 => {
                let mut v: i64 = if s[0] & 0x80 != 0 { -1 } else { 0 };
                for &x in s {
                    v = (v << 8) | i64::from(x);
                }
                Val::Int(v)
            }
            7 => Val::Real(f64::from_be_bytes(
                s.try_into().map_err(|_| guasto("real"))?,
            )),
            n if n % 2 == 0 => Val::Blob(s.to_vec()),
            _ => Val::Text(String::from_utf8_lossy(s).into_owned()),
        });
    }
    Ok(())
}
