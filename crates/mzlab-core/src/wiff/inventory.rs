//! Read-only inventory of a `.wiff` + `.wiff.scan` pair (`wiffwrite` feature): per sample its section of the `.scan`, its streams, its index and a status, per file
//! its container and the streams shared by all samples, all with SHA-256. `list` prints it (`wiff.list/1`); `verify` compares an output pair against the
//! sources it was assembled from (`wiff.verify/1`, in `inventory_verify.rs`). Nothing is decoded or rewritten: the bytes are hashed as the file holds them.

use sha2::{Digest, Sha256};

use super::reader::{build, member, Part};
use super::{idx, samples, Wiff, SAMPLES};
use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::Value;
use crate::jsonb::{int, text, write, Obj};
use crate::ole::{DirMeta, Info, Ole};
use crate::readers::{FileSet, Limits};

/// Bytes of the head of a section kept as hex: the marker, the sample number and what the reader skips.
const LEAD: u64 = 44;
const LEAD_BYTES: usize = 44;

pub struct Stream {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

pub struct SampleInv {
    pub number: u32,
    pub name: Option<String>,
    pub method: u32,
    pub status: &'static str,
    pub reason: Option<String>,
    pub scans: usize,
    pub start: Option<u64>,
    pub bytes: u64,
    pub used: u64,
    pub sha256: String,
    pub lead: String,
    pub rows: usize,
    pub zero_rows: usize,
    pub streams: Vec<Stream>,
}

pub struct FileInv {
    pub name: String,
    pub sha256: String,
    pub scan_bytes: u64,
    pub scan_sha256: Option<String>,
    pub info: Info,
    pub head: u64,
    pub unindexed_skipped: usize,
    pub globals: Vec<Stream>,
    pub samples: Vec<SampleInv>,
    pub dir: Vec<DirMeta>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

fn range(scan: &mut dyn ReadAt, start: u64, end: u64) -> (String, Vec<u8>) {
    let (mut h, mut at, mut lead) = (Sha256::new(), start, Vec::new());
    while at < end {
        let n = (end - at).min(1 << 20) as usize;
        let mut b = vec![0u8; n];
        if scan.read_at(at, &mut b).is_err() {
            break;
        }
        if at == start {
            lead = b.get(..n.min(LEAD_BYTES)).unwrap_or_default().to_vec();
        }
        h.update(&b);
        at += n as u64;
    }
    (hex(&h.finalize()), lead)
}

fn stream(ole: &Ole, path: &str) -> Stream {
    let b = ole.read(path).unwrap_or_default();
    Stream {
        path: path.to_string(),
        bytes: b.len() as u64,
        sha256: sha(&b),
    }
}

fn refusal(w: &Wiff) -> Option<String> {
    if let Some(e) = &w.failure {
        return Some(e.key.to_string());
    }
    w.experiments
        .iter()
        .any(|e| matches!(e.code, Some(8 | 9)))
        .then(|| "tof".to_string())
}

/// The inventory of the pair `fs`. `full` also hashes the whole `.wiff.scan`.
pub fn inventory(mut fs: FileSet, limits: &Limits, full: bool) -> Result<FileInv> {
    let (name, ole, si) = super::listing::container_of(&mut fs, limits)?;
    let sm = member(&mut fs, si)?;
    let scan_len = sm.dim;
    let mut scan = std::mem::replace(&mut sm.dati, Box::new(Vec::<u8>::new()));
    let table = ole
        .read(&format!("{SAMPLES}SampleIdxTable"))
        .map(|t| idx::sections(&t))
        .unwrap_or_default();
    let starts: Vec<u64> = table.iter().filter_map(|s| s.1).collect();
    let (mut parts, mut invs) = (Vec::new(), Vec::new());
    for s in samples(&ole) {
        let (inv, part) = sample_inv(&ole, &name, &mut scan, scan_len, s, &table, &starts)?;
        invs.push(inv);
        parts.push(part);
    }
    // sections after the last indexed one: reported, never silently dropped
    let known: Vec<&Wiff> = parts.iter().map(|p| &p.w).collect();
    let (after, last) = super::unindexed::index_end(&ole, &known);
    let found = super::unindexed::recover(&mut *scan, scan_len, &known, after, last);
    for w in &found.samples {
        let a = w.records.first().map_or(after, |r| r.at.saturating_sub(24));
        let (sha256, lead) = range(&mut *scan, a, scan_len);
        invs.push(SampleInv {
            number: w.sample,
            name: None,
            method: 0,
            status: "unindexed",
            reason: Some("section not in SampleIdxTable".into()),
            scans: w.records.len(),
            start: Some(a),
            bytes: scan_len - a,
            used: scan_len - a,
            sha256,
            lead: hex(&lead),
            rows: 0,
            zero_rows: 0,
            streams: Vec::new(),
        });
    }
    let owned = |p: &str| {
        p.strip_prefix(SAMPLES)
            .is_some_and(|r| r.starts_with("Sample") && r.contains('/'))
            || p.strip_prefix("MethodSubtree/")
                .is_some_and(|r| r.starts_with("Method") && r.contains('/'))
    };
    let globals = ole
        .paths_list()
        .filter(|p| !owned(p))
        .map(|p| stream(&ole, p))
        .collect();
    Ok(FileInv {
        name,
        sha256: ole.sha256_hex(),
        scan_bytes: scan_len,
        scan_sha256: full.then(|| range(&mut *scan, 0, scan_len).0),
        info: ole.info(),
        head: starts.iter().copied().min().unwrap_or(scan_len),
        unindexed_skipped: found.skipped,
        globals,
        samples: invs,
        dir: ole.dir_meta(),
    })
}

/// The streams of the sample (`own`) and of its method (`meth`), each with its SHA-256.
fn sample_streams(ole: &Ole, own: &str, meth: &str) -> Vec<Stream> {
    ole.paths_list()
        .filter(|p| p.starts_with(own) || p.starts_with(meth))
        .map(|p| stream(ole, p))
        .collect()
}

/// One indexed sample: its section in the `.scan`, its streams, its index rows and its status.
fn sample_inv(
    ole: &Ole,
    name: &str,
    scan: &mut Box<dyn ReadAt>,
    scan_len: u64,
    s: u32,
    table: &[(u32, Option<u64>)],
    starts: &[u64],
) -> Result<(SampleInv, Part)> {
    let w = Wiff::open_or_refuse(ole, name, scan_len, s)?;
    let mut part = Part::new(w);
    let mut refused = refusal(&part.w);
    if refused.is_none() {
        // a block the reader cannot take (a TOF block) sets the failure while the blocks are read
        build(&mut part, scan, scan_len, 0)?;
        refused = refusal(&part.w);
    }
    let w = &part.w;
    let start = table.iter().find(|r| r.0 == s).and_then(|r| r.1);
    let (end, used) = match start {
        Some(a) => {
            let used = w
                .records
                .iter()
                .map(|r| r.data_end())
                .max()
                .unwrap_or(a + LEAD);
            (
                starts
                    .iter()
                    .copied()
                    .filter(|&x| x > a)
                    .min()
                    .unwrap_or(scan_len)
                    .min(scan_len),
                used.max(a),
            )
        }
        None => (0, 0),
    };
    let (sha256, lead) = start.map_or((sha(&[]), Vec::new()), |a| range(&mut **scan, a, end));
    let rows = ole
        .read(&format!("{SAMPLES}Sample{s}/Idx"))
        .map(|b| idx::parse(&b))
        .unwrap_or_default();
    let method = super::method_no(ole, s);
    let (own, meth) = (
        format!("{SAMPLES}Sample{s}/"),
        format!("MethodSubtree/Method{method}/"),
    );
    let streams = sample_streams(ole, &own, &meth);
    let (status, reason) = match refused {
        Some(r) => ("refused", Some(r)),
        None if w.records.iter().any(|r| r.cut(scan_len)) => ("truncated", Some("scan".into())),
        None if part.damaged > 0 || w.unplaced > 0 => ("damaged", Some("blocks".into())),
        None => ("ok", None),
    };
    let inv = SampleInv {
        number: s,
        name: ole
            .read(&format!("{own}SampleDABE/DATA"))
            .and_then(|b| crate::method_wiff::counted_strings(&b).into_iter().next()),
        method,
        status,
        reason,
        scans: part.scans.len(),
        start,
        bytes: end.saturating_sub(start.unwrap_or(0)),
        used: used.saturating_sub(start.unwrap_or(0)),
        sha256,
        lead: hex(&lead),
        rows: rows.len(),
        zero_rows: rows.iter().filter(|r| r.zeroed()).count(),
        streams,
    };
    Ok((inv, part))
}

fn streams_json(v: &[Stream]) -> Value {
    Value::Arr(
        v.iter()
            .map(|s| {
                Obj::new()
                    .put("path", text(&s.path))
                    .i("bytes", s.bytes as i64)
                    .put("sha256", text(&s.sha256))
                    .done()
            })
            .collect(),
    )
}

fn sample_json(s: &SampleInv) -> Value {
    let opt = |v: &Option<String>| v.as_deref().map_or(Value::Null, text);
    Obj::new()
        .i("number", i64::from(s.number))
        .put("name", opt(&s.name))
        .i("method", i64::from(s.method))
        .put("status", text(s.status))
        .put("reason", opt(&s.reason))
        .i("scans", s.scans as i64)
        .put(
            "section",
            Obj::new()
                .put("start", s.start.map_or(Value::Null, |a| int(a as i64)))
                .i("bytes", s.bytes as i64)
                .i("slack", s.bytes.saturating_sub(s.used) as i64)
                .put("sha256", text(&s.sha256))
                .put("lead_hex", text(&s.lead))
                .done(),
        )
        .put(
            "idx",
            Obj::new()
                .i("rows", s.rows as i64)
                .i("zero_rows", s.zero_rows as i64)
                .done(),
        )
        .put("streams", streams_json(&s.streams))
        .done()
}

fn file_json(f: &FileInv, inventory: bool) -> Value {
    let mut o = Obj::new()
        .put("name", text(&f.name))
        .put("sha256", text(&f.sha256))
        .put(
            "container",
            Obj::new()
                .i("version", i64::from(f.info.version))
                .i("sector_bytes", f.info.sector as i64)
                .i("mini_sector_bytes", f.info.mini_sector as i64)
                .i("mini_cutoff", i64::from(f.info.mini_cutoff))
                .i("bytes", f.info.bytes as i64)
                .done(),
        )
        .put(
            "scan",
            Obj::new()
                .i("bytes", f.scan_bytes as i64)
                .put("sha256", f.scan_sha256.as_deref().map_or(Value::Null, text))
                .i("head_bytes", f.head as i64)
                .i("unindexed_skipped", f.unindexed_skipped as i64)
                .done(),
        )
        .put("global_streams", streams_json(&f.globals))
        .put(
            "samples",
            Value::Arr(f.samples.iter().map(sample_json).collect()),
        );
    if inventory {
        o.set("directory", dir_json(&f.dir));
    }
    o.done()
}

fn dir_json(d: &[DirMeta]) -> Value {
    Value::Arr(
        d.iter()
            .map(|e| {
                Obj::new()
                    .put("path", text(&e.path))
                    .put("kind", text(if e.stream { "stream" } else { "storage" }))
                    .i("bytes", e.size as i64)
                    .put("clsid", text(&e.clsid))
                    .i("state", i64::from(e.state))
                    .put("created", text(&e.created))
                    .put("modified", text(&e.modified))
                    .done()
            })
            .collect(),
    )
}

/// `wiff.list/1` of the pairs `sets` (a refused sample is listed, never hidden). The flag is true when any sample is not `ok`.
pub fn list(sets: Vec<FileSet>, limits: &Limits, full: bool) -> Result<(String, bool)> {
    let files = sets
        .into_iter()
        .map(|fs| inventory(fs, limits, full))
        .collect::<Result<Vec<_>>>()?;
    let bad = files
        .iter()
        .any(|f| f.samples.iter().any(|s| s.status != "ok"));
    let v = Obj::new()
        .put("schema", text("wiff.list/1"))
        .put(
            "files",
            Value::Arr(files.iter().map(|f| file_json(f, full)).collect()),
        )
        .b("all_ok", !bad)
        .done();
    Ok((write(&v), bad))
}

pub use super::inventory_verify::verify;
