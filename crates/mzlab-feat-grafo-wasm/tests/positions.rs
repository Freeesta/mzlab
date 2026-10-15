//! The `positions` of the bridge answers the `posiz/1` of `mzlab_feat::chem::posizioni::positions_json` (the error path builds a `JsValue`, which only exists in a browser).
use mzlab_feat_grafo_wasm::{positions, version};

// 4-methyldiphenylamine + O: the fragment C7H8O (tolyl + O, +H) 109.0648 carries the oxygen on ring A, C12H11NO 186.0913 is everything but the methyl.
const DPA: &str = "Cc1ccc(Nc2ccccc2)cc1";

#[test]
fn bridge_gives_the_same_json_as_the_library() {
    let peaks = "[[109.0648,100],[186.0913,50]]";
    let j =
        positions(DPA, "+O", peaks, 5.0, true, true).unwrap_or_else(|_| panic!("positions failed"));
    assert_eq!(
        j,
        mzlab_feat::chem::posizioni::positions_json(DPA, "+O", peaks, 5.0, true, true).unwrap()
    );
    for k in [
        "\"format\":\"posiz\"",
        "\"status\":\"supported\"",
        "\"sublevel\":\"3a\"",
        "\"atoms\":[2,3,12,13]",
    ] {
        assert!(j.contains(k), "{k} in {j}");
    }
}

#[test]
fn bridge_version_names_the_current_contract() {
    assert!(version().starts_with("mzfeat-grafo/4"));
}
