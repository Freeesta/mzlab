//! Prints `reatt/1` for each "name|SMILES[|opts]" argument as `{"name":..., "reatt":...}` lines (opts: engine JSON, e.g. {"solvent":"none"}).
//! Used by `tools/fukui_valida/valida.py`; no product code.

fn main() {
    for arg in std::env::args().skip(1) {
        let mut it = arg.splitn(3, '|');
        let (name, smiles, opts) = (it.next().unwrap_or(""), it.next().unwrap_or(""), it.next().unwrap_or(""));
        match mzlab_chem::reactivity_json(smiles, opts) {
            Ok(j) => println!("{{\"name\":\"{name}\",\"reatt\":{j}}}"),
            Err(e) => println!("{{\"name\":\"{name}\",\"error\":{:?}}}", format!("{e:?}")),
        }
    }
}
