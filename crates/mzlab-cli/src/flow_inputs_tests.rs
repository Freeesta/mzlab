//! The inputs read on several threads (`flow_inputs`): hashes in plan order, and the first input that fails in plan order is the one reported,
//! as when the files were read one at a time.

use mzlab_core::budget::sha256_hex;
use mzlab_core::flow::{FileInput, Plan};

use super::{fill_inputs, map_par};

fn files(tag: &str, n: usize) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_flow_inputs_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for k in 0..n {
        std::fs::write(
            d.join(format!("f{k}.txt")),
            format!("file {k} ").repeat(k + 1),
        )
        .unwrap();
    }
    d
}

fn plan(names: &[&str]) -> Plan {
    let inputs = names.iter().map(|n| FileInput {
        name: n.to_string(),
        ..FileInput::default()
    });
    Plan {
        inputs: inputs.collect(),
        ..Plan::default()
    }
}

#[test]
fn results_keep_the_order_of_the_items() {
    let items: Vec<u64> = (0..200).collect();
    assert_eq!(
        map_par(&items, |x| x * 3),
        (0..200).map(|x| x * 3).collect::<Vec<_>>()
    );
    assert!(map_par(&Vec::<u8>::new(), |x| *x).is_empty());
}

#[test]
fn every_input_gets_the_hash_of_its_own_file() {
    let d = files("hash", 7);
    let names: Vec<String> = (0..7).map(|k| format!("f{k}.txt")).collect();
    let mut p = plan(&names.iter().map(String::as_str).collect::<Vec<_>>());
    p.inputs[2].ty = Some("mzml".into()); // not an mzML: no facets, as before
    fill_inputs(&mut p, |n| d.join(n)).unwrap();
    for (k, f) in p.inputs.iter().enumerate() {
        let body = std::fs::read(d.join(&names[k])).unwrap();
        assert_eq!(
            (f.sha256.as_str(), f.bytes),
            (sha256_hex(&body).as_str(), Some(body.len() as u64))
        );
    }
    assert!(p.inputs[2].observed.resolution.is_empty() && p.inputs[2].observed.polarity.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_first_input_that_fails_in_plan_order_is_reported() {
    let d = files("fail", 5);
    // f1 names a wrong hash, f3 is missing: f1 comes first
    let mut p = plan(&["f0.txt", "f1.txt", "f2.txt", "missing.txt", "f4.txt"]);
    p.inputs[1].sha256 = "0".repeat(64);
    let e = fill_inputs(&mut p, |n| d.join(n)).unwrap_err();
    assert_eq!(e.key, "err.flow.input.sha");
    assert!(e.to_string().contains("f1.txt"), "{e}");
    // the missing file first: its read error is the one reported
    let mut p = plan(&["f0.txt", "missing.txt", "f1.txt"]);
    p.inputs[2].sha256 = "0".repeat(64);
    let e = fill_inputs(&mut p, |n| d.join(n)).unwrap_err();
    assert_ne!(e.key, "err.flow.input.sha");
    assert!(e.to_string().contains("missing.txt"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}
