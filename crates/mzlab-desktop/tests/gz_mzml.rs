//! A `.mzML.gz` in the app and on the site. The app reads it through the readers of the registry (`aperti/mzml_gz.rs`: inflated into memory); the site inflates it piece by
//! piece into OPFS with `mzlab_core::gunzip` (the `Gunzip` of the engine `.wasm`, fed by `rust-carica.js` `putGz`) and opens the mzML it wrote as a plain upload. The mzML of the
//! site must be the original byte for byte, and its TIC, BPC, XIC, spectra and scans the app's answers for the `.gz`, exactly. Synthetic run, two gzip members.
use flate2::write::GzEncoder;
use flate2::Compression;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

const SCANS: usize = 60;

fn run() -> Vec<u8> {
    let h = Header {
        run: "gz".into(),
        sources: vec!["gz.mzML".into()],
        software: ("mzLab".into(), "0".into()),
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(SCANS).unwrap();
    for t in 0..SCANS {
        let n = 200 + (t * 37) % 90;
        let s = Spectrum {
            index: t,
            id: format!("scan={}", t + 1),
            level: if t % 4 == 3 { 2 } else { 1 },
            rt: t as f64 * 0.05,
            polarity: 1,
            precursor: if t % 4 == 3 { Some(301.2) } else { None },
            mz: (0..n)
                .map(|i| 100.0 + i as f64 * 1.37 + t as f64 * 0.0013)
                .collect(),
            intensity: (0..n)
                .map(|i| 50.0 + ((i * 7919 + t * 31) % 997) as f32 * 13.0)
                .collect(),
            ..Default::default()
        };
        w.spectrum(&s).unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// Two gzip members one after the other (as `bgzip` or `cat a.gz b.gz` make): the stream must not stop at the end of the first.
fn two_members(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for half in [&b[..b.len() / 3], &b[b.len() / 3..]] {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(half).unwrap();
        out.extend(e.finish().unwrap());
    }
    out
}

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// The answers of a file: a plain mzML as a picked file (what the site's upload of the inflated mzML is), or, with `native`, a dataset of the registry of readers.
fn answers(dir: &Path, file: &Path, name: &str, native: bool) -> Vec<Value> {
    let st = State::new();
    st.set_data_folder(dir.join(format!("app-{name}")));
    st.set_index_folder(dir.join(format!("idx-{name}")))
        .unwrap();
    let (fid, _) = st.pick(file.to_path_buf()).unwrap();
    let r = |op: &str, p: Value| {
        st.rpc(op, &p, &[])
            .map(text)
            .unwrap_or_else(|e| panic!("{op} {name}: {}", e.key))
    };
    // the item takes the name of the mzML in both: the site names the sample without `.gz`
    let item = "run.mzML";
    if native {
        let members = json!([{"name": name, "fid": fid}]);
        let nat = json!({"key": "k", "c": 0, "id": 0, "centroid": false, "members": members});
        r("lr_put", json!({"name": item, "nat": nat}));
    } else {
        r("lr_put", json!({"name": item, "fid": fid}));
    }
    r("lr_index", json!({"name": item}));
    r("idxopen", json!({"slot": 0, "name": item}));
    // an answer that is not JSON (a table as text) is compared as its text
    let get = |path: &str, q: &str| {
        let t = r("route", json!({"slot": 0, "path": path, "query": q}));
        serde_json::from_str::<Value>(&t).unwrap_or(Value::String(t))
    };
    vec![
        get("/api/chrom", "?k=0&kind=tic&level=1"),
        get("/api/chrom", "?k=0&kind=bpc&level=1"),
        get("/api/chrom", "?k=0&kind=tic&level=2"),
        get("/api/xic", "?k=0&mz=150&tol=0.5&level=1"),
        get("/api/spectrum", "?k=0&level=1&rt0=0.2&rt1=0.2"),
        get("/api/spectrum", "?k=0&level=1&rt0=0.5&rt1=1.5"),
        get("/api/scanlist", "?k=0&level=2"),
        get("/api/scan_stats", "?k=0"),
    ]
}

#[test]
fn the_site_mzml_of_a_gz_is_the_original_and_answers_as_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let mzml = run();
    let gz = two_members(&mzml);
    // the site: pieces of the File (odd sizes, as the slices of a Blob) through the streaming decoder
    let mut g = mzlab_core::gunzip::Gunzip::new(4 << 30);
    let mut site = Vec::new();
    for piece in gz.chunks(65_537) {
        site.extend(g.push(piece).unwrap());
    }
    let (rest, whole) = g.finish().unwrap();
    site.extend(rest);
    assert!(whole);
    assert_eq!(
        site, mzml,
        "the inflated mzML is the original byte for byte"
    );
    let (gz_file, site_file) = (dir.path().join("run.mzML.gz"), dir.path().join("site.mzML"));
    std::fs::write(&gz_file, &gz).unwrap();
    std::fs::write(&site_file, &site).unwrap();
    let app = answers(dir.path(), &gz_file, "run.mzML.gz", true);
    let web = answers(dir.path(), &site_file, "site.mzML", false);
    assert_eq!(app.len(), web.len());
    for (k, (a, w)) in app.iter().zip(&web).enumerate() {
        assert_eq!(a, w, "answer {k}");
    }
    // not vacuous: the TIC has every MS1 scan and the spectrum has peaks
    assert_eq!(app[0]["y"].as_array().unwrap().len(), SCANS - SCANS / 4);
    assert!(!app[4]["mz"].as_array().unwrap().is_empty());
}
