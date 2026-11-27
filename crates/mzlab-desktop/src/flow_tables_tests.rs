//! The route `flow/live/tables` of the app: no run is an empty `tables/1`, a run folder gives the document `mzlab-cli flow tables` builds (the step of `prov.json`, a file that is no table named).

use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};
use serde_json::json;

use crate::engine::State;
use crate::rpc::Answer;

fn answer(r: Answer) -> String {
    match r {
        Answer::Text(t) => t,
        _ => panic!("a text"),
    }
}

fn table() -> Vec<u8> {
    let mut t = Aligned::with_capacity(3, 2);
    for (mz, rt) in [(200.1, 1.5), (250.2, 2.5)] {
        t.mz.push(mz);
        t.rt.push(rt);
        t.area.push(1e6);
        t.height.push(1e5);
        t.apex_rt.push(rt as f32);
        t.width.push(0.1);
        t.state.push(CELL_DETECTED);
    }
    let mut r = Recipe::default();
    r.detect.polarity = Some(1);
    write_table(&t, &r, &Origin::default())
}

#[test]
fn no_run_is_an_empty_list_and_a_run_folder_is_the_document_of_the_cli() {
    let st = State::new();
    let root = tempfile::tempdir().unwrap();
    st.set_flow_folder(root.path().join("flow"));
    let empty = answer(st.flow_live("tables", &json!({})).unwrap());
    assert!(
        empty.contains(r#""schema":"tables/1""#)
            && empty.contains(r#""tables":[]"#)
            && empty.contains(r#""skipped":[]"#),
        "{empty}"
    );

    let out = root.path().join("flow").join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("gap.mzfeat"), table()).unwrap();
    std::fs::write(out.join("bad.mzfeat"), "no").unwrap();
    std::fs::write(
        out.join("prov.json"),
        r#"{"schema":"prov/1","entity":{},"activity":{"gap":{"step":"features.gapfill@1","ms":12.34,"warnings":[]}}}"#,
    )
    .unwrap();
    let got = answer(st.flow_live("tables", &json!({})).unwrap());
    let (want, skipped) = mzlab_cli::flow_report::tables::document_of(&out).unwrap();
    assert_eq!(got, want, "the app and the CLI give the same bytes");
    assert!(
        got.contains(r#""name":"gap.mzfeat""#)
            && got.contains(r#""step":"features.gapfill@1""#)
            && got.contains(r#""rows":2"#),
        "{got}"
    );
    assert_eq!(skipped.len(), 1);
    assert!(
        got.contains("bad.mzfeat"),
        "the file that is no table is named: {got}"
    );
}
