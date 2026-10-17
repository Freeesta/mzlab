//! The Dig of mzFinder on high-resolution files, natively: the pipeline (as `nativo`) and the adapter `mzlab_feat::legacy` that turns the series into the
//! JSON of `api.run` and `api.detail`. Writes `run.json`, `detail.json` (the first candidates) and `inclusion.csv` in OUT_DIR.
//!
//!     cargo run --release -p mzlab-feat-wasm --example finder -- OUT_DIR file.mzidx... --parent FORMULA --rules regole_tp.json [--smiles S] [--parent-rt R]

use std::fs::File;
use std::path::Path;

use mzlab_core::idxread::Index;
use mzlab_core::json::Out;
use mzlab_feat::legacy;
use mzlab_feat_wasm::run;

fn open(path: &str) -> Result<Index<File>, String> {
    let f = File::open(path).map_err(|e| e.to_string())?;
    let size = f.metadata().map_err(|e| e.to_string())?.len();
    Index::open(f, size).map_err(|e| e.to_string())
}

fn base(path: &str) -> String {
    Path::new(path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .replace(".mzidx", ".mzML")
}

/// The files as the page passes them to `run`: name, type and time guessed from the name; the LC files keep this order.
fn files_json(files: &[String]) -> Out {
    let mut fj = Out::default();
    fj.raw("[");
    for (i, p) in files.iter().enumerate() {
        let g = legacy::sample::guess(&base(p));
        fj.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&base(p))
            .raw(",\"type\":")
            .str(g.kind);
        fj.raw(",\"time\":").opt(g.time).raw(",\"kind\":\"full\"}");
    }
    fj.raw("]");
    fj
}

/// Writes `run.json`, `detail.json` (first five candidates) and `inclusion.csv`.
fn write(out: &str, d: &legacy::dig::Dig) {
    let o = Path::new(out);
    std::fs::write(o.join("run.json"), legacy::out::summary(d, "{}", "{}")).expect("write");
    let det: Vec<String> = d
        .cands
        .iter()
        .take(5)
        .map(|c| legacy::out::detail(d, c.id()).expect("detail"))
        .collect();
    std::fs::write(o.join("detail.json"), format!("[{}]", det.join(","))).expect("write");
    std::fs::write(o.join("inclusion.csv"), legacy::out::inclusion_csv(d, 0)).expect("write");
}

fn main() {
    let (mut files, mut out, mut parent, mut rules, mut smiles, mut prt) = (
        Vec::<String>::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        f64::NAN,
    );
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut next = || args.next().expect("value");
        match a.as_str() {
            "--parent" => parent = next(),
            "--rules" => rules = next(),
            "--smiles" => smiles = next(),
            "--parent-rt" => prt = next().parse().expect("number"),
            _ if out.is_empty() => out = a,
            _ => files.push(a),
        }
    }
    let fj = files_json(&files);
    let lc = legacy::series::lc_files(&fj.0).expect("files");
    let paths: Vec<&String> = lc
        .iter()
        .map(|f| files.iter().find(|p| base(p) == f.name).expect("path"))
        .collect();
    let r = run::recipe_for("", &mut open(paths[0]).expect("open")).expect("recipe");
    let stored: Vec<Vec<u8>> = paths
        .iter()
        .map(|p| run::detect(&mut open(p).expect("open"), &base(p), &r).expect("detect"))
        .collect();
    let refs: Vec<&[u8]> = stored.iter().map(Vec::as_slice).collect();
    let (feats, origin) = run::read_files(&refs).expect("read");
    let table =
        run::align(&feats, &origin, &r, |k| Ok(open(paths[k]).expect("open"))).expect("align");
    let structure = if smiles.is_empty() {
        String::new()
    } else {
        mzlab_feat::chem::grafo::analyse_json(&smiles).expect("smiles")
    };
    let mut ix: Vec<_> = paths.iter().map(|p| open(p).expect("open")).collect();
    let fr: Vec<_> = feats.iter().collect();
    let rules = std::fs::read(rules).expect("rules");
    let rt = prt.is_finite().then_some(prt);
    let h = run::hypotheses(&mut ix, &fr, &rules, &structure, &parent, rt, &r).expect("hypotheses");
    let mins: Vec<f64> = lc.iter().map(|f| f.time.unwrap_or(f64::NAN)).collect();
    let mut pj = Out::default();
    pj.raw("{\"name\":\"parent\",\"neutral\":")
        .str(&parent)
        .raw(",\"adduct\":\"[M+H]+\"");
    if !smiles.is_empty() {
        pj.raw(",\"smiles\":").str(&smiles);
    }
    pj.raw("}");
    let p = legacy::series::parent(&pj.0).expect("parent");
    // as the page does (`finder_parent`): the proofs read the kinetics, the in-source fragments and the MS2 notes against the parent peak found in the table
    let par = legacy::parent_peak(&fj.0, &pj.0, &table)
        .expect("parent peak")
        .map(|x| (p.mz, x.1));
    let ev = run::evidence(&table, &h, &mins, par, &r).expect("evidence");
    let d = legacy::dig(&fj.0, &pj.0, &ev, &h.registry.to_json(), |k| open(paths[k])).expect("dig");
    write(&out, &d);
    let with_ms2 = d.cands.iter().filter(|c| c.ms2.is_some()).count();
    println!(
        "rows {} candidates {} with MS2 {} parent row {:?} parent MS2 {}",
        table.len(),
        d.cands.len(),
        with_ms2,
        d.parent_row,
        d.parent_ms2.is_some()
    );
}
