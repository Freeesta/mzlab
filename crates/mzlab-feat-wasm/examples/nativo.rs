//! The pipeline of the bridge on files, natively: the reference of the byte-for-byte test against the `.wasm` (`tests/test_feat_wasm.py`).
//!
//!     cargo run --release -p mzlab-feat-wasm --example nativo -- OUT_DIR file.mzidx... [--recipe JSON]
//!
//! Writes `<name>.mzfeatf` for every file and `series.mzfeat`, and prints the SHA-256 of each; `--parent FORMULA --rules regole_tp.json` adds the
//! hypotheses and the graph of the proofs (`series_evid.mzfeat`; times from `tNNNmin` in the name, `--parent-mz M --parent-rt R` for the parent row).

use std::fs::File;
use std::path::Path;
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat_wasm::run;
use sha2::{Digest, Sha256};

fn open(path: &str) -> Index<File> {
    let f = File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn base(path: &str) -> String {
    Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// Minutes of `..._t030min...` (NaN for a control).
fn minutes(name: &str) -> f64 {
    name.rfind("_t")
        .and_then(|k| name[k + 2..].split("min").next()?.parse().ok())
        .unwrap_or(f64::NAN)
}

fn main() {
    let (mut files, mut recipe, mut out) = (Vec::<String>::new(), String::new(), String::new());
    let (mut parent, mut rules, mut pmz, mut prt) =
        (String::new(), String::new(), f64::NAN, f64::NAN);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut next = || args.next().expect("value");
        match a.as_str() {
            "--recipe" => recipe = next(),
            "--parent" => parent = next(),
            "--rules" => rules = next(),
            "--parent-mz" => pmz = next().parse().expect("number"),
            "--parent-rt" => prt = next().parse().expect("number"),
            _ if out.is_empty() => out = a,
            _ => files.push(a),
        }
    }
    let r = run::recipe_from_json(&recipe).expect("recipe");
    let t0 = Instant::now();
    let mut stored = Vec::new();
    for p in &files {
        let b = run::detect(&mut open(p), &base(p), &r).expect("detect");
        println!("{}.mzfeatf {}", base(p), sha(&b));
        std::fs::write(Path::new(&out).join(format!("{}.mzfeatf", base(p))), &b).expect("write");
        stored.push(b);
    }
    eprintln!("detect: {:.2} s", t0.elapsed().as_secs_f64());
    let refs: Vec<&[u8]> = stored.iter().map(Vec::as_slice).collect();
    let (feats, origin) = run::read_files(&refs).expect("read");
    let table = run::align(&feats, &origin, &r, |k| Ok(open(&files[k]))).expect("align");
    println!("series.mzfeat {}", sha(&table));
    std::fs::write(Path::new(&out).join("series.mzfeat"), &table).expect("write");
    if parent.is_empty() {
        return;
    }
    let mut ix: Vec<_> = files.iter().map(|p| open(p)).collect();
    let fr: Vec<_> = feats.iter().collect();
    let rt = prt.is_finite().then_some(prt);
    let rules = std::fs::read(rules).expect("rules");
    let h = run::hypotheses(&mut ix, &fr, &rules, &parent, rt, &r).expect("hypotheses");
    println!(
        "hypotheses {} tried, {} found",
        h.result.tried, h.result.found
    );
    let mins: Vec<f64> = files.iter().map(|p| minutes(&base(p))).collect();
    let par = (pmz.is_finite() && prt.is_finite()).then_some((pmz, prt));
    let ev = run::evidence(&table, &h, &mins, par, &r).expect("evidence");
    println!("series_evid.mzfeat {}", sha(&ev));
    std::fs::write(Path::new(&out).join("series_evid.mzfeat"), &ev).expect("write");
}
