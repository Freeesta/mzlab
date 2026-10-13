//! `metodo/1` of the `.wiff` files of the 3200 QTRAP against what is known of them: the `.dam` that the file names, the SHA-1 that ProteoWizard wrote in its mzML.
//! Needs the private data repository (`MZLAB_DATI`, or `mzlab-dati` beside this one); without it the test is skipped (no data of the laboratory lives here).
use mzlab_core::json::{self, Value};
use mzlab_core::methodinfo::read_methods;
use mzlab_core::metodo::{self, Source, SrcFile};
use mzlab_core::metodo_wiff::from_wiff;
use mzlab_core::ole::Ole;
use std::path::PathBuf;

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    if let Ok(cwd) = std::env::current_dir() {
        for up in cwd.ancestors() {
            cands.push(up.join("mzlab-dati"));
        }
    }
    cands.into_iter().find(|p| p.join("3200 qtrap").is_dir())
}

fn src(name: &str, bytes: &[u8], sample: u32) -> Source {
    let mut d = bytes.to_vec();
    Source {
        format: "sciex-wiff",
        files: vec![SrcFile {
            name: name.into(),
            sha1: Some(metodo::sha1_hex(&mut d, bytes.len() as u64).unwrap()),
            bytes: bytes.len() as u64,
        }],
        sample: Some(sample),
    }
}

fn ms_of(doc: &str) -> String {
    let v = json::parse(doc.as_bytes()).unwrap();
    let ms = v.get("ms").expect("ms section");
    let mut o = json::Out::default();
    // compare through the text of the section: the same values in the same order
    fn w(v: &Value, o: &mut json::Out) {
        match v {
            Value::Obj(f) => {
                o.raw("{");
                // the kind of source (`tipo` of the source, which has only it and `parametri`) is said by the log of a .wiff, not by a .dam
                let is_source = f.len() == 2 && f.iter().any(|(k, _)| k == "parametri");
                let f: Vec<_> = f
                    .iter()
                    .filter(|(k, _)| !(is_source && k == "tipo"))
                    .collect();
                for (i, (k, x)) in f.into_iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
                    w(x, o);
                }
                o.raw("}");
            }
            Value::Arr(a) => {
                o.raw("[");
                for (i, x) in a.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" });
                    w(x, o);
                }
                o.raw("]");
            }
            Value::Num(x) => drop(o.num(*x)),
            Value::Str(s) => drop(o.str(s)),
            _ => drop(o.raw("null")),
        }
    }
    w(ms, &mut o);
    o.0
}

/// One sample of a `.wiff`: the sections of the document, the method equal to the `.dam` it names. Gives 1 when a `.dam` of the repository was compared.
fn check_sample(d: &std::path::Path, ole: &Ole, wiff: &str, bytes: &[u8], sample: u32) -> usize {
    let doc = from_wiff(ole, sample, &src(wiff, bytes, sample));
    let v =
        json::parse(doc.as_bytes()).unwrap_or_else(|e| panic!("{wiff} {sample}: not JSON: {e:?}"));
    assert_eq!(
        v.get("schema").and_then(|s| s.str()),
        Some("metodo/1"),
        "{wiff} {sample}"
    );
    for k in ["origine", "campione", "strumento", "registro", "grezzo"] {
        assert!(
            v.get(k).is_some(),
            "{wiff} {sample}: section {k} is missing"
        );
    }
    let text = |sec: &str, key: &str| {
        v.get(sec)
            .and_then(|s| s.get(key))
            .and_then(|s| s.str())
            .map(String::from)
    };
    assert_eq!(
        text("strumento", "modello").as_deref(),
        Some("3200 Q TRAP"),
        "{wiff} {sample}"
    );
    let mut checked = 0;
    // the .dam named by the sample, when the repository has it: every parameter of the method the same
    if let (Some(name), true) = (text("campione", "metodo_file"), v.get("ms").is_some()) {
        if let Ok(dam) = std::fs::read(d.join("dam").join(&name)) {
            let m = read_methods(dam, &name).unwrap().remove(0);
            let source = Source {
                format: "sciex-dam",
                files: vec![SrcFile {
                    name: name.clone(),
                    ..Default::default()
                }],
                ..Default::default()
            };
            assert_eq!(
                ms_of(&doc),
                ms_of(&metodo::from_dam(&m, &source)),
                "{wiff} sample {sample}: ms differs from {name}"
            );
            checked = 1;
        }
    }
    checked
}

#[test]
fn the_wiff_method_is_the_dam_it_names_and_the_sha1_is_the_one_of_the_mzml() {
    let Some(d) = dati() else {
        eprintln!("SKIP metodo_parity: no data repository (MZLAB_DATI)");
        return;
    };
    let dir = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    let mut checked = 0;
    for wiff in ["MRM.wiff", "FullMass.wiff", "MS2.wiff"] {
        let bytes = std::fs::read(dir.join(wiff)).unwrap();
        let ole = Ole::open(bytes.clone(), wiff).unwrap();
        for sample in
            (1..=16u32).filter(|n| ole.has(&format!("SampleSubtree/Sample{n}/SampleDABE/DATA")))
        {
            checked += check_sample(&d, &ole, wiff, &bytes, sample);
        }
    }
    assert!(checked >= 1, "no .dam of the repository matched a .wiff");
    // the mzML of ProteoWizard says the SHA-1 of the .wiff it was made from (its name differs: B_MRM.wiff)
    let mzml = d.join("mzML/B_MRM-STD_0_6ppm.mzML");
    if let Ok(t) = std::fs::read(&mzml) {
        let head = String::from_utf8_lossy(&t[..t.len().min(8000)]).to_string();
        let mut bytes = std::fs::read(dir.join("MRM.wiff")).unwrap();
        let sha = metodo::sha1_hex(
            &mut bytes,
            std::fs::metadata(dir.join("MRM.wiff")).unwrap().len(),
        )
        .unwrap();
        assert!(
            metodo::source_sha1s(&head).iter().any(|(_, h)| *h == sha),
            "the SHA-1 of MRM.wiff is not the one of its mzML"
        );
    }
}
