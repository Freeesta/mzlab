//! The block `interp` of the `.mzfeat`: the meta key `interp` (parameters, calibration, library of the tree, registry of the filters) and three arrays,
//! `interp_id` (id of every interpreted row, ascending), `interp_off` (offsets of the text of each row, one more than the rows) and `interp` (the JSON of
//! the rows, one after the other). `append` writes a copy of a `.mzfeat` that carries the block (the old block, if any, is replaced); a file without
//! the block reads as before.

use mzlab_core::budget::WriteAt;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::Value;

use crate::store::{bad, ALIGN, MAGIC, PIECE};
use crate::store_read::{open, Opened};

const OWN: [&str; 3] = ["interp_id", "interp_off", "interp"];

fn width(dtype: &str) -> Result<usize> {
    Ok(match dtype {
        "f64" => 8,
        "f32" | "u32" => 4,
        "u16" => 2,
        "u8" | "i8" => 1,
        _ => return Err(bad()),
    })
}

/// Sequential writer with the alignment rule of the container.
struct Seq<'a, W: WriteAt> {
    sink: &'a mut W,
    pos: u64,
}

impl<W: WriteAt> Seq<'_, W> {
    fn put(&mut self, b: &[u8]) -> Result<()> {
        self.sink.write_at(self.pos, b)?;
        self.pos += b.len() as u64;
        Ok(())
    }
    fn pad(&mut self) -> Result<()> {
        let k = (ALIGN - (self.pos as usize) % ALIGN) % ALIGN;
        self.put(&[0u8; ALIGN][..k])
    }
}

/// An array to write: name, dtype, count, and where its bytes come from.
enum Src<'a> {
    Old(u64),
    Bytes(&'a [u8]),
}

type Item<'a> = (String, String, usize, Src<'a>);

/// The ids, the end offsets (n + 1, the first is 0) and the bytes of the JSON of every row.
fn pack(rows: &[(u32, String)]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (mut ids, mut offs, mut data) = (Vec::new(), vec![0u8; 4], Vec::new());
    for (id, json) in rows {
        ids.extend(id.to_le_bytes());
        data.extend(json.as_bytes());
        offs.extend((data.len() as u32).to_le_bytes());
    }
    (ids, offs, data)
}

/// The arrays of the old file except ours, and `evid` apart: it stays the last one.
fn old_arrays<'a>(o: &Opened) -> Result<(Vec<Item<'a>>, Option<Item<'a>>)> {
    let (mut list, mut evid) = (Vec::new(), None);
    for e in o.meta.get("arrays").map(Value::arr).unwrap_or(&[]) {
        let name = e.get("name").and_then(Value::str).ok_or_else(bad)?;
        let dtype = e.get("dtype").and_then(Value::str).ok_or_else(bad)?;
        let get = |k: &str| {
            e.get(k)
                .and_then(Value::f64)
                .map(|v| v as u64)
                .ok_or_else(bad)
        };
        let item = (
            name.to_string(),
            dtype.to_string(),
            get("count")? as usize,
            Src::Old(o.start + get("offset")?),
        );
        if OWN.contains(&name) {
            continue;
        }
        if name == "evid" {
            evid = Some(item);
        } else {
            list.push(item);
        }
    }
    Ok((list, evid))
}

/// The directory of the meta (JSON text) and where each array starts, aligned to `ALIGN`.
fn directory(list: &[Item]) -> Result<(String, Vec<usize>)> {
    let (mut dir, mut at, mut places) = (String::new(), 0usize, Vec::new());
    for (i, (name, dtype, count, _)) in list.iter().enumerate() {
        at = at.div_ceil(ALIGN) * ALIGN;
        dir.push_str(&format!(
            "{}{{\"name\":\"{name}\",\"dtype\":\"{dtype}\",\"count\":{count},\"offset\":{at}}}",
            if i > 0 { "," } else { "" }
        ));
        places.push(at);
        at += count * width(dtype)?;
    }
    Ok((dir, places))
}

/// Writes to `sink` the `.mzfeat` of `src` (`size` bytes) with the block: `meta` is the JSON of the key `interp`, `rows` the `(id, JSON)` of every row.
pub fn append<R: ReadAt, W: WriteAt>(
    src: &mut R,
    size: u64,
    sink: &mut W,
    meta: &str,
    rows: &[(u32, String)],
) -> Result<()> {
    let o = open(src, size, "table")?;
    let mut h = [0u8; 16];
    src.read_at(0, &mut h)?;
    let n = u64::from_le_bytes(h[8..].try_into().unwrap()) as usize;
    let mut text = vec![0u8; n];
    src.read_at(16, &mut text)?;
    let text = String::from_utf8(text).map_err(|_| bad())?;
    let cut = text.rfind(",\"arrays\":[").ok_or_else(bad)?;
    let head = &text[..cut];
    let head = head.rfind(",\"interp\":").map_or(head, |i| &head[..i]);

    let (ids, offs, data) = pack(rows);
    let own: [(&str, &str, usize, Src); 3] = [
        ("interp_id", "u32", rows.len(), Src::Bytes(&ids)),
        ("interp_off", "u32", rows.len() + 1, Src::Bytes(&offs)),
        ("interp", "u8", data.len(), Src::Bytes(&data)),
    ];
    let (mut list, evid) = old_arrays(&o)?;
    list.extend(
        own.into_iter()
            .map(|(a, b, c, d)| (a.to_string(), b.to_string(), c, d)),
    );
    list.extend(evid);
    let (dir, places) = directory(&list)?;
    let meta_text = format!("{head},\"interp\":{meta},\"arrays\":[{dir}]}}");
    let mut seq = Seq { sink, pos: 0 };
    seq.put(MAGIC)?;
    seq.put(&(meta_text.len() as u64).to_le_bytes())?;
    seq.put(meta_text.as_bytes())?;
    seq.pad()?;
    let mut buf = vec![0u8; PIECE * 8];
    for (item, _) in list.iter().zip(&places) {
        seq.pad()?;
        let (_, dtype, count, from) = item;
        match from {
            Src::Bytes(b) => seq.put(b)?,
            Src::Old(off) => {
                let (mut done, total) = (0usize, count * width(dtype)?);
                while done < total {
                    let k = (total - done).min(buf.len());
                    src.read_at(off + done as u64, &mut buf[..k])?;
                    seq.put(&buf[..k])?;
                    done += k;
                }
            }
        }
    }
    seq.pad()
}

/// The meta key `interp` of a `.mzfeat` and the ids of the interpreted rows; `None` for a file without the block.
pub fn read_meta<R: ReadAt>(r: &mut R, size: u64) -> Result<Option<(Value, Vec<u32>)>> {
    let o = open(r, size, "table")?;
    let Some(m) = o.meta.get("interp").cloned() else {
        return Ok(None);
    };
    let ids = o.col(r, size, "interp_id", "u32", None, u32::from_le_bytes)?;
    Ok(Some((m, ids)))
}

/// The JSON of the row with id `id`, `None` when the row was not interpreted or the file has no block.
pub fn read_row<R: ReadAt>(r: &mut R, size: u64, id: u32) -> Result<Option<String>> {
    let Some((_, ids)) = read_meta(r, size)? else {
        return Ok(None);
    };
    let Ok(k) = ids.binary_search(&id) else {
        return Ok(None);
    };
    let o = open(r, size, "table")?;
    let off = o.col(
        r,
        size,
        "interp_off",
        "u32",
        Some((k, k + 2)),
        u32::from_le_bytes,
    )?;
    let (a, b) = (off[0] as usize, off[1] as usize);
    let bytes = o.col(r, size, "interp", "u8", Some((a, b)), |x: [u8; 1]| x[0])?;
    String::from_utf8(bytes).map(Some).map_err(|_| bad())
}
