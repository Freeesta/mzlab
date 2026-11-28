//! The compound search (`molecules/1`, op `lr_molecules`) over the one built-in catalogue in the core, the app, the CLI (`mzlab-cli mol find`) and, when
//! `MZLAB_WASM_PKG` names the built `.wasm` (see `parity_routes.rs`), the module of the site: the same file gives the same answer bytes everywhere. The wasm leg is run, and must pass, by `tests/test_parita_rotte_app.py`.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// The engine keeps one archive per process: the calls of this file take turns.
static LOCK: Mutex<()> = Mutex::new(());

const CATALOGUE: &str = include_str!("../../../mzlab/web/molecules/catalogue.csv");

/// What a person types: names, prefixes, a formula, a mass, a removed code, a typing slip, nothing.
const QUERIES: &[&str] = &[
    "dimeth",
    "dimethomorph",
    "sd",
    "azoxystrobin",
    "tebuconazole",
    "imidacloprid",
    "atrazine",
    "carbamazepine",
    "diclofenac",
    "bisphenol",
    "C21H22ClNO4",
    "387.1237",
    "paracetamol",
    "LSD",
    "dimethomorf",
    "zzzzqq",
    "x",
];

fn ask(q: &str, wide: bool) -> String {
    json!({"q": q, "limit": 8, "ppm": 10, "wide": wide}).to_string()
}

fn on_app(st: &State, mop: &str, body: &str) -> Result<String, String> {
    match st.rpc("lr_molecules", &json!({"mop": mop, "body": body}), &[]) {
        Ok(Answer::Text(t)) => Ok(t),
        Ok(Answer::Bytes(_)) => panic!("bytes"),
        Err(e) => Err(e.key.to_string()),
    }
}

fn on_core(mop: &str, body: &str) -> Result<String, String> {
    mzlab_core::molecules::route(mop, body).map_err(|e| e.key.to_string())
}

/// Without a user list: `load` keeps what it is not given, so the tests of this file reset it.
fn load_body() -> String {
    json!({ "catalog": CATALOGUE, "user": {"schema": "molecules/1", "molecules": []} }).to_string()
}

#[test]
fn the_app_and_the_cli_give_the_bytes_of_the_core() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let st = State::new();
    on_core("load", &load_body()).unwrap();
    on_app(&st, "load", &load_body()).unwrap();
    let mut found = 0;
    for q in QUERIES {
        for wide in [false, true] {
            let body = ask(q, wide);
            let core = on_core("search", &body);
            assert_eq!(on_app(&st, "search", &body), core, "app {q} wide={wide}");
            let mut args: Vec<String> = ["mol", "find", q, "--set", "limit=8"]
                .map(String::from)
                .to_vec();
            if wide {
                args.extend(["--set".into(), "wide=1".into()]);
            }
            let o = mzlab_cli::execute(&args);
            assert_eq!(o.code, 0, "cli {q}: {}", o.stderr);
            assert_eq!(
                String::from_utf8(o.stdout).unwrap(),
                format!("{}\n", core.as_ref().unwrap()),
                "cli {q} wide={wide}"
            );
            found += (core.unwrap() != "[]") as usize;
        }
    }
    assert!(
        found >= 20,
        "the queries that name a compound of the catalogue find it ({found})"
    );
}

/// Two invented molecules of a lab, as the CSV a person exports: one the catalogue does not know, one with a catalogue name in a synonym.
const USER_CSV: &str =
    "name;synonyms;molecular_formula\nExampline;Exa-1;C10H15NO\nProvolone B;;C12H17NO2\n";
const USER_QUERIES: &[&str] = &[
    "examp",
    "Exa-1",
    "C12H17NO2",
    "provolone",
    "dimethomorph",
    "zzzzqq",
];

fn user_doc() -> String {
    let ms = mzlab_core::molecules::from_csv(USER_CSV, "user", None).unwrap();
    mzlab_core::molecules::to_json(&ms)
}

fn ask_levels(q: &str) -> String {
    json!({"q": q, "limit": 8, "ppm": 10, "levels": true}).to_string()
}

#[test]
fn a_user_molecule_gives_the_same_bytes_in_the_core_the_app_and_the_cli() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let st = State::new();
    let load = json!({"catalog": CATALOGUE, "user": user_doc()}).to_string();
    on_core("load", &load).unwrap();
    on_app(&st, "load", &load).unwrap();
    let file = std::env::temp_dir().join(format!("parity_user_{}.csv", std::process::id()));
    std::fs::write(&file, USER_CSV).unwrap();
    let mut levels = Vec::new();
    for q in USER_QUERIES {
        let body = ask_levels(q);
        let core = on_core("search", &body);
        assert_eq!(on_app(&st, "search", &body), core, "app {q}");
        let args: Vec<String> = ["mol", "find", q, "--user", file.to_str().unwrap()]
            .map(String::from)
            .to_vec();
        let o = mzlab_cli::execute(&args);
        assert_eq!(o.code, 0, "cli {q}: {}", o.stderr);
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            format!("{}\n", core.as_ref().unwrap()),
            "cli {q}"
        );
        levels.push(core.unwrap());
    }
    std::fs::remove_file(&file).ok();
    assert!(levels[0].contains("\"level\":\"user\""), "{}", levels[0]);
    assert!(levels[4].contains("\"level\":\"catalog\""), "{}", levels[4]);
    assert_eq!(levels[5], "[]");
}

#[test]
fn a_body_that_is_not_json_is_the_same_error_key_in_the_core_and_the_app() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let st = State::new();
    let core = on_core("search", "not json");
    assert!(core.is_err());
    assert_eq!(on_app(&st, "search", "not json"), core);
}

/// The wasm leg runs inside `tests/test_parita_rotte_app.py`, which builds the package and sets `MZLAB_WASM_PKG`; a plain `cargo test` has no package and
/// leaves it to that test, as every other parity test of this folder does.
#[test]
fn the_app_gives_the_same_bytes_as_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut plan = vec![json!({"op": "lr_molecules", "mop": "load", "body": load_body()})];
    let mut bodies = Vec::new();
    for q in QUERIES {
        for wide in [false, true] {
            bodies.push(ask(q, wide));
        }
    }
    bodies.push("not json".to_string());
    plan.extend(
        bodies
            .iter()
            .map(|b| json!({"op": "lr_molecules", "mop": "search", "body": b})),
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("plan.json");
    std::fs::write(&file, serde_json::to_string(&plan).unwrap()).unwrap();
    let empty = dir.path().join("empty");
    std::fs::write(&empty, b"").unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .arg(&pkg)
        .arg(&empty)
        .arg(&empty)
        .arg(&file)
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(wasm[0]["s"].is_string(), "load: {}", wasm[0]);
    let st = State::new();
    on_app(&st, "load", &load_body()).unwrap();
    for (body, w) in bodies.iter().zip(&wasm[1..]) {
        let app = on_app(&st, "search", body);
        let site = match w["s"].as_str() {
            Some(s) => Ok(s.to_string()),
            None => {
                let e = w["e"].as_str().unwrap_or("");
                Err(serde_json::from_str::<Value>(e)
                    .ok()
                    .and_then(|j| j["error_key"].as_str().map(String::from))
                    .unwrap_or_else(|| e.to_string()))
            }
        };
        assert_eq!(app, site, "{}", &body[..body.len().min(60)]);
    }
}

/// The same for a user molecule: load the catalogue and the user's document, then search with the levels.
#[test]
fn a_user_molecule_gives_the_same_bytes_in_the_app_and_the_wasm() {
    let Ok(pkg) = std::env::var("MZLAB_WASM_PKG") else {
        return;
    };
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let load = json!({"catalog": CATALOGUE, "user": user_doc()}).to_string();
    let mut plan = vec![json!({"op": "lr_molecules", "mop": "load", "body": load})];
    let bodies: Vec<String> = USER_QUERIES.iter().map(|q| ask_levels(q)).collect();
    plan.extend(
        bodies
            .iter()
            .map(|b| json!({"op": "lr_molecules", "mop": "search", "body": b})),
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("plan.json");
    std::fs::write(&file, serde_json::to_string(&plan).unwrap()).unwrap();
    let empty = dir.path().join("empty");
    std::fs::write(&empty, b"").unwrap();
    let out = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/rotte_wasm_node.js"))
        .arg(&pkg)
        .arg(&empty)
        .arg(&empty)
        .arg(&file)
        .output()
        .expect("node runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wasm: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(wasm[0]["s"].is_string(), "load: {}", wasm[0]);
    let st = State::new();
    on_app(&st, "load", &load).unwrap();
    for (body, w) in bodies.iter().zip(&wasm[1..]) {
        assert_eq!(
            on_app(&st, "search", body),
            Ok(w["s"].as_str().unwrap().to_string()),
            "{body}"
        );
    }
}
