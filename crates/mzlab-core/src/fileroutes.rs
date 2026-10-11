//! The routes that read the chromatograms and the header of an mzML (`/api/mrm`, `/api/fileinfo`): `app.mrm` / `Item.mrm` and
//! `app.fileinfo` / `Item.file_info` of `mzlab/app.py` and `mzlab/explore.py`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::mzmlhead::{Chrom, Mzml};
use crate::routes::Routes;

fn strs(o: &mut Out, v: &[String]) {
    o.raw("[");
    for (i, s) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

/// The SRM transitions of one file: `[{id, q1, q3, ce, name, dwell, rt, y}]` (rt rounded to 4 decimals, intensity to 1).
pub fn mrm_transitions<M: ReadAt>(m: &mut Mzml<M>, chroms: &[Chrom]) -> Result<String> {
    let mut o = Out::default();
    o.raw("[");
    let mut first = true;
    for c in chroms.iter().filter(|c| c.kind == "srm") {
        let (t, y) = m.chromatogram(c)?;
        o.raw(if first { "" } else { "," })
            .raw("{\"id\":")
            .int(c.index as i64)
            .raw(",\"q1\":")
            .opt(c.q1)
            .raw(",\"q3\":")
            .opt(c.q3)
            .raw(",\"ce\":")
            .opt(c.ce);
        o.raw(",\"name\":")
            .str(&c.name)
            .raw(",\"dwell\":")
            .opt(c.dwell)
            .raw(",\"rt\":")
            .rounded(&t, 4)
            .raw(",\"y\":")
            .rounded(&y, 1)
            .raw("}");
        first = false;
    }
    o.raw("]");
    Ok(o.0)
}

/// `/api/mrm` for the files `ks` (each with its transitions already built).
pub fn mrm_files(files: &[(i64, String)]) -> String {
    let mut o = Out::default();
    o.raw("{\"files\":[");
    for (i, (k, tr)) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"k\":")
            .int(*k)
            .raw(",\"transitions\":")
            .raw(tr)
            .raw("}");
    }
    o.raw("]}");
    o.0
}

/// Values of `attr="..."` inside the first tag that starts with `tag`, or inside every such tag.
fn tag_attrs(head: &str, tag: &str, attr: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = head;
    let key = format!("{attr}=\"");
    while let Some(p) = rest.find(tag) {
        let t = &rest[p + tag.len()..];
        rest = t;
        if t.chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        let end = t.find('>').unwrap_or(t.len());
        if let Some(a) = t[..end].find(&key) {
            let v = &t[a + key.len()..end.max(a + key.len())];
            if let Some(e) = v.find('"') {
                out.push(v[..e].to_string());
            }
        }
    }
    out
}

/// `/api/fileinfo`: what the mzML says about the file and the instrument, the scan types and the m/z range of the survey and product ions.
/// `head` is the start of the file up to `<run ` plus 3000 bytes.
pub fn fileinfo<R: ReadAt, M: ReadAt>(
    routes: &mut Routes<R>,
    m: &mut Mzml<M>,
    file: &str,
    head: &str,
) -> Result<String> {
    let md = m.metadata()?;
    let ts = tag_attrs(head, "<run", "startTimeStamp").into_iter().next();
    let proc: Vec<String> = {
        let mut v = vec![];
        let mut rest = head;
        while let Some(p) = rest.find("<dataProcessing id=\"") {
            let t = &rest[p + 20..];
            if let Some(e) = t.find('"') {
                v.push(t[..e].to_string());
            }
            rest = t;
        }
        v
    };
    let mut conv = tag_attrs(head, "<processingMethod", "softwareRef");
    conv.sort();
    conv.dedup();
    let scans = &routes.idx.scans;
    let rt = |f: fn(f64, f64) -> f64, init: f64| {
        if scans.is_empty() {
            0.0
        } else {
            scans.iter().map(|s| s.rt).fold(init, f)
        }
    };
    let (rmin, rmax) = (
        round(rt(f64::min, f64::INFINITY), 3),
        round(rt(f64::max, f64::NEG_INFINITY), 3),
    );
    let mut levels: Vec<(u8, usize)> = vec![];
    for s in scans {
        match levels.iter_mut().find(|(l, _)| *l == s.level) {
            Some(x) => x.1 += 1,
            None => levels.push((s.level, 1)),
        }
    }
    levels.sort();
    let n = scans.len();
    let filters = routes.filters()?;
    let filters = filters
        .strip_prefix("{\"filters\":")
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or("[]")
        .to_string();
    let mut ranges = vec![];
    for lv in [1u8, 2u8] {
        if let Some(si) = routes.idx.section_of(lv, None) {
            if let Some((a, b)) = routes.idx.mz_range(si)? {
                ranges.push((lv, round(a, 4), round(b, 4)));
            }
        }
    }
    let mut o = Out::default();
    o.raw("{\"file\":")
        .str(file)
        .raw(",\"instrument\":")
        .str(&md.instrument)
        .raw(",\"serial\":")
        .str(&md.serial)
        .raw(",\"components\":");
    strs(&mut o, &md.components);
    o.raw(",\"analyzers\":");
    strs(&mut o, &md.analyzers);
    o.raw(",\"software\":");
    strs(&mut o, &md.software);
    o.raw(",\"source_files\":");
    strs(&mut o, &md.source_files);
    o.raw(",\"conversion\":");
    strs(&mut o, &conv);
    o.raw(",\"processing\":");
    strs(&mut o, &proc);
    o.raw(",\"start\":")
        .str(&ts.unwrap_or_default())
        .raw(",\"scans\":")
        .int(n as i64)
        .raw(",\"rt\":[")
        .num(rmin)
        .raw(",")
        .num(rmax)
        .raw("],\"levels\":{");
    for (i, (l, c)) in levels.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(&l.to_string())
            .raw(":")
            .int(*c as i64);
    }
    o.raw("},\"filters\":").raw(&filters);
    if !ranges.is_empty() {
        o.raw(",\"mz_range\":{");
        for (i, (l, a, b)) in ranges.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .str(&l.to_string())
                .raw(":[")
                .num(*a)
                .raw(",")
                .num(*b)
                .raw("]");
        }
        o.raw("}");
    }
    o.raw("}");
    Ok(o.0)
}
