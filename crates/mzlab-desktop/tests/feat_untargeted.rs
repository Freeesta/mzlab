//! The routes `feat/untargeted/*` of the app give the same bytes as the steps of the `.wasm` `untargeted` (`mzlab_feat_wasm::untargeted`) run directly on the same
//! `.mzidx`: the `.mzfeatf` of every file and the `.mzfeat` of the series, on the high-resolution examples.
use std::fs::File;
use std::path::Path;

use mzlab_core::budget::MemBudget;
use mzlab_core::idxread::Index;
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use mzlab_feat::untargeted::pipeline::UntParams;
use mzlab_feat_wasm::untargeted;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const NAMES: [&str; 4] = [
    "HRMS_t000.mzML",
    "HRMS_t005.mzML",
    "HRMS_t010.mzML",
    "HRMS_t030.mzML",
];
// the examples are small and weak: thresholds well under the defaults of the batch of mzmine
const PARAMS: &str = r#"{"engine":"untargeted","adap":{"noise":1000,"min_group_intensity":1000,"min_highest":3000},"resolve":{"min_abs_height":3000}}"#;

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn open(path: &Path) -> Index<File> {
    let f = File::open(path).unwrap();
    let size = f.metadata().unwrap().len();
    Index::open(f, size).unwrap()
}

struct Held(Vec<Vec<u8>>);

impl mzlab_feat::slabs::Stored for Held {
    fn count(&self) -> usize {
        self.0.len()
    }
    fn bytes(&mut self, k: usize) -> mzlab_core::Result<Vec<u8>> {
        Ok(self.0[k].clone())
    }
}

#[test]
fn the_routes_give_the_bytes_of_the_steps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(data.path().join("app"));
    st.set_index_folder(data.path().join("idx")).unwrap();
    for n in NAMES {
        let (id, _) = st.pick(root.join("mzlab/web/esempi").join(n)).unwrap();
        st.rpc("lr_put", &json!({"name": n, "fid": id}), &[])
            .unwrap();
        st.rpc("lr_index", &json!({"name": n}), &[]).unwrap();
    }
    let paths: Vec<_> = NAMES
        .iter()
        .map(|n| st.indexer().unwrap().path(n).unwrap())
        .collect();
    let p = UntParams::from_json(PARAMS).unwrap();
    let call = |op: &str, v: Value| -> Value {
        serde_json::from_str(&text(st.rpc(op, &v, &[]).unwrap())).unwrap()
    };
    let mut stored = Vec::new();
    for (n, path) in NAMES.iter().zip(&paths) {
        let mut direct = Vec::new();
        untargeted::detect_to(&mut open(path), n, &p, &mut direct).unwrap();
        let got = call(
            "feat/untargeted/detect",
            json!({"name": n, "params": PARAMS}),
        );
        assert_eq!(got["sha256"], sha(&direct), "{n}");
        stored.push(direct);
    }
    let mut table = Vec::new();
    let budget = MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
    untargeted::align_to(
        &mut Held(stored),
        &budget,
        &p,
        |k| Ok(open(&paths[k])),
        &mut table,
    )
    .unwrap();
    let got = call(
        "feat/untargeted/align",
        json!({"names": NAMES, "params": PARAMS}),
    );
    assert_eq!(got["sha256"], sha(&table));
    assert_eq!(got["bytes"], table.len());
    // a recipe of another engine is refused with the key of the page
    let bad = st.rpc(
        "feat/untargeted/detect",
        &json!({"name": NAMES[0], "params": "{\"engine\":\"x\"}"}),
        &[],
    );
    assert!(bad.is_err());
}
