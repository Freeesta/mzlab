//! Engine parity on the real HR series (`dati/HRMS/Timolol/` of the private data repository): the indexes, the "Feature table" plan (index, detect, align,
//! gapfill) run twice natively, and the table of the page chain (`run::recipe_of_layers`, the one the browser runner builds) must be byte for byte the same.
//! Skipped without the data repository (`MZLAB_DATI`). Nothing measured on the data is written here: only the time and the peak RSS go to stderr.

use std::path::{Path, PathBuf};
use std::time::Instant;

use mzlab_core::budget::sha256_hex;
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat_wasm::run;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn series() -> Option<(PathBuf, Vec<PathBuf>)> {
    let dir = dati_percorsi::percorso("dati/HRMS/Timolol")?;
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
        .collect();
    v.sort();
    (!v.is_empty()).then_some((dir, v))
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

fn index_of(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut src = bytes.to_vec();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    build_mzml_to_vec(&p.table, bytes, 256 << 20).unwrap().0
}

/// Panics at the first byte where `a` and `b` differ, naming the file and the offset with the bytes around it.
fn same(file: &str, a: &[u8], b: &[u8], what: &str) {
    let n = a.iter().zip(b).position(|(x, y)| x != y);
    let at = n.or((a.len() != b.len()).then(|| a.len().min(b.len())));
    if let Some(o) = at {
        let w = |s: &[u8]| s[o.saturating_sub(8)..(o + 8).min(s.len())].to_vec();
        panic!(
            "{what}: {file} differs at offset {o} (lengths {} and {}): {:02x?} vs {:02x?}",
            a.len(),
            b.len(),
            w(a),
            w(b)
        );
    }
}

/// The files of a run folder without the clocks of `prov.json` and `corsa.json`.
fn folder(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|f| f.is_file())
        .map(|f| (name(&f), std::fs::read(&f).unwrap()))
        .filter(|(n, _)| n != "prov.json" && n != "corsa.json")
        .collect();
    v.sort();
    v
}

fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .map_or(0, |kb| kb / 1024)
}

/// The "Feature table" plan run natively with 1 and 2 workers: the files of the first run (the two must agree byte for byte).
fn native_runs(dir: &Path, names: &[String]) -> Vec<(String, Vec<u8>)> {
    let inputs: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let refs: Vec<String> = names.iter().map(|n| format!(r#""file:{n}""#)).collect();
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{}],"steps":[
{{"id":"idx","step":"index.mzidx@1","in":{{"source":[{r}]}}}},
{{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
{{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
{{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}}]}}"#,
        inputs.join(","),
        r = refs.join(",")
    );
    let work = std::env::temp_dir().join(format!("mzlab_parita_dati_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let mut runs = Vec::new();
    for k in 0..2 {
        let out = work.join(format!("run{k}"));
        std::fs::create_dir_all(&out).unwrap();
        let (_, prov) = crate::flow::run_plan(plan.as_bytes(), dir, &out, 1 + k, None).unwrap();
        assert!(!prov.contains("\"error\""), "{prov}");
        runs.push(folder(&out));
    }
    let (a, b) = (&runs[0], &runs[1]);
    assert_eq!(
        a.iter().map(|f| &f.0).collect::<Vec<_>>(),
        b.iter().map(|f| &f.0).collect::<Vec<_>>(),
        "the two runs make different files"
    );
    for (x, y) in a.iter().zip(b) {
        same(&x.0, &x.1, &y.1, "native run (1 worker vs 2)");
        assert_eq!(sha256_hex(&x.1), sha256_hex(&y.1));
    }
    let _ = std::fs::remove_dir_all(&work);
    runs.swap_remove(0)
}

/// The bytes of the table of the browser-runner chain over the indexes in memory.
fn page_chain(names: &[String], idxs: &[Vec<u8>]) -> Vec<u8> {
    // the chain of the browser runner (`run::recipe_of_layers`, the floor estimated on each file for detection) over the indexes in memory
    let open = |k: usize| Index::open(idxs[k].clone(), idxs[k].len() as u64);
    let layers = run::layers_from_json(
        r#"[{"step":"features.detect","params":{}},{"step":"features.align","params":{}},{"step":"features.gapfill","params":{}}]"#,
    )
    .unwrap();
    let det: Vec<Vec<u8>> = names
        .iter()
        .enumerate()
        .map(|(k, n)| {
            let r = run::recipe_of_layers(&mut open(k).unwrap(), &layers).unwrap();
            run::detect(&mut open(k).unwrap(), n, &r).unwrap()
        })
        .collect();
    let r = run::recipe_of_layers(&mut open(0).unwrap(), &layers).unwrap();
    let (tables, origin) = run::read_files(&det).unwrap();
    run::align(&tables, &origin, &r, open).unwrap()
}

#[test]
fn the_real_series_gives_the_same_bytes_in_every_engine() {
    let Some((dir, files)) = series() else {
        eprintln!("SKIP flow_parita_dati: no data repository (MZLAB_DATI) or no mzML in dati/HRMS/Timolol");
        return;
    };
    let t0 = Instant::now();
    let names: Vec<String> = files.iter().map(|p| name(p)).collect();
    let bytes: Vec<Vec<u8>> = files.iter().map(|p| std::fs::read(p).unwrap()).collect();

    // 1. the index of every file, built twice
    let idxs: Vec<Vec<u8>> = names
        .iter()
        .zip(&bytes)
        .map(|(n, b)| index_of(b, n))
        .collect();
    for ((n, b), first) in names.iter().zip(&bytes).zip(&idxs) {
        same(
            &format!("{n}.mzidx"),
            first,
            &index_of(b, n),
            "index built twice",
        );
    }

    // 2. the plan natively, twice
    let a = native_runs(&dir, &names);
    // 3. the chain of the browser runner
    let page = page_chain(&names, &idxs);
    let flow = &a
        .iter()
        .find(|f| f.0 == "gap.mzfeat")
        .expect("gap.mzfeat")
        .1;
    same("gap.mzfeat", flow, &page, "native flow run vs page chain");

    eprintln!(
        "flow_parita_dati: {} files, {:.1} s, peak RSS {} MB",
        files.len(),
        t0.elapsed().as_secs_f64(),
        peak_rss_mb()
    );
}
