//! `hilite/1` on the synthetic drawing of `tests/dati/hilite_disegno.json` (the fixture the molkit `.wasm`, the app and `mzlab-cli mol-hilite` are checked
//! against too): the HR request colours every layer, the LR one only what is selected, and every bad request gives its error key.
use mzlab_feat::chem::hilite::hilite_json;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../tests/dati/hilite_disegno.json")).unwrap()
}

#[test]
fn the_requests_give_the_bytes_of_the_fixture() {
    let fx = fixture();
    for (req, want) in [("request", "hilite"), ("lr", "hilite_lr")] {
        assert_eq!(
            hilite_json(&fx[req].to_string()).unwrap(),
            fx[want].as_str().unwrap(),
            "{req}"
        );
    }
}

#[test]
fn the_hr_layers_land_on_the_atoms_of_the_drawing() {
    let fx = fixture();
    let r: Value = serde_json::from_str(fx["hilite"].as_str().unwrap()).unwrap();
    let l = r["layers"].as_array().unwrap();
    // atom 0 of the drawing is Cl and atom 5 is O: the sites O and Cl of the SMILES, numbered the other way round, land on them
    assert_eq!(l[1]["atoms"], serde_json::json!([0, 5]));
    // the C-Cl bond of the SMILES (4-5) is the bond 0-1 of the drawing
    assert_eq!(l[2]["bonds"], serde_json::json!([[0, 1]]));
    assert_eq!(l[3]["skip"], "noMatch");
}

#[test]
fn a_lr_request_without_a_selection_colours_nothing() {
    let mut req = fixture()["lr"].clone();
    req["selected"] = serde_json::json!([]);
    let r: Value = serde_json::from_str(&hilite_json(&req.to_string()).unwrap()).unwrap();
    for l in r["layers"].as_array().unwrap() {
        assert!(
            l["atoms"].as_array().unwrap().is_empty() && l["bonds"].as_array().unwrap().is_empty()
        );
        assert!(l["skip"] == "lr" || l["skip"] == "noMatch", "{l}");
    }
}

#[test]
fn bad_requests_give_their_keys() {
    for e in fixture()["errors"].as_array().unwrap() {
        let key = e[1].as_str().unwrap();
        assert_eq!(
            hilite_json(&e[0].to_string()).err().map(|x| x.key),
            Some(key),
            "{e}"
        );
    }
    assert_eq!(
        hilite_json("{").err().map(|x| x.key),
        Some("error.hilite.request")
    );
}
