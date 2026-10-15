//! The low-resolution Dig run natively as the browser runs it (the pool of `feat-worker.js`, then `finder_lr_dig`), for the e2e test that
//! compares the two: `cargo run --release -p mzlab-finder-lr-wasm --example finder_lr -- OUT FILES.json PARENT.json RULES.json [--ms2 N] MZML...`.
//! FILES.json = the files of `api.run` (same order as the mzML). `--ms2 N`: the last N mzML are MS2 files (kind `ms2` in FILES.json): they
//! are not detected nor aligned, as in the browser, and the matches go to `ms2_links.json`. Writes in OUT the `.mzidx` of every mzML (`<name>.mzidx`), `run.json`
//! (`timing` = {}) and `detail.json` (the parent and every row that passes the criteria).

use std::path::Path;

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::legacy::lr::{self, detail::detail, summary::summary};
use mzlab_feat_wasm::run;

fn index(path: &Path) -> Vec<u8> {
    let mzml = std::fs::read(path).expect("mzML");
    let mut src = mzml.clone();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let p = prepare(&mut src, mzml.len() as u64, &name).expect("prepare");
    build_mzml_to_vec(&p.table, &mzml, 64 << 20)
        .expect("index")
        .0
}

fn open(idx: &[u8]) -> Index<Vec<u8>> {
    Index::open(idx.to_vec(), idx.len() as u64).unwrap()
}

fn main() {
    let mut a: Vec<String> = std::env::args().skip(1).collect();
    let ms2 = match a.iter().position(|x| x == "--ms2") {
        Some(i) => {
            let n = a[i + 1].parse::<usize>().expect("--ms2 N");
            a.drain(i..i + 2);
            n
        }
        None => 0,
    };
    let (out, files, parent, rules) = (
        Path::new(&a[0]),
        std::fs::read_to_string(&a[1]).unwrap(),
        std::fs::read_to_string(&a[2]).unwrap(),
        std::fs::read(&a[3]).unwrap(),
    );
    let idxs: Vec<Vec<u8>> = a[4..].iter().map(|p| index(Path::new(p))).collect();
    for (p, b) in a[4..].iter().zip(&idxs) {
        std::fs::write(
            out.join(format!(
                "{}.mzidx",
                Path::new(p).file_name().unwrap().to_string_lossy()
            )),
            b,
        )
        .unwrap();
    }
    let cols = idxs.len() - ms2;
    let plan = lr::plan::plan_json(&files, &parent).unwrap();
    let formula = plan
        .split("\"formula\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("");
    let r = run::recipe_for("", &mut open(&idxs[0])).unwrap();
    // as detect_file_to of the pool: the recipe of every file from its own index, the file name as the name
    let stored: Vec<Vec<u8>> = idxs[..cols]
        .iter()
        .zip(&a[4..])
        .map(|(b, p)| {
            let name = Path::new(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let rk = run::recipe_for("", &mut open(b)).unwrap();
            run::detect(&mut open(b), &name, &rk).unwrap()
        })
        .collect();
    let (feats, origin) = run::read_files(&stored).unwrap();
    let mut ix: Vec<Index<Vec<u8>>> = idxs[..cols].iter().map(|b| open(b)).collect();
    let refs: Vec<_> = feats.iter().collect();
    let h = run::hypotheses(&mut ix, &refs, &rules, "", formula, None, &r).unwrap();
    let table = run::align(&feats, &origin, &r, |k| Ok(open(&idxs[k]))).unwrap();
    let mut d = lr::dig(&files, &parent, "{}", &table, &h.registry.to_json()).unwrap();
    lr::ms2link::ms2_link(&mut d, &[], &mut |k| Ok(open(&idxs[k]))).unwrap();
    let mut o = mzlab_core::json::Out::default();
    lr::ms2link::json(&mut o, d.links.as_ref().unwrap());
    let links = o.0;
    std::fs::write(out.join("ms2_links.json"), links).unwrap();
    std::fs::write(out.join("run.json"), summary(&d, "{}")).unwrap();
    let shown = d
        .entries
        .iter()
        .filter(|e| matches!(e.label, "progenitore" | "forte" | "possibile"));
    let det: Vec<String> = shown
        .map(|e| detail(&d, e.id, |k| Ok(open(&idxs[k]))).unwrap())
        .collect();
    std::fs::write(out.join("detail.json"), format!("[{}]", det.join(","))).unwrap();
}
