//! An mzML with MS-Numpress arrays opened by the app: the core reader refuses it (`err.file.numpress`), the module `aperti` reads it (the same reader the site
//! loads on demand), so the app goes through the readers of the registry like for a maker's file. Its answers must be those of the plain twin within the codec's precision.
use mzlab_core::lettori::{FileSet, Membro};
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};
use mzlab_desktop::mapped::Mapped;
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::rpc::MapAt;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

const SCANS: usize = 8;

/// The mzML of the same run written with or without numpress.
fn run(numpress: bool) -> Vec<u8> {
    let h = Header {
        run: "np".into(),
        sources: vec!["np.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let opz = Opzioni {
        numpress,
        ..Default::default()
    };
    let mut w = MzmlWriter::con_opzioni(Vec::new(), &h, opz).unwrap();
    w.open_spectra(SCANS).unwrap();
    for t in 0..SCANS {
        let n = 30 + t;
        let s = Spectrum {
            index: t,
            id: format!("scan={}", t + 1),
            level: 1,
            rt: t as f64 * 0.1,
            polarity: 1,
            mz: (0..n)
                .map(|i| 100.0 + i as f64 * 3.17 + t as f64 * 0.013)
                .collect(),
            intensity: (0..n)
                .map(|i| 500.0 + ((i * 7919 + t * 31) % 997) as f32 * 11.0)
                .collect(),
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// Opens `file` and gives the answers of TIC and of a spectrum: as a plain mzML (a picked file, registered, indexed), or, with `native`, as a dataset of the
/// registry of readers (`natsamples`, then `lr_put` with `nat`), the way a maker's file is opened.
fn answers(dir: &Path, file: &Path, name: &str, native: bool) -> Result<(Value, Value), String> {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    let (fid, _) = st.pick(file.to_path_buf()).unwrap();
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]).map(text).map_err(|e| e.key.to_string());
    if native {
        let members = json!([{"name": name, "fid": fid}]);
        let listed: Value =
            serde_json::from_str(&r("natsamples", json!({"members": members}))?).unwrap();
        assert_eq!(listed["samples"][0]["scans"], SCANS, "{listed}");
        let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
        r("lr_put", json!({"name": name, "nat": nat}))?;
    } else {
        r("lr_put", json!({"name": name, "fid": fid}))?;
    }
    r("lr_index", json!({"name": name}))?;
    r("idxopen", json!({"slot": 0, "name": name}))?;
    let get = |path: &str, q: &str| {
        r("route", json!({"slot": 0, "path": path, "query": q}))
            .map(|t| serde_json::from_str::<Value>(&t).unwrap())
    };
    Ok((
        get("/api/chrom", "?k=0&kind=tic&level=1")?,
        get("/api/spectrum", "?k=0&level=1&rt0=0.2&rt1=0.2")?,
    ))
}

#[test]
fn a_numpress_mzml_opens_in_the_app_and_answers_as_its_plain_twin() {
    let dir = tempfile::tempdir().unwrap();
    let (plain, np) = (dir.path().join("plain.mzML"), dir.path().join("np.mzML"));
    std::fs::write(&plain, run(false)).unwrap();
    std::fs::write(&np, run(true)).unwrap();
    assert!(
        std::fs::read_to_string(&np).unwrap().contains("MS:1002746"),
        "the fixture is numpress"
    );
    let (tic0, sp0) =
        answers(dir.path(), &plain, "plain.mzML", false).expect("the plain twin opens");
    assert_eq!(
        answers(dir.path(), &np, "np.mzML", false),
        Err("err.file.numpress".into())
    );
    let (tic1, sp1) = answers(dir.path(), &np, "np.mzML", true).expect("the numpress file opens");
    // what the site does: the module `aperti` writes the file again as a plain mzML and the engine reads that; the app's native path must say the same
    let again = dir.path().join("again.mzML");
    let map = Mapped::open(&np).unwrap();
    let dim = map.bytes().len() as u64;
    let set = FileSet {
        membri: vec![Membro {
            nome: "np.mzML".into(),
            dim,
            dati: Box::new(MapAt(Arc::new(map))),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &mzlab_core::lettori::Limiti::desktop()).unwrap();
    let mut out = Vec::new();
    mzlab_core::lettori_mzml::scrivi(&mut *l, 0, &mut out, &mut |_, _| {}).unwrap();
    std::fs::write(&again, out).unwrap();
    let (tic2, sp2) =
        answers(dir.path(), &again, "again.mzML", false).expect("the module's mzML opens");
    assert_eq!((&tic1, &sp1), (&tic2, &sp2));
    // and both are the plain twin within the precision of the codecs
    let close = |a: &Value, b: &Value| {
        let (a, b): (Vec<f64>, Vec<f64>) = (
            a.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect(),
            b.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect(),
        );
        a.len() == b.len()
            && !a.is_empty()
            && a.iter()
                .zip(&b)
                .all(|(x, y)| (x - y).abs() <= 0.01 * y.abs() + 1.0)
    };
    assert!(close(&tic1["y"], &tic0["y"]), "{tic1} {tic0}");
    assert!(
        close(&sp1["y"], &sp0["y"]) && sp1["mz"] == sp0["mz"],
        "{sp1} {sp0}"
    );
}

/// `lr_index` of a numpress mzML opened as a dataset says how many spectra a cut file declares beyond the ones it holds, and 0 for a whole file.
#[test]
fn the_index_of_a_truncated_numpress_mzml_reports_the_spectra_it_lacks() {
    let dir = tempfile::tempdir().unwrap();
    let whole = run(true);
    let cut_at = String::from_utf8_lossy(&whole)
        .match_indices("</spectrum>")
        .nth(2)
        .map(|(i, m)| i + m.len())
        .expect("more than three spectra");
    let missing = |bytes: &[u8], name: &str| -> u64 {
        let file = dir.path().join(name);
        std::fs::write(&file, bytes).unwrap();
        let st = State::new();
        st.set_data_folder(dir.path().join(format!("app-{name}")));
        st.set_index_folder(dir.path().join(format!("idx-{name}")))
            .unwrap();
        let (fid, _) = st.pick(file).unwrap();
        let r = |op: &str, p: Value| st.rpc(op, &p, &[]).map(text).unwrap();
        let members = json!([{"name": name, "fid": fid}]);
        let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
        r("lr_put", json!({"name": name, "nat": nat}));
        let j: Value = serde_json::from_str(&r("lr_index", json!({"name": name}))).unwrap();
        j["missing"].as_u64().unwrap()
    };
    assert_eq!(missing(&whole, "whole.mzML"), 0);
    assert!(missing(&whole[..cut_at], "cut.mzML") > 0);
}
