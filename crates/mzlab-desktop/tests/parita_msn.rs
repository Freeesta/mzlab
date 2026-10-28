//! The ops `msngrafo`, `mzml2cen`, `lr_index_cen` and `natsrm` of the desktop engine give the bytes the site's `.wasm` gives: the graph and its exports are those of
//! `mzlab-cli msn-graph` (which `tests/test_msngrafo_wasm.py` holds equal to the `.wasm`), the centroid copy that of `mzlab_core::centroidi` (what `mzml2cen` of
//! `rust-worker.js` runs), the SRM bytes `srm::impacchetta` of the reader. Public synthetic files only.
#[path = "../../mzlab-core/tests/agilent_synth/mod.rs"]
mod agilent_synth;

use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

fn state(dir: &Path) -> State {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    st
}

fn mzml() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

/// What `mzlab-cli msn-graph <msn.mzML> <args>` prints (the line end included).
fn cli(args: &[&str]) -> String {
    let mut v = vec![mzml().to_string_lossy().into_owned()];
    v.extend(args.iter().map(|s| s.to_string()));
    let a = mzlab_cli::msn_graph::parse_args(v.into_iter())
        .unwrap()
        .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    mzlab_cli::msn_graph::esegui(&a, tmp.path()).unwrap()
}

#[test]
fn the_graph_and_its_exports_are_those_of_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let st = state(dir.path());
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let (fid, _) = st.pick(mzml()).unwrap();
    r("lr_put", json!({"name": "msn.mzML", "fid": fid})).unwrap();
    r("lr_index", json!({"name": "msn.mzML"})).unwrap();
    r("idxopen", json!({"slot": 3, "name": "msn.mzML"})).unwrap();
    let ask = |query: &str, more: Value| {
        let mut p = json!({"slot": 3, "name": "msn.mzML", "fid": fid, "query": query});
        p.as_object_mut()
            .unwrap()
            .extend(more.as_object().unwrap().clone());
        text(r("msngrafo", p).unwrap())
    };
    let f = "formula=C14H15N3O5%2B";
    assert_eq!(ask("", json!({})) + "\n", cli(&[]));
    assert_eq!(
        ask(&format!("{f}&ppm_formula=10"), json!({})) + "\n",
        cli(&["--formula", "C14H15N3O5+", "--ppm-formula", "10"])
    );
    let smi = "smiles=CCOC%28%3DO%29c1c%5BnH%5Dc2ccccc12";
    let args = [
        "--formula",
        "C14H15N3O5+",
        "--smiles",
        "CCOC(=O)c1c[nH]c2ccccc12",
    ];
    let with = |extra: &[&str]| cli(&[&args[..], extra].concat());
    assert_eq!(
        ask(&format!("{f}&{smi}&strutture=n3"), json!({})) + "\n",
        with(&["--strutture", "n3"])
    );
    assert_eq!(
        ask(&format!("{f}&{smi}&interpreta=1"), json!({})) + "\n",
        with(&["--interpreta"])
    );
    assert_eq!(
        ask(&format!("{f}&{smi}&rotture=1"), json!({})) + "\n",
        with(&["--rotture"])
    );
    assert_eq!(
        ask("formato=csv", json!({"esporta": true})),
        cli(&["--export", "csv"])
    );
    assert_eq!(
        ask("formato=msp&pulito=1", json!({"esporta": true})),
        cli(&["--export", "msp", "--pulito"])
    );
    // the library entry and the search over it
    let add: Value = serde_json::from_str(&ask(
        &format!("{f}&{smi}"),
        json!({"libreria": "", "molecola": "m1"}),
    ))
    .unwrap();
    assert_eq!(add["doppione"], false);
    let lib = add["libreria"].as_str().unwrap();
    assert!(!lib.is_empty());
    let again: Value = serde_json::from_str(&ask(
        &format!("{f}&{smi}"),
        json!({"libreria": lib, "molecola": "m1"}),
    ))
    .unwrap();
    assert_eq!(again["doppione"], true);
    assert!(ask(f, json!({"cerca": true, "libreria": lib})).contains("\"formato\":\"msncerca/1\""));
    // errors keep their key as the .wasm throws it
    let e = r(
        "msngrafo",
        json!({"slot": 3, "name": "msn.mzML", "fid": fid, "query": "ppm_formula=x"}),
    )
    .err()
    .unwrap();
    assert_eq!(e.key, "err.request.badNumber");
}

/// A small profile mzML (Gaussian peaks on a 0.05 grid with a little noise), written by the writer of the core.
fn profile_mzml(n: usize) -> Vec<u8> {
    use mzlab_core::mzml::Spectrum;
    use mzlab_core::mzmlwrite::{Header, MzmlWriter};
    let h = Header {
        run: "demo".into(),
        sources: vec!["demo.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(n).unwrap();
    for t in 0..n {
        let mz: Vec<f64> = (0..200).map(|i| 297.0 + 0.05 * i as f64).collect();
        let y: Vec<f32> = mz
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let g = |c: f64, a: f64| a * (-((x - c) / 0.3).powi(2) / 2.0).exp();
                (g(300.0, 900.0)
                    + g(304.0, 700.0)
                    + g(306.0, 500.0)
                    + 2.5 * (((i * 7 + t * 3) % 5) as f64) / 4.0) as f32
            })
            .collect();
        let s = Spectrum {
            index: t,
            id: format!("scan={}", t + 1),
            level: 1,
            rt: t as f64 * 0.1,
            polarity: 1,
            profile: true,
            intensity: y,
            mz,
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

#[test]
fn a_profile_mzml_becomes_the_centroid_copy_of_the_core() {
    let dir = tempfile::tempdir().unwrap();
    let st = state(dir.path());
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let src = profile_mzml(40);
    let file = dir.path().join("profile.mzML");
    std::fs::write(&file, &src).unwrap();
    let (fid, _) = st.pick(file).unwrap();
    r("lr_put", json!({"name": "profile.mzML", "fid": fid})).unwrap();
    let out: Value = serde_json::from_str(&text(
        r("mzml2cen", json!({"name": "profile.mzML", "pid": 5})).unwrap(),
    ))
    .unwrap();
    let mut want = Vec::new();
    let did = mzlab_core::centroidi::mzml_to_centroids(
        || Ok(std::io::Cursor::new(&src[..])),
        &mut want,
        "profile.mzML",
        &mut |_, _| {},
    )
    .unwrap();
    assert!(did, "the synthetic file has a profile spectrum");
    let got = st
        .chunk(
            out["id"].as_u64().unwrap() as u32,
            0,
            out["size"].as_u64().unwrap(),
        )
        .unwrap();
    assert_eq!(
        got, want,
        "the copy has the bytes of mzlab_core::centroidi, as the .wasm writes them"
    );
    assert!(got.len() < src.len());
    // the progress the page polls ends with every spectrum done
    let (_, _, _, scans, total) = st.raw_progress(5).unwrap();
    assert_eq!((scans, total), (40, 40));
    // a file that is already centroid is not copied
    let ex = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    let (fid, _) = st.pick(ex).unwrap();
    r("lr_put", json!({"name": "FullScan_t10.mzML", "fid": fid})).unwrap();
    let none = text(r("mzml2cen", json!({"name": "FullScan_t10.mzML", "pid": 6})).unwrap());
    assert_eq!(none, "{\"none\":true}");
}

#[test]
fn the_srm_bytes_and_the_centroid_index_of_a_native_sample() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("src/x.D/AcqData");
    std::fs::create_dir_all(&d).unwrap();
    let (idx, peak) = agilent_synth::encode(&agilent_synth::sample());
    std::fs::write(d.join("MSScan.bin"), idx).unwrap();
    std::fs::write(d.join("MSPeak.bin"), peak).unwrap();
    let st = state(dir.path());
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let mut members = Vec::new();
    let mut paths = Vec::new();
    for n in ["MSScan.bin", "MSPeak.bin"] {
        let (fid, _) = st.pick(d.join(n)).unwrap();
        members.push(json!({"name": format!("x.D/AcqData/{n}"), "fid": fid}));
        paths.push((format!("x.D/AcqData/{n}"), d.join(n)));
    }
    let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
    r("lr_put", json!({"name": "x", "nat": nat})).unwrap();
    r("lr_index", json!({"name": "x"})).unwrap();

    // natsrm: the bytes of srm::impacchetta over the reader (an error stays an error with its key)
    let set = mzlab_core::lettori::FileSet {
        membri: paths
            .iter()
            .map(|(n, p)| {
                let bytes = std::fs::read(p).unwrap();
                mzlab_core::lettori::Membro {
                    nome: n.clone(),
                    dim: bytes.len() as u64,
                    dati: Box::new(bytes),
                    sha1: None,
                }
            })
            .collect(),
    };
    let mut l = mzlab_lettori::apri(set, &mzlab_core::lettori::Limiti::desktop()).unwrap();
    let mut t = mzlab_core::sorgente::SorgenteLettore { l: &mut *l, c: 0 };
    let want = mzlab_core::srm::leggi(&mut t).map(|v| mzlab_core::srm::impacchetta(&v));
    match (r("natsrm", json!({"name": "x"})), want) {
        (Ok(Answer::Bytes(b)), Ok(w)) => assert_eq!(b, w),
        (Err(e), Err(w)) => assert_eq!(e.key, w.key),
        (a, w) => panic!(
            "natsrm and the core disagree: {:?} / {:?}",
            a.is_ok(),
            w.is_ok()
        ),
    }
    assert!(r("natsrm", json!({"name": "nope"})).is_err());

    // lr_index_cen: <name>.cen.mzidx apart from the index of the Data tab
    assert_eq!(text(r("lr_index_cen", json!({"name": "x"})).unwrap()), "{}");
    assert!(dir.path().join("idx/x.cen.mzidx").exists());
    assert!(dir.path().join("idx/x.mzidx").exists());
    assert!(
        text(r("idxopen", json!({"slot": 4, "name": "x.cen"})).unwrap()).contains("\"scans\":3")
    );
    assert!(r("lr_index_cen", json!({"name": "nope"})).is_err());
}

#[test]
fn the_graph_of_a_native_sample_is_that_of_its_mzml() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path().join("src/x.D/AcqData");
    std::fs::create_dir_all(&d).unwrap();
    let (idx, peak) = agilent_synth::encode(&agilent_synth::sample());
    std::fs::write(d.join("MSScan.bin"), idx).unwrap();
    std::fs::write(d.join("MSPeak.bin"), peak).unwrap();
    let st = state(dir.path());
    let r = |op: &str, p: Value| st.rpc(op, &p, &[]);
    let mut members = Vec::new();
    for n in ["MSScan.bin", "MSPeak.bin"] {
        let (fid, _) = st.pick(d.join(n)).unwrap();
        members.push(json!({"name": format!("x.D/AcqData/{n}"), "fid": fid}));
    }
    let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
    r("lr_put", json!({"name": "x", "nat": nat})).unwrap();
    r("lr_index", json!({"name": "x"})).unwrap();
    r("idxopen", json!({"slot": 3, "name": "x"})).unwrap();

    // the same sample as a plain mzML, as «Download .mzML» writes it
    let made: Value = serde_json::from_str(&text(
        r("nat2mzml", json!({"name": "x", "pid": 1})).unwrap(),
    ))
    .unwrap();
    let id = made["id"].as_u64().unwrap();
    r("lr_put", json!({"name": "x.mzML", "fid": id})).unwrap();
    r("lr_index", json!({"name": "x.mzML"})).unwrap();
    r("idxopen", json!({"slot": 4, "name": "x.mzML"})).unwrap();

    let ask = |slot: u32, name: &str, more: Value| {
        let mut p = json!({"slot": slot, "name": name, "fid": id, "query": ""});
        p.as_object_mut()
            .unwrap()
            .extend(more.as_object().unwrap().clone());
        match r("msngrafo", p) {
            Ok(Answer::Text(t)) => Ok(t),
            Ok(Answer::Bytes(_)) => panic!("bytes"),
            Err(e) => Err(e.key.to_string()),
        }
    };
    let want = ask(4, "x.mzML", json!({}));
    let got = ask(3, "x", json!({}));
    // the graph names the file it was made from and the hash of its index (which holds the name of the index): all else is the same
    let norm = |s: Result<String, String>| {
        s.map(|t| {
            let mut j: Value = serde_json::from_str(&t).unwrap();
            j["sorgente"] = json!(null);
            j
        })
    };
    assert_ne!(
        got,
        Err("err.route.unavailable".to_string()),
        "a native sample is served"
    );
    assert_eq!(norm(got.clone()), norm(want));
    // the second request reads what the first one wrote
    assert_eq!(ask(3, "x", json!({})), got);
    assert_eq!(
        ask(3, "x", json!({"esporta": true})).is_ok(),
        ask(4, "x.mzML", json!({"esporta": true})).is_ok()
    );
}
