//! The pipeline after mzmine 3.9.0 on files, natively: the reference of the byte-for-byte test against the `.wasm` `untargeted`, the engine of the gate
//! (`tools/bench/corri_mzlab.py --engine untargeted`) and the run on real data.
//!
//!     cargo run --release -p mzlab-feat-wasm --features untargeted --example untargeted -- OUT_DIR INPUT... [--params JSON | --params-file FILE] [--csv FILE] [--budget-mb N]
//!
//! `INPUT` = a `.mzidx`, an `.mzML` (indexed in memory) or a folder of them (`.mzidx` if it has any, else `.mzML`, sorted by name). Writes `<name>.mzfeatf` for every
//! file and `series.mzfeat` in `OUT_DIR` (the formats of the site), and prints per file the number of features, the seconds and the peak memory, then the SHA-256
//! of every output. `--csv` also writes the table as the export of mzmine (`row ID`, `row m/z`, `row retention time`, `datafile:<file>:area|height`; areas in
//! intensity x minutes, an empty cell left blank).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;

use mzlab_core::budget::{Hashing, MemBudget, WriteAt};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::read_table_from;
use mzlab_feat::table::CELL_ABSENT;
use mzlab_feat::untargeted::pipeline::UntParams;
use mzlab_feat_wasm::untargeted::{align_to, detect_to};

/// A file written at offsets.
struct FileSink(File);

impl WriteAt for FileSink {
    fn write_at(&mut self, off: u64, data: &[u8]) -> Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        self.0
            .seek(SeekFrom::Start(off))
            .and_then(|_| self.0.write_all(data))
            .map_err(|_| Error::new("error.io"))
    }
}

/// The `.mzfeatf` of a series held in memory.
struct Held(Vec<Vec<u8>>);

impl Stored for Held {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
}

/// Peak resident memory of the process in MB (Linux), 0 elsewhere.
fn peak_mb() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        .unwrap_or(0.0)
        / 1024.0
}

fn expand(inputs: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for i in inputs {
        let p = PathBuf::from(i);
        if !p.is_dir() {
            out.push(p);
            continue;
        }
        let list = |ext: &str| {
            let mut v: Vec<PathBuf> = std::fs::read_dir(&p)
                .expect("read folder")
                .flatten()
                .map(|e| e.path())
                .filter(|f| f.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext)))
                .collect();
            v.sort();
            v
        };
        let idx = list("mzidx");
        out.extend(if idx.is_empty() { list("mzml") } else { idx });
    }
    out
}

/// The index of a file: a `.mzidx` is opened, an `.mzML` is indexed in memory.
fn open(path: &Path) -> Index<Box<dyn ReadAt>> {
    let ext = path.extension().map(|x| x.to_ascii_lowercase());
    if ext.is_some_and(|x| x == "mzidx") {
        let f = File::open(path).expect("open");
        let size = f.metadata().expect("size").len();
        return Index::open(Box::new(f) as Box<dyn ReadAt>, size)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
    let mzml = std::fs::read(path).expect("read mzML");
    let mut src = mzml.clone();
    let name = path.file_name().unwrap().to_string_lossy();
    let p = prepare(&mut src, mzml.len() as u64, &name).expect("prepare");
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20)
        .expect("index")
        .0;
    let n = bytes.len() as u64;
    Index::open(Box::new(bytes) as Box<dyn ReadAt>, n).expect("index")
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

/// Writes the table as the export of mzmine.
fn write_csv(path: &str, series: &Path, names: &[String]) {
    let f = File::open(series).expect("series");
    let size = f.metadata().expect("size").len();
    let mut f = f;
    let (t, _) = read_table_from(&mut f, size).expect("table");
    let mut s = String::from("row ID,row m/z,row retention time");
    for n in names {
        s += &format!(",datafile:{n}.mzML:area,datafile:{n}.mzML:height");
    }
    s.push('\n');
    for r in 0..t.len() {
        s += &format!("{},{},{}", r + 1, t.mz[r], t.rt[r]);
        for k in 0..t.n_files {
            let c = r * t.n_files + k;
            if t.state[c] == CELL_ABSENT {
                s += ",,";
            } else {
                s += &format!(",{},{}", t.area[c] / 60.0, t.height[c]);
            }
        }
        s.push('\n');
    }
    std::fs::write(path, s).expect("csv");
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (mut inputs, mut params, mut csv, mut mb) = (Vec::new(), String::new(), None, f64::NAN);
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--params" => params = a[i + 1].clone(),
            "--params-file" => params = std::fs::read_to_string(&a[i + 1]).expect("params file"),
            "--csv" => csv = Some(a[i + 1].clone()),
            "--budget-mb" => mb = a[i + 1].parse().expect("--budget-mb"),
            _ => {
                inputs.push(a[i].clone());
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    let out = inputs.remove(0);
    let files = expand(&inputs);
    let p = UntParams::from_json(&params).unwrap_or_else(|e| panic!("params: {e}"));
    std::fs::create_dir_all(&out).expect("out dir");
    let names: Vec<String> = files.iter().map(|f| stem(f)).collect();
    let mut stored = Vec::new();
    for (path, name) in files.iter().zip(&names) {
        let t = Instant::now();
        let mut ix = open(path);
        let dest = Path::new(&out).join(format!("{name}.mzfeatf"));
        let mut sink = Hashing::new(FileSink(File::create(&dest).expect("create")));
        detect_to(&mut ix, name, &p, &mut sink).unwrap_or_else(|e| panic!("{name}: {e}"));
        let (_, bytes, sha) = sink.finish();
        stored.push(std::fs::read(&dest).expect("read back"));
        println!(
            "{name}.mzfeatf {bytes} bytes sha256 {sha} ({:.1} s, peak memory {:.0} MB)",
            t.elapsed().as_secs_f64(),
            peak_mb()
        );
    }
    let t = Instant::now();
    let series = Path::new(&out).join("series.mzfeat");
    let mut sink = Hashing::new(FileSink(File::create(&series).expect("create")));
    let mut ixs: Vec<Option<Index<Box<dyn ReadAt>>>> =
        files.iter().map(|f| Some(open(f))).collect();
    let open_k = |k: usize| Ok(ixs[k].take().expect("index opened once"));
    align_to(
        &mut Held(stored),
        &MemBudget::from_request(mb),
        &p,
        open_k,
        &mut sink,
    )
    .unwrap_or_else(|e| panic!("series: {e}"));
    let (_, bytes, sha) = sink.finish();
    println!(
        "series.mzfeat {bytes} bytes sha256 {sha} ({:.1} s, peak memory {:.0} MB)",
        t.elapsed().as_secs_f64(),
        peak_mb()
    );
    if let Some(c) = csv {
        write_csv(&c, &series, &names);
    }
}
