//! Native reference of the bridge: writes the views, the report and the files of a table to a directory and prints `name sha256` of each. The same bytes
//! must come from the `.wasm` (`tests/test_feat_table_wasm.py`).
//! usage: tabella <out dir> <table.mzfeat> <campioni.json> <spec.json|-> <polarity> [<file.mzidx>...]
use std::fs::File;
use std::path::Path;

use mzlab_core::budget::sha256_hex;
use mzlab_core::idxread::Index;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (out, table, samples) = (Path::new(&a[0]), std::fs::read(&a[1]).expect("table"), std::fs::read_to_string(&a[2]).expect("samples"));
    let spec = if a[3] == "-" { String::new() } else { std::fs::read_to_string(&a[3]).expect("spec") };
    let (pol, size, sha) = (a[4].parse::<i32>().expect("polarity"), table.len() as u64, sha256_hex(&table));
    let put = |name: &str, text: String| {
        std::fs::write(out.join(name), &text).expect("write");
        println!("{name} {}", sha256_hex(text.as_bytes()));
    };
    let r = || Slice(&table);
    put("view.json", core::view_json(&mut r(), size, &samples, &spec).expect("view"));
    put("report.json", core::report_json(&mut r(), size, &samples, &spec, &sha).expect("report"));
    put("qc.json", core::qc_json(&mut r(), size, &samples, &spec).expect("qc"));
    for (name, kind) in [("full.csv", "full"), ("full_mzmine.csv", "full_mzmine"), ("quant.csv", "quant"), ("features.csv", "csv_semicolon"), ("features.rows.json", "xlsx_rows")] {
        put(name, core::table_file(&mut r(), size, &samples, &spec, &sha, pol, kind).expect("file"));
    }
    let open = |k: usize| {
        let f = File::open(&a[5 + k]).expect("index");
        let n = f.metadata().expect("size").len();
        Index::open(f, n)
    };
    for (name, kind) in [("ms2.mgf", "mgf"), ("sirius.ms", "sirius")] {
        put(name, core::ms2_file(&mut r(), size, &samples, &spec, &sha, pol, kind, open).expect("ms2"));
    }
}
