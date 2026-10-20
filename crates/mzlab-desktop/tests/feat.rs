//! mzFinder in the app: the routes `feat/*` and `finder/*` on the high-resolution examples give the same bytes and the same JSON as the steps of the `.wasm`
//! (`mzlab_feat_wasm::run`, `mzlab_feat::legacy`) run directly on the same `.mzidx`: the `.mzfeatf`, the `.mzfeat`, the one with the proofs, the Dig.
use std::fs::File;
use std::path::Path;

use mzlab_core::budget::MemBudget;
use mzlab_core::idxread::Index;
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use mzlab_feat::legacy;
use mzlab_feat::store::Slice;
use mzlab_feat_wasm::run;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const NAMES: [&str; 4] = [
    "HRMS_t000.mzML",
    "HRMS_t005.mzML",
    "HRMS_t010.mzML",
    "HRMS_t030.mzML",
];
const PARENT: &str = "C13H24N4O3S";

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
        .collect::<String>()
}

fn open(path: &Path) -> Index<File> {
    let f = File::open(path).unwrap();
    let size = f.metadata().unwrap().len();
    Index::open(f, size).unwrap()
}

struct Env {
    st: State,
    root: std::path::PathBuf,
    paths: Vec<std::path::PathBuf>,
    _data: tempfile::TempDir,
}

impl Env {
    fn r(&self, op: &str, p: Value) -> String {
        text(self.st.rpc(op, &p, &[]).unwrap())
    }
    fn json(&self, op: &str, p: Value) -> Value {
        serde_json::from_str(&self.r(op, p)).unwrap()
    }
}

/// The four examples registered and indexed by the routes of the page's list.
fn setup() -> Env {
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
    let paths = NAMES
        .iter()
        .map(|n| st.indexer().unwrap().path(n).unwrap())
        .collect();
    Env {
        st,
        root,
        paths,
        _data: data,
    }
}

fn budget() -> MemBudget {
    MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64)
}

/// classify, L1-L3, L4-L6: the `.mzfeatf` of every file and the table, as the direct steps write them.
fn detect_and_align(e: &Env) -> (Vec<Vec<u8>>, Vec<u8>) {
    let cl = e.r("finder/classify", json!({"names": [NAMES[0], "nope.mzML"]}));
    let items = [NAMES[0].to_string(), "nope.mzML".to_string()];
    assert_eq!(
        cl,
        legacy::classify(&items, |n| if n == NAMES[0] {
            Ok(open(&e.paths[0]))
        } else {
            Err("no index of the file: open it again".to_string())
        })
    );
    let rec = run::recipe_for("", &mut open(&e.paths[0])).unwrap();
    let mut stored = Vec::new();
    for (n, p) in NAMES.iter().zip(&e.paths) {
        let direct = run::detect(&mut open(p), n, &rec).unwrap();
        assert_eq!(
            e.json("feat/detect", json!({"name": n, "recipe": ""}))["sha256"],
            sha(&direct),
            "{n}"
        );
        stored.push(direct);
    }
    let mut table = Vec::new();
    run::align_to(
        &mut stored.clone(),
        &budget(),
        &rec,
        |k| Ok(open(&e.paths[k])),
        &mut table,
    )
    .unwrap();
    assert_eq!(
        e.json("feat/align", json!({"names": NAMES, "recipe": ""}))["sha256"],
        sha(&table)
    );
    (stored, table)
}

/// L7-L9: the hypotheses and the table with the proofs; returns the table with `evid` and the registry.
fn hypotheses_and_proofs(
    e: &Env,
    stored: Vec<Vec<u8>>,
    table: &[u8],
    files_json: &str,
    parent_json: &str,
) -> (Vec<u8>, String) {
    let rec = run::recipe_from_json("").unwrap();
    let rules = std::fs::read_to_string(e.root.join("TP_Mine/data/regole_tp.json")).unwrap();
    let hy = e.json(
        "feat/hypotheses",
        json!({"names": NAMES, "rules": rules, "structure": "", "parent": PARENT, "recipe": ""}),
    );
    let mut ix: Vec<_> = e.paths.iter().map(|p| open(p)).collect();
    let h = run::hypotheses_stored(
        &mut ix,
        &mut stored.clone(),
        rules.as_bytes(),
        "",
        PARENT,
        None,
        &rec,
    )
    .unwrap();
    assert_eq!(
        (hy["tried"].as_u64(), hy["found"].as_u64()),
        (Some(h.result.tried.into()), Some(h.result.found.into()))
    );
    let plan = e.json(
        "finder/plan",
        json!({"files": files_json, "parent": parent_json}),
    );
    let minutes: Vec<f64> = plan["minutes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_f64().unwrap_or(f64::NAN))
        .collect();
    let ev = e.json(
        "feat/evidence",
        json!({"minutes": plan["minutes"], "recipe": ""}),
    );
    let mut out = Vec::new();
    run::evidence_to(
        &mut Slice(table),
        table.len() as u64,
        &h,
        &minutes,
        None,
        &rec,
        &budget(),
        &mut out,
    )
    .unwrap();
    assert_eq!(ev["sha256"], sha(&out));
    (out, h.registry.to_json())
}

/// The Dig and what is asked of it, then the views of the table.
fn dig_and_views(e: &Env, proofs: &[u8], registry: &str, files_json: &str, parent_json: &str) {
    let summary = e.r(
        "finder/dig",
        json!({"files": files_json, "parent": parent_json, "registry": registry}),
    );
    let d = legacy::dig(files_json, parent_json, proofs, registry, |k| {
        Ok::<_, String>(open(&e.paths[k]))
    })
    .unwrap();
    let rules = std::fs::read(e.root.join("TP_Mine/data/regole_tp.json")).unwrap();
    let net = mzlab_feat::hypo::Rules::parse(&rules)
        .map_or("null".into(), |r| legacy::network::mass_network(&d, &r));
    assert_eq!(summary, legacy::out::summary_with(&d, "{}", "{}", &net));
    assert_eq!(
        e.r("finder/inclusion_csv", json!({"id": 0})),
        legacy::out::inclusion_csv(&d, 0)
    );
    assert_eq!(
        e.r("finder/frag_tree", json!({})),
        legacy::fragtree::of_dig(&d)
    );
    if let Some(c) = d.cands.first() {
        assert_eq!(
            e.r("finder/detail", json!({"id": c.row})),
            legacy::out::detail(&d, c.id()).unwrap()
        );
    }
    let piece =
        e.st.rpc(
            "feat/table",
            &json!({"kind": "bytes", "off": 0, "len": 8}),
            &[],
        )
        .unwrap();
    assert!(matches!(piece, Answer::Bytes(b) if b.len() == 8));
    let list: Vec<Value> = NAMES
        .iter()
        .map(|n| json!({"name": n, "sha256": ""}))
        .collect();
    let samples = e.r(
        "feat/table",
        json!({"kind": "samples", "files": json!({"files": list}).to_string()}),
    );
    assert!(samples.contains("HRMS_t000"), "{samples}");
    assert!(e
        .r(
            "feat/table",
            json!({"kind": "view", "samples": samples, "spec": ""})
        )
        .contains("\"righe\""));
    assert!(e.st.rpc("feat/nope", &json!({}), &[]).is_err());
    assert!(e
        .r(
            "finder/formula_info",
            json!({"text": PARENT, "adduct": "[M+H]+"})
        )
        .contains("mz"));
}

#[test]
fn routes_give_the_bytes_and_the_json_of_the_steps() {
    let e = setup();
    let (stored, table) = detect_and_align(&e);
    let files: Vec<Value> = NAMES
        .iter()
        .map(|n| {
            let g = legacy::sample::guess(n);
            json!({"name": n, "type": g.kind, "time": g.time, "kind": "full"})
        })
        .collect();
    let files_json = Value::Array(files).to_string();
    let parent_json = json!({"name": "parent", "neutral": PARENT, "adduct": "[M+H]+"}).to_string();
    let (proofs, registry) = hypotheses_and_proofs(&e, stored, &table, &files_json, &parent_json);
    dig_and_views(&e, &proofs, &registry, &files_json, &parent_json);
}

#[test]
fn detect_of_several_files_at_once_gives_the_same_bytes() {
    let e = setup();
    let rec = run::recipe_for("", &mut open(&e.paths[0])).unwrap();
    let direct: Vec<String> = NAMES
        .iter()
        .zip(&e.paths)
        .map(|(n, p)| sha(&run::detect(&mut open(p), n, &rec).unwrap()))
        .collect();
    let together: Vec<String> = std::thread::scope(|s| {
        let hs: Vec<_> = NAMES
            .iter()
            .map(|n| {
                let e = &e;
                s.spawn(move || {
                    e.json("feat/detect", json!({"name": n, "recipe": ""}))["sha256"]
                        .as_str()
                        .unwrap()
                        .to_string()
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(together, direct);
}

#[test]
fn preview_and_version_routes_equal_the_wasm_functions() {
    let e = setup();
    assert_eq!(
        e.r("feat/version", json!({})),
        format!("\"{}\"", mzlab_feat::CRATE_VERSION)
    );
    for recipe in ["", r#"{"floor":1234.5}"#] {
        let mut ix = open(&e.paths[0]);
        let r = run::recipe_for(recipe, &mut ix).unwrap();
        let direct = run::preview(&mut ix, &r, run::floor_given(recipe)).unwrap();
        assert_eq!(
            e.r("feat/preview", json!({"name": NAMES[0], "recipe": recipe})),
            direct,
            "recipe {recipe:?}"
        );
    }
}

#[test]
fn files_route_lists_the_indexed_files() {
    let e = setup();
    let mut want: Vec<&str> = NAMES.to_vec();
    want.sort();
    assert_eq!(e.json("feat/files", json!({})), json!(want));
}

#[test]
fn polpair_route_pairs_the_held_run_with_the_kept_one_and_holds_the_kept_one_again() {
    let e = setup();
    let (_, table) = detect_and_align(&e);
    // nothing kept: the state error, and the held table stays
    assert!(e
        .st
        .rpc("feat/polpair", &json!({"step": "pair", "pol": 1}), &[])
        .is_err());
    e.r("feat/polpair", json!({"step": "stash"}));
    // the run of the "other polarity" replaces the held table (here the same files: the pairing finds no pair)
    e.json("feat/align", json!({"names": NAMES, "recipe": ""}));
    let pairs = e.json("feat/polpair", json!({"step": "pair", "pol": 1}));
    assert_eq!(pairs["schema"], "polpair/1");
    let held = text(
        e.st.rpc(
            "feat/table",
            &json!({"kind": "bytes", "off": 0, "len": table.len()}),
            &[],
        )
        .map(|a| match a {
            Answer::Bytes(b) => Answer::Text(sha(&b)),
            t => t,
        })
        .unwrap(),
    );
    assert_eq!(held, sha(&table));
}
