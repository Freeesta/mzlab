//! The steps behind `detect_untargeted_to` and `align_untargeted_to` (`untargeted::{detect_to, align_to}`) over the readers and sinks that the `.wasm` and the
//! native example use: the bytes do not depend on where they go (a `Vec`, a file written at offsets, the hashing sink), the series equals the table of the
//! stages run by hand, and a budget that holds nothing is refused in order. The synthetic series is the one of `tools/bench/oro_mzmine.py --genera` (python3 with
//! numpy; skipped without). The byte-for-byte comparison of the `.wasm` with the example runs in node: `tests/test_feat_wasm_untargeted.py`.
#![cfg(feature = "untargeted")]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use mzlab_core::budget::{Hashing, MemBudget};
use mzlab_core::error::Result;
use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::slabs::Stored;
use mzlab_feat::store::read_table_from;
use mzlab_feat::untargeted::pipeline::{run_file, run_series, UntParams};
use mzlab_feat_wasm::untargeted::{align_to, detect_to};

struct Held(Vec<Vec<u8>>);

impl Stored for Held {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
}

fn index_bytes(path: &Path) -> Vec<u8> {
    let mzml = std::fs::read(path).unwrap();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0
}

/// The four indexes of the synthetic series, made once (the tests run in parallel) and opened afresh at every call.
fn series() -> Option<Vec<Index<Vec<u8>>>> {
    static BYTES: OnceLock<Option<Vec<Vec<u8>>>> = OnceLock::new();
    let b = BYTES.get_or_init(make).as_ref()?;
    Some(
        b.iter()
            .map(|x| Index::open(x.clone(), x.len() as u64).unwrap())
            .collect(),
    )
}

fn make() -> Option<Vec<Vec<u8>>> {
    let dir = std::env::temp_dir().join(format!("mzlab_untargeted_wasm_{}", std::process::id()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/bench/oro_mzmine.py"))
        .arg("--genera")
        .arg(&dir)
        .current_dir(&root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("SKIP untargeted_wasm: python3/numpy not available");
        return None;
    }
    let v = (1..=4)
        .map(|i| index_bytes(&dir.join(format!("oro_{i}.mzML"))))
        .collect();
    let _ = std::fs::remove_dir_all(&dir);
    Some(v)
}

fn recipe() -> UntParams {
    UntParams::from_json(r#"{"engine":"untargeted","polarity":1}"#).unwrap()
}

#[test]
fn the_bytes_do_not_depend_on_the_sink_and_equal_the_stages_by_hand() {
    let Some(mut ixs) = series() else { return };
    let p = recipe();
    let mut stored = Vec::new();
    for (k, ix) in ixs.iter_mut().enumerate() {
        let name = format!("oro_{}", k + 1);
        let (mut a, mut b) = (Vec::new(), Hashing::new(Vec::new()));
        detect_to(ix, &name, &p, &mut a).unwrap();
        detect_to(ix, &name, &p, &mut b).unwrap();
        let (inner, n, sha) = b.finish();
        assert_eq!((inner.as_slice(), n as usize), (a.as_slice(), a.len()));
        assert_eq!(sha.len(), 64);
        stored.push(a);
    }
    let mut table = Vec::new();
    let mut open_ixs: Vec<_> = series().unwrap().into_iter().map(Some).collect();
    align_to(
        &mut Held(stored),
        &MemBudget::from_request(f64::NAN),
        &p,
        |k| Ok(open_ixs[k].take().unwrap()),
        &mut table,
    )
    .unwrap();
    // the stages by hand give the same table
    let lists: Vec<_> = ixs.iter_mut().map(|ix| run_file(ix, &p).unwrap()).collect();
    let names: Vec<String> = (1..=4).map(|i| format!("oro_{i}")).collect();
    let by_hand = run_series(&lists, &mut ixs, &names, &p).unwrap();
    let (t, _) =
        read_table_from(&mut mzlab_feat::store::Slice(&table), table.len() as u64).unwrap();
    assert_eq!(t.len(), by_hand.len());
    assert_eq!(t.mz, by_hand.mz);
    assert_eq!(t.state, by_hand.state);
    assert_eq!(t.area, by_hand.area);
}

#[test]
fn a_budget_that_holds_nothing_is_refused_in_order() {
    let Some(mut ixs) = series() else { return };
    let p = recipe();
    let mut one = Vec::new();
    detect_to(&mut ixs[0], "oro_1", &p, &mut one).unwrap();
    let mut open_ixs: Vec<_> = series().unwrap().into_iter().map(Some).collect();
    let err = align_to(
        &mut Held(vec![one; 4]),
        &MemBudget::mb(0.0001),
        &p,
        |k| Ok(open_ixs[k].take().unwrap()),
        &mut Vec::new(),
    )
    .unwrap_err();
    assert_eq!(err.key, "error.memoryBudget");
}

/// The `.wasm` export `untargeted_formula_id` gives the JSON `formula_id/1` of `untargeted::formula_id` (caffeine, [M+H]+, the pattern from the natural abundances).
#[test]
fn formula_id_export_gives_the_json_of_the_module() {
    let mz = vec![195.0876520311805, 196.09015386499837, 197.09225064161305];
    let y = vec![1000000.0, 103166.01515119415, 8936.76966493754];
    let text = mzlab_feat_wasm::untargeted_formula_id(mz, y, 195.0876520311805, "").unwrap();
    assert!(text.starts_with("{\"v\":\"formula_id/1\""), "{text}");
    assert!(text.contains("\"neutral\":\"C8H10N4O2\""), "{text}");
    assert!(text.contains("\"chosen\":0"));
}
