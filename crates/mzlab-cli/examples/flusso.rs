//! Prototype of a workflow file (`docs/studi/WORKFLOW_MODULARI.md` §6): runs a linear chain `convert.mzml` → `features.table` read from a
//! `mzflow/2-draft` JSON file and writes, next to the outputs, `prov.json` with the SHA-256 of every file, the time and the heap peak of every step.
//! Native: `cargo run --release -p mzlab-cli --example flusso -- FLOW.json WORKDIR`; the same source builds for `wasm32-wasip1`.
use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};
use mzlab_core::lettori::{FileSet, Limiti, Membro};

struct Conta;
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PICCO: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forwards every call to the system allocator and only counts the bytes.
unsafe impl GlobalAlloc for Conta {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            let v = VIVI.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PICCO.fetch_max(v, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        VIVI.fetch_sub(l.size(), Ordering::Relaxed);
    }
}
#[global_allocator]
static A: Conta = Conta;

fn io(e: std::io::Error) -> Error {
    Error::new("err.flow.io").with("detail", e)
}

/// One step of the chain as the file declares it.
struct Step {
    id: String,
    kind: String,
    input: String,
    params: String,
}

/// The JSON text of a flat object of numbers, strings and booleans (the parameters of a step).
fn flat_json(v: Option<&Value>) -> String {
    let mut o = Out::default();
    o.raw("{");
    if let Some(Value::Obj(kv)) = v {
        for (k, (key, val)) in kv.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(key).raw(":");
            match val {
                Value::Num(x) => o.num(*x),
                Value::Bool(b) => o.raw(if *b { "true" } else { "false" }),
                Value::Str(s) => o.str(s),
                _ => o.raw("null"),
            };
        }
    }
    o.raw("}");
    o.0
}

fn steps(flow: &Value) -> Result<Vec<Step>> {
    if flow.get("schema").and_then(Value::str) != Some("mzflow/2-draft") {
        return Err(Error::new("err.flow.schema"));
    }
    let text = |s: &Value, k: &str| {
        s.get(k)
            .and_then(Value::str)
            .unwrap_or_default()
            .to_string()
    };
    Ok(flow
        .get("steps")
        .map(Value::arr)
        .unwrap_or_default()
        .iter()
        .map(|s| Step {
            id: text(s, "id"),
            kind: text(s, "step"),
            input: s
                .get("in")
                .and_then(|i| match i {
                    Value::Obj(kv) => kv.first().and_then(|(_, v)| v.str()),
                    _ => None,
                })
                .unwrap_or_default()
                .to_string(),
            params: flat_json(s.get("params")),
        })
        .collect())
}

/// `convert.mzml`: sample 0 of any single file of the registry, as mzML.
fn convert(raw: &Path, out: &Path) -> Result<()> {
    let name = raw
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let dim = std::fs::metadata(raw).map_err(io)?.len();
    let fs = FileSet {
        membri: vec![Membro {
            nome: name,
            dim,
            dati: Box::new(File::open(raw).map_err(io)?),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(fs, &Limiti::desktop())?;
    let w = BufWriter::new(File::create(out).map_err(io)?);
    mzlab_core::lettori_mzml::scrivi(&mut *l, 0, w, &mut |_, _| {})?;
    Ok(())
}

fn run(s: &Step, input: &Path, work: &Path) -> Result<PathBuf> {
    match s.kind.as_str() {
        "convert.mzml" => {
            let out = work.join(format!("{}.mzML", s.id));
            convert(input, &out)?;
            Ok(out)
        }
        "features.table" => {
            let tmp = work.join(format!("{}.tmp", s.id));
            std::fs::create_dir_all(&tmp).map_err(io)?;
            let csv = mzlab_cli::features(&[input.to_path_buf()], &s.params, &tmp);
            let _ = std::fs::remove_dir_all(&tmp);
            let out = work.join(format!("{}.csv", s.id));
            std::fs::write(&out, csv?).map_err(io)?;
            Ok(out)
        }
        k => Err(Error::new("err.flow.unknownStep").with("step", k)),
    }
}

/// The entity record of a file: name, bytes, SHA-256.
fn entity(o: &mut Out, p: &Path) -> Result<()> {
    let b = std::fs::read(p).map_err(io)?;
    let name = p
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    o.raw("{\"name\":")
        .str(&name)
        .raw(",\"bytes\":")
        .int(b.len() as i64);
    o.raw(",\"sha256\":").str(&sha256_hex(&b)).raw("}");
    Ok(())
}

fn main_r(flow_path: &Path, work: &Path) -> Result<String> {
    let text = std::fs::read(flow_path).map_err(io)?;
    let flow = parse(&text)?;
    let dir = flow_path.parent().unwrap_or(Path::new("."));
    let mut o = Out::default();
    o.raw("{\"schema\":\"prov/1-draft\",\"software\":{\"name\":\"mzLab\",\"version\":");
    o.str(env!("CARGO_PKG_VERSION"))
        .raw("},\"flow_sha256\":")
        .str(&sha256_hex(&text));
    o.raw(",\"activities\":[");
    let mut last: Option<PathBuf> = None;
    for (k, s) in steps(&flow)?.iter().enumerate() {
        let input = match s.input.strip_prefix("file:") {
            Some(f) => dir.join(f),
            None => last
                .clone()
                .ok_or_else(|| Error::new("err.flow.noInput").with("step", &s.id))?,
        };
        PICCO.store(VIVI.load(Ordering::Relaxed), Ordering::Relaxed);
        let t = Instant::now();
        let out = run(s, &input, work)?;
        let ms = t.elapsed().as_secs_f64() * 1e3;
        o.raw(if k > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(&s.id)
            .raw(",\"step\":")
            .str(&s.kind);
        o.raw(",\"params\":").raw(&s.params).raw(",\"used\":");
        entity(&mut o, &input)?;
        o.raw(",\"generated\":");
        entity(&mut o, &out)?;
        o.raw(",\"ms\":").num((ms * 10.0).round() / 10.0);
        o.raw(",\"heap_peak_bytes\":")
            .int(PICCO.load(Ordering::Relaxed) as i64)
            .raw("}");
        last = Some(out);
    }
    o.raw("]}\n");
    Ok(o.0)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 3 {
        eprintln!("usage: flusso FLOW.json WORKDIR");
        std::process::exit(2);
    }
    let work = Path::new(&a[2]);
    match main_r(Path::new(&a[1]), work) {
        Ok(prov) => {
            let _ = std::fs::write(work.join("prov.json"), &prov);
            print!("{prov}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}
