//! `natregistro` (one series of the instrument log of a `.raw`, `/api/registro` of `lr-method.js`) is answered by the app as by the readers module of the
//! site: the same reader (`mzlab_lettori::apri`) and the same writer (`mzlab_lettori::json::cromatogramma_json`, which `Sessioni::cromatogramma` of
//! `mzlab-lettori-wasm` calls), byte for byte. The synthetic `.raw` (`comune/thermo.rs`, invented numbers) has no status log: the TIC id walks the same
//! path (`rawlettore::cromatogramma_di`), an id the file lacks gives the reader's error, a name that is not a maker's sample `err.session.fileNotFound`.
#[path = "../../mzlab-lettori/tests/comune/thermo.rs"]
mod thermo;

use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// What the site's module gives for `cromatogramma(0, id)` on the same file.
fn module_answer(path: &std::path::Path, id: &str) -> Result<String, &'static str> {
    let b = std::fs::read(path).unwrap();
    let set = FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::desktop()).unwrap();
    l.cromatogramma(0, id)
        .map(|c| mzlab_lettori::json::cromatogramma_json(&c))
        .map_err(|e| e.key)
}

#[test]
fn the_app_answers_natregistro_as_the_readers_module() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("x.raw");
    std::fs::write(&raw, thermo::Raw::tsq(63, 10).bytes()).unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let err = |p: Value| match st.rpc("natregistro", &p, &[]) {
        Ok(_) => panic!("answered {p}"),
        Err(e) => e.key,
    };
    let (fid, _) = st.pick(raw.clone()).unwrap();
    let members = json!([{"name": "x.raw", "fid": fid}]);
    let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
    r("lr_put", json!({"name": "x.raw", "nat": nat})).unwrap();

    let tic = text(r("natregistro", json!({"name": "x.raw", "serie": "TIC"})).unwrap());
    assert_eq!(tic, module_answer(&raw, "TIC").unwrap());
    assert!(tic.starts_with("{\"id\":\"TIC\",\"kind\":\"Tic\""), "{tic}");
    // twice: the reader is opened anew for each request, the bytes do not change
    assert_eq!(
        tic,
        text(r("natregistro", json!({"name": "x.raw", "serie": "TIC"})).unwrap())
    );

    let missing = "status:Ion Source/Spray Voltage (kV)";
    assert_eq!(
        Err(err(json!({"name": "x.raw", "serie": missing}))),
        module_answer(&raw, missing)
    );
    assert_eq!(
        err(json!({"name": "other.raw", "serie": "TIC"})),
        "err.session.fileNotFound"
    );
}
