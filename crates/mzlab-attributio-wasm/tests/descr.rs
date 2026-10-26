//! The Crippen descriptors the RT-logP check of the Banco computes: one JSON line per SMILES of `SMILES`, printed for the parity test with the `.wasm`
//! (tests/test_attributio_descr_wasm.py); `values_are_the_known_ones` pins a few of them.
#![cfg(feature = "rev")]
use mzlab_attributio_wasm::rt_descriptors_json;

#[test]
fn values_are_the_known_ones() {
    let j: serde_json::Value = serde_json::from_str(&rt_descriptors_json("CCO")).unwrap();
    assert_eq!(j["tpsa"], 20.23);
    assert_eq!(j["hbd"], 1.0);
    assert_eq!(rt_descriptors_json("not a smiles("), "{}");
}

#[test]
fn print_for_parity() {
    if let Ok(list) = std::env::var("MZLAB_DESCR_SMILES") {
        let all: Vec<String> = list.split('\n').map(rt_descriptors_json).collect();
        std::fs::write(
            std::env::var("MZLAB_DESCR_OUT").unwrap(),
            format!("[{}]", all.join(",")),
        )
        .unwrap();
    }
}
