//! Writes `scandump/1` (docs/agenti/scandump.md) for a Thermo `.raw` from our reader, with the functions the `thermo` module uses (`opentfraw::scan_metadata`, `rawlettore`):
//! the twin of the dump RFR writes on the Mac, compared by `tools/rosina/scandump_compare.py`. A tool of the test bench: it never enters a .wasm.
//!   thermo_scandump FILE.raw OUT.scandump.jsonl.gz [--full] [--text] [--downstream]
use std::io::{Cursor, Write};

mod downstream;
use std::time::Instant;

use flate2::{write::GzEncoder, Compression};
use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::rawlettore;
use opentfraw::generic_data::GenericValue;
use opentfraw::{RawFileReader, ScanMetadata, ScanParams};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn text_of(v: &GenericValue) -> Option<String> {
    use GenericValue as G;
    let t = match v {
        G::Gap => return None,
        G::Int8(x) => x.to_string(),
        G::Bool(x) => x.to_string(),
        G::UInt8(x) => x.to_string(),
        G::Int16(x) => x.to_string(),
        G::UInt16(x) => x.to_string(),
        G::Int32(x) => x.to_string(),
        G::UInt32(x) => x.to_string(),
        G::Float32(x) => x.to_string(),
        G::Float64(x) => x.to_string(),
        G::String(x) => x.trim().to_string(),
    };
    (!t.is_empty()).then_some(t)
}

/// A label of a generic record without its trailing colon; separator rows (`===`) have none.
fn label_of(l: &str) -> Option<String> {
    let t = l.trim().trim_end_matches(':').trim();
    (!t.is_empty() && !t.contains("===")).then(|| t.to_string())
}

fn values_of(rec: &opentfraw::generic_data::GenericRecord) -> Map<String, Value> {
    let mut m = Map::new();
    for (l, v) in &rec.values {
        if let (Some(k), Some(t)) = (label_of(l), text_of(v)) {
            m.insert(k, Value::String(t));
        }
    }
    m
}

pub(crate) fn put<T: Into<Value>>(m: &mut Map<String, Value>, k: &str, v: Option<T>) {
    if let Some(v) = v {
        m.insert(k.into(), v.into());
    }
}

/// The reactions the reader exposes today: one element, from the same values the mzML writer uses. When the reader gives one per reaction this is the line to widen.
fn reactions_of(meta: &ScanMetadata, params: Option<&ScanParams<'_>>) -> Vec<Value> {
    let Some(p) = meta.precursor.as_ref() else {
        return Vec::new();
    };
    let mut r = Map::new();
    put(&mut r, "precursor", p.target_mz);
    put(&mut r, "iso_width", p.isolation_width);
    put(&mut r, "activation", p.activation.map(|a| format!("{a:?}")));
    put(&mut r, "energy", p.collision_energy);
    // the unit is known only when the energy comes from the trailer
    let from_trailer = params
        .and_then(|q| q.activation_energy())
        .is_some_and(|e| e > 0.0);
    if p.collision_energy.is_some() && from_trailer {
        r.insert(
            "energy_unit".into(),
            if p.ce_is_nce { "NCE" } else { "eV" }.into(),
        );
    }
    put(
        &mut r,
        "supplemental",
        params.and_then(|q| q.supplemental_activation_energy()),
    );
    if r.is_empty() {
        return Vec::new();
    }
    vec![Value::Object(r)]
}

fn scan_row(
    raw: &RawFileReader,
    src: &mut Cursor<&[u8]>,
    idx: u32,
    n_peaks: usize,
) -> Option<(Value, ScanMetadata)> {
    let first = raw.run_header.sample_info.first_scan_number;
    let params = raw.scan_params(src, first + idx);
    let meta = opentfraw::mzml::scan_metadata_with(raw, idx, params.clone_params())?;
    let mut m = Map::new();
    m.insert("type".into(), "scan".into());
    m.insert("n".into(), meta.scan_number.into());
    m.insert("rt".into(), meta.retention_time_min.into());
    m.insert("level".into(), meta.ms_level.into());
    put(
        &mut m,
        "polarity",
        meta.polarity.map(|p| {
            if p == opentfraw::Polarity::Positive {
                "+"
            } else {
                "-"
            }
        }),
    );
    put(&mut m, "analyzer", meta.analyzer.map(|a| format!("{a:?}")));
    put(
        &mut m,
        "mode",
        meta.scan_mode.map(|s| format!("{s:?}").to_lowercase()),
    );
    put(&mut m, "filter", meta.filter.clone());
    let ev = raw.scan_events.get(idx as usize);
    put(
        &mut m,
        "scan_type",
        ev.as_ref()
            .and_then(|e| e.preamble.scan_type())
            .map(|t| format!("{t:?}").to_lowercase()),
    );
    for (k, v) in [("segment", meta.scan_segment), ("event", meta.scan_event)] {
        if v != 0xFFFF {
            m.insert(k.into(), v.into());
        }
    }
    put(
        &mut m,
        "master",
        params
            .as_ref()
            .and_then(|p| p.master_scan_number())
            .filter(|&x| x > 0),
    );
    m.insert("tic".into(), meta.total_ion_current.into());
    m.insert("bp_mz".into(), meta.base_peak_mz.into());
    m.insert("bp_int".into(), meta.base_peak_intensity.into());
    m.insert("mz_low".into(), meta.low_mz.into());
    m.insert("mz_high".into(), meta.high_mz.into());
    m.insert("n_peaks".into(), n_peaks.into());
    m.insert(
        "reactions".into(),
        Value::Array(reactions_of(&meta, params.as_ref())),
    );
    if let Some(p) = params.as_ref() {
        let sps: Vec<f64> = (0..64)
            .map_while(|c| p.sps_mass(c).map(f64::from).filter(|&x| x > 0.0))
            .collect();
        if !sps.is_empty() {
            m.insert("sps_precursors".into(), json!(sps));
        }
        m.insert("trailer".into(), Value::Object(values_of(p.record())));
    }
    put(&mut m, "faims_cv", meta.faims_cv);
    Some((Value::Object(m), meta))
}

/// Trait glue: `scan_params` hands out an owned record; `scan_metadata_with` wants a borrowed one.
trait Borrow {
    fn clone_params(&self) -> Option<ScanParams<'_>>;
}
impl Borrow for Option<ScanParams<'static>> {
    fn clone_params(&self) -> Option<ScanParams<'_>> {
        self.as_ref().map(ScanParams::borrowed)
    }
}

fn fileset(name: &str, bytes: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: name.to_string(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }],
    }
}

struct Sink(GzEncoder<std::fs::File>);
impl Sink {
    fn row(&mut self, v: &Value) {
        let mut s = serde_json::to_string(v).expect("json");
        s.push('\n');
        self.0.write_all(s.as_bytes()).expect("write");
    }
}

pub(crate) type Scans = Vec<(Value, ScanMetadata)>;

/// What the options of the command line ask for.
struct Opts {
    full: bool,
    text: bool,
    down: bool,
}

/// Scans whose peaks are written: first, last, median, largest TIC and one per level (F.1), or all with `--full`.
fn peak_scans(scans: &Scans, full: bool) -> Vec<usize> {
    let n = scans.len();
    if full || n == 0 {
        return (0..n).collect();
    }
    let top = (0..n).max_by(|&i, &j| {
        scans[i]
            .1
            .total_ion_current
            .total_cmp(&scans[j].1.total_ion_current)
    });
    let mut pick = vec![0, n - 1, n / 2];
    pick.extend(top);
    for lv in 1..=8 {
        pick.extend(scans.iter().position(|s| s.1.ms_level == lv));
    }
    pick.sort_unstable();
    pick.dedup();
    pick
}

/// The optional rows after the scans: chromatograms, status, errors, method.
fn write_extras(
    out: &mut Sink,
    l: &mut dyn Lettore,
    raw: &RawFileReader,
    src: &mut Cursor<&[u8]>,
    text: bool,
) {
    for v in l.cromatogrammi(0).unwrap_or_default() {
        if let Ok(c) = l.cromatogramma(0, &v.id) {
            out.row(&json!({"type": "chromatogram", "id": c.id, "kind": format!("{:?}", c.kind).to_lowercase(),
                "unit": c.unit, "time": c.time, "values": c.intensity}));
        }
    }
    if let Some(sl) = raw.status_log.as_ref() {
        let n = sl.records.len();
        for i in (0..n).step_by(n.div_ceil(16).max(1)) {
            let mut o = Map::new();
            o.insert("type".into(), "status".into());
            o.insert("scan".into(), i.into());
            put(&mut o, "rt", sl.times_min.get(i).map(|t| f64::from(*t)));
            o.insert("values".into(), Value::Object(values_of(&sl.records[i])));
            out.row(&Value::Object(o));
        }
    }
    for e in &raw.error_log {
        out.row(&json!({"type": "error", "rt": f64::from(e.time), "message": e.message.trim()}));
    }
    for (b, t) in raw.instrument_method_blocks(src).iter().enumerate() {
        let mut o = json!({"type": "method", "block": b, "length": t.chars().count(), "sha256": hex(&Sha256::digest(t.as_bytes()))});
        if text {
            o["text"] = t.clone().into();
        }
        out.row(&o);
    }
}

fn file_row(
    name: &str,
    bytes: &[u8],
    raw: &RawFileReader,
    src: &mut Cursor<&[u8]>,
    n_scans: usize,
    t: [f64; 3],
    down: bool,
) -> Value {
    let controller = raw
        .controllers(src)
        .ok()
        .and_then(|c| c.iter().position(|x| x.is_ms_controller));
    let mut head = Map::new();
    head.insert("type".into(), "file".into());
    head.insert("schema".into(), "scandump/1".into());
    head.insert("name".into(), name.into());
    head.insert("sha256".into(), hex(&Sha256::digest(bytes)).into());
    head.insert("format_version".into(), raw.version.into());
    put(&mut head, "instrument", raw.instrument_model);
    put(&mut head, "controller", controller);
    head.insert("n_scans".into(), n_scans.into());
    head.insert("producer".into(), "mzlab".into());
    head.insert("producer_version".into(), env!("CARGO_PKG_VERSION").into());
    for (k, v) in ["open_ms", "read_all_ms", "read_one_ms"].iter().zip(t) {
        head.insert((*k).into(), v.into());
    }
    if down {
        head.insert("downstream".into(), true.into());
    }
    Value::Object(head)
}

fn main() {
    // a file that does not open is a result of the bench, not a crash: one line and exit code 2
    std::panic::set_hook(Box::new(|i| {
        eprintln!("thermo_scandump: {i}");
        std::process::exit(2);
    }));
    let a: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| a.iter().any(|x| x == f);
    let pos: Vec<&String> = a.iter().filter(|x| !x.starts_with("--")).collect();
    let (src_path, out_path) = (
        pos.first().expect("FILE.raw"),
        pos.get(1).expect("OUT.scandump.jsonl.gz"),
    );
    let o = Opts {
        full: flag("--full"),
        text: flag("--text"),
        down: flag("--downstream"),
    };
    let bytes = std::fs::read(src_path).expect("read");
    let name = std::path::Path::new(src_path.as_str())
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());

    let t0 = Instant::now();
    let mut src = Cursor::new(bytes.as_slice());
    let raw = RawFileReader::open(&mut src).expect("open");
    let mut l = (rawlettore::VOCE.apri)(fileset(&name, bytes.clone()), &Limiti::desktop())
        .expect("lettore");
    let open_ms = t0.elapsed().as_secs_f64() * 1e3;
    let n = l.indice(0).expect("index").len();

    // every scan once: the row of the dump and the peaks that the lettore reads
    let t1 = Instant::now();
    let mut scans: Scans = Vec::new();
    let mut spectra = Vec::new();
    for k in 0..n {
        let s = l.scansione(0, k).expect("scan");
        if let Some(r) = scan_row(&raw, &mut src, k as u32, s.mz.len()) {
            scans.push(r);
        }
        spectra.push(s);
    }
    let read_all_ms = t1.elapsed().as_secs_f64() * 1e3;
    let t2 = Instant::now();
    let _ = l.scansione(0, n / 2);
    let read_one_ms = t2.elapsed().as_secs_f64() * 1e3;

    if o.down {
        downstream::add_downstream(&mut *l, &raw, &mut src, &name, &mut scans);
    }
    let mut out = Sink(GzEncoder::new(
        std::fs::File::create(out_path).expect("create"),
        Compression::default(),
    ));
    let t = [open_ms, read_all_ms, read_one_ms];
    out.row(&file_row(
        &name,
        &bytes,
        &raw,
        &mut src,
        scans.len(),
        t,
        o.down,
    ));
    for (row, _) in &scans {
        out.row(row);
    }
    for i in peak_scans(&scans, o.full) {
        let s = &spectra[i];
        let ints: Vec<f64> = (0..s.mz.len()).map(|j| s.intensity_f64(j)).collect();
        out.row(&json!({"type": "peaks", "n": scans[i].1.scan_number, "mz": s.mz, "int": ints}));
    }
    write_extras(&mut out, &mut *l, &raw, &mut src, o.text);
    out.0.finish().expect("finish");
}
