//! Native reference of the bridge: prints the `feature-slots/1` of one row of a table, the same bytes the `.wasm` gives (`tests/test_feat_slots_wasm.py`).
//! usage: slots <table.mzfeat> <samples.json> <id> <output name>
use mzlab_feat::store::Slice;
use mzlab_ops::run::table::core;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let table = std::fs::read(&a[0]).expect("table");
    let samples = std::fs::read_to_string(&a[1]).expect("samples");
    let id: u32 = a[2].parse().expect("id");
    let out = core::slots_json(&mut Slice(&table), table.len() as u64, &samples, id, &a[3]).expect("slots");
    print!("{out}");
}
