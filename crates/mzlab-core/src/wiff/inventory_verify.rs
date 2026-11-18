//! `wiff.verify/1` (`wiffwrite` feature): an output pair against the pairs it was assembled from, on the inventories of `inventory.rs`.

use super::inventory::{inventory, FileInv, SampleInv, Stream};
use super::SAMPLES;
use crate::error::{Error, Result};
use crate::json::Value;
use crate::jsonb::{text, write, Obj};
use crate::ole::DirMeta;
use crate::readers::{FileSet, Limits};

/// The path of a stream without the sample or method number: the same stream of two samples compares under one name.
fn relative(p: &str) -> String {
    let mut parts: Vec<&str> = p.split('/').collect();
    for seg in parts.iter_mut().take(2).skip(1) {
        if let Some(r) = seg
            .strip_prefix("Sample")
            .or_else(|| seg.strip_prefix("Method"))
            && r.parse::<u32>().is_ok()
        {
            *seg = "N";
        }
    }
    parts.join("/")
}

fn differences(
    out: &SampleInv,
    src: &SampleInv,
    dir_out: &[DirMeta],
    dir_src: &[DirMeta],
) -> Vec<String> {
    let mut d = Vec::new();
    let find = |s: &[Stream], r: &str| {
        s.iter()
            .find(|x| relative(&x.path) == r)
            .map(|x| x.sha256.clone())
    };
    for s in &src.streams {
        let r = relative(&s.path);
        match find(&out.streams, &r) {
            None => d.push(format!("missing stream {r}")),
            Some(h) if h != s.sha256 => d.push(format!("stream differs {r}")),
            _ => {}
        }
    }
    for s in &out.streams {
        if find(&src.streams, &relative(&s.path)).is_none() {
            d.push(format!("extra stream {}", relative(&s.path)));
        }
    }
    let key = |m: &DirMeta| {
        (
            m.clsid.clone(),
            m.state,
            m.created.clone(),
            m.modified.clone(),
        )
    };
    let (po, ps) = (
        format!("{SAMPLES}Sample{}/", out.number),
        format!("{SAMPLES}Sample{}/", src.number),
    );
    for m in dir_src.iter().filter(|m| m.path.starts_with(&ps)) {
        let r = relative(&m.path);
        match dir_out
            .iter()
            .find(|o| o.path.starts_with(&po) && relative(&o.path) == r)
        {
            Some(o) if key(o) != key(m) => d.push(format!("directory metadata differs {r}")),
            None => d.push(format!("missing entry {r}")),
            _ => {}
        }
    }
    d
}

/// The streams shared by all samples must equal the first source's.
fn global_differences(first: &FileInv, o: &FileInv) -> Vec<String> {
    let mut gd = Vec::new();
    for g in &first.globals {
        match o.globals.iter().find(|x| x.path == g.path) {
            None => gd.push(format!("missing global stream {}", g.path)),
            Some(x) if x.sha256 != g.sha256 => gd.push(format!("global stream differs {}", g.path)),
            _ => {}
        }
    }
    for g in &o.globals {
        if !first.globals.iter().any(|x| x.path == g.path) {
            gd.push(format!("extra global stream {}", g.path));
        }
    }
    gd
}

/// `wiff.verify/1`: the pair `out` against the pairs it was assembled from. A sample of `out` is matched to the source sample with the same section bytes
/// (SHA-256), or, for an empty injection, the same name; the match must agree on every stream and on the directory metadata of the sample. The streams shared
/// by all samples must equal those of the first source. The flag is true when everything is equal.
pub fn verify(out: FileSet, against: Vec<FileSet>, limits: &Limits) -> Result<(String, bool)> {
    let o = inventory(out, limits, false)?;
    let srcs = against
        .into_iter()
        .map(|fs| inventory(fs, limits, false))
        .collect::<Result<Vec<_>>>()?;
    if srcs.is_empty() {
        return Err(Error::new("err.file.wiff").with("detail", "no source"));
    }
    let mut rows = Vec::new();
    let mut all = o.samples.iter().all(|s| s.status == "ok");
    for s in &o.samples {
        let hit = srcs.iter().enumerate().find_map(|(i, f)| {
            f.samples
                .iter()
                .find(|c| {
                    c.status == "ok"
                        && (if s.bytes > 0 {
                            c.sha256 == s.sha256
                        } else {
                            c.bytes == 0 && c.name == s.name
                        })
                })
                .map(|c| (i, f, c))
        });
        let (src, diffs) = match hit {
            Some((i, f, c)) => (
                Obj::new()
                    .i("file", i as i64)
                    .put("name", text(&f.name))
                    .i("number", i64::from(c.number))
                    .done(),
                differences(s, c, &o.dir, &f.dir),
            ),
            None => (
                Value::Null,
                vec!["no source sample has this section".to_string()],
            ),
        };
        all &= diffs.is_empty();
        rows.push(
            Obj::new()
                .i("number", i64::from(s.number))
                .put("name", s.name.as_deref().map_or(Value::Null, text))
                .put("source", src)
                .b("equal", diffs.is_empty())
                .put(
                    "differences",
                    Value::Arr(diffs.iter().map(|x| text(x)).collect()),
                )
                .done(),
        );
    }
    let Some(first) = srcs.first() else {
        return Err(Error::new("err.file.wiff").with("detail", "no source"));
    };
    let gd = global_differences(first, &o);
    all &= gd.is_empty();
    let v = Obj::new()
        .put("schema", text("wiff.verify/1"))
        .put("output", text(&o.name))
        .put("samples", Value::Arr(rows))
        .put(
            "global_differences",
            Value::Arr(gd.iter().map(|x| text(x)).collect()),
        )
        .b("equal", all)
        .done();
    Ok((write(&v), all))
}
