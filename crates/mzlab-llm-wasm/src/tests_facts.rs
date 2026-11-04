//! The facts without a model: `inspect_build` gives the bytes of `mzlab_llm::inspect` and of its golden file on the synthetic files (skipped without
//! python3 and numpy), `params_propose` with the registry `docs/schemi/steps-1.json` gives the proposal the CLI writes with `flow::steps()`.

#[path = "../../mzlab-llm/tests/ispez/mod.rs"]
mod ispez;

use super::*;
use ispez::*;

fn data(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/../mzlab-llm/tests/data/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn inspect_build_gives_the_golden_bytes() {
    let Some(dir) = synthetic() else { return };
    let src = sources(dir, &KINDS, &|_| String::new(), false);
    let text = stats_text(&src);
    let out = inspect_build(&text, 30000).unwrap();
    assert_eq!(out, mzlab_llm::inspect::build(&src, 30000).unwrap());
    let lines: Vec<String> = files_of(&out).iter().map(stats_text).collect();
    assert_eq!(lines.join("\n") + "\n", data("ispezione_sintetici.json"));
}

#[test]
fn inspect_build_refuses_the_course_profile_as_json() {
    let e = inspect_build(r#"{"profile":"corso","files":[]}"#, 1000).unwrap_err();
    assert!(e.contains("err.llm.profile.course"), "{e}");
}

#[test]
fn params_propose_gives_the_bytes_of_the_cli() {
    let steps = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemi/steps-1.json"
    ))
    .unwrap();
    let out = params_propose(data("ispezione_g01.json").trim_end(), "", &steps).unwrap();
    assert_eq!(out + "\n", data("proposta_g01.json"));
    let e = params_propose("not json", "", &steps).unwrap_err();
    assert!(e.contains("err.llm.params.input"), "{e}");
}
