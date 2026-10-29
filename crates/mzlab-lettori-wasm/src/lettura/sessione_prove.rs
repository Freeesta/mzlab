// The tests of sessione.rs.
use super::*;
use crate::sessione::duro::{Duro, STRANI};
use mzlab_core::lettori::{Campione, Campo, Formato, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};
use serde_json::Value;

struct Finto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
}

impl Lettore for Finto {
    fn formato(&self) -> Formato {
        Formato::Altro("finto")
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, _: usize) -> Result<&[VoceScansione]> {
        Ok(&self.indice)
    }
    fn scansione(&mut self, _: usize, k: usize) -> Result<Spectrum> {
        Ok(Spectrum {
            index: k,
            id: format!("scan={k}"),
            level: 1,
            rt: 0.5,
            polarity: 1,
            profile: false,
            filter: "f".into(),
            precursor: None,
            mz: vec![100.0, 200.0],
            intensity: vec![1.5, 2.5],
            wide: None,
        })
    }
    fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
        Ok(vec![Campo {
            nome: "a".into(),
            valore: "b".into(),
            unita: None,
            cv: Some("MS:1".into()),
        }])
    }
    fn cromatogrammi(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(vec![VoceCromatogramma {
            id: "TIC".into(),
            tipo: ChromKind::Tic,
            unita: "u".into(),
        }])
    }
    fn cromatogramma(&mut self, _: usize, id: &str) -> Result<Chromatogram> {
        Ok(Chromatogram {
            index: 0,
            id: id.into(),
            kind: ChromKind::Tic,
            q1: None,
            q3: None,
            unit: "u".into(),
            time: vec![0.0, 1.0],
            intensity: vec![3.0, 4.0],
        })
    }
    fn metodo(&mut self, _: usize) -> Result<String> {
        Ok("{}".into())
    }
}

#[test]
fn la_sessione_risponde_e_chiude() {
    let mut s = Sessioni::default();
    let fin = Finto {
        campioni: vec![Campione {
            id: 1,
            nome: "a".into(),
            scansioni: 1,
            dimensione: None,
        }],
        indice: vec![VoceScansione {
            id: "scan=0".into(),
            rt: 0.5,
            livello: 1,
            polarita: 1,
            tic: 4.0,
            picco_base: Some(200.0),
            filtro: "f".into(),
            precursore: None,
            posizione: None,
        }],
    };
    let h = s.inserisci(Box::new(fin));
    let c: serde_json::Value = serde_json::from_str(&s.campioni(h).unwrap()).unwrap();
    assert_eq!(c["formato"], "finto");
    assert_eq!(c["campioni"][0]["scans"], 1);
    let i = mzlab_core::trasporto::decode_indice(&s.indice(h, 0).unwrap()).unwrap();
    assert_eq!((i[0].id.as_str(), i[0].filtro.as_str()), ("scan=0", "f"));
    let sc = mzlab_core::trasporto::decode_scansione(0, &s.scansione(h, 0, 0).unwrap()).unwrap();
    assert_eq!((sc.mz, sc.intensity), (vec![100.0, 200.0], vec![1.5, 2.5]));
    assert_eq!((sc.level, sc.polarity), (1, 1));
    assert!(s.intestazione(h, 0, 0).unwrap().contains("MS:1"));
    assert!(s.cromatogrammi(h, 0).unwrap().contains("Tic"));
    let x: serde_json::Value =
        serde_json::from_str(&s.cromatogramma(h, 0, "TIC").unwrap()).unwrap();
    assert_eq!(
        x["intensity"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>(),
        [3.0, 4.0]
    );
    assert_eq!(s.metodo(h, 0).unwrap(), "{}");
    assert_eq!(s.testata(h, 0).unwrap(), "null");
    s.chiudi(h);
    assert_eq!(s.campioni(h).err().map(|e| e.key), Some("error.badIndex"));
}

/// The index crosses as bytes and comes back as it was, with every column (NaN, infinities, 64-bit offsets, the repeated filter once).
#[test]
fn l_indice_in_byte_torna_uguale() {
    use mzlab_core::lettori::Posizione;
    let voci = vec![
        VoceScansione {
            id: "scan=1 \"q\"\n\u{8}é".into(),
            rt: 1.0,
            livello: 2,
            polarita: -1,
            tic: 1e21,
            picco_base: None,
            filtro: "FTMS + c ESI Full ms2 445.12@hcd30.00 [50.00-500.00]".into(),
            precursore: Some(445.12),
            posizione: Some(Posizione {
                membro: 3,
                offset: 1 << 40,
                lunghezza: 77,
            }),
        },
        VoceScansione {
            id: String::new(),
            rt: 0.1 + 0.2,
            livello: 1,
            polarita: 0,
            tic: 0.1 + 0.2,
            picco_base: Some(f64::INFINITY),
            filtro: String::new(),
            precursore: None,
            posizione: None,
        },
    ];
    let mut s = Sessioni::default();
    let h = s.inserisci(Box::new(Finto {
        campioni: vec![],
        indice: voci.clone(),
    }));
    let b = s.indice(h, 0).unwrap();
    assert_eq!(mzlab_core::trasporto::decode_indice(&b).unwrap(), voci);
}

// `3` and `3.0` are the same number for the page (`JSON.parse`): compare every number as a double
/// Every number as a double, a non-finite one as null.
fn norma(v: Value) -> Value {
    match v {
        Value::Number(n) => n
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map_or(Value::Null, Value::Number),
        Value::Array(a) => Value::Array(a.into_iter().map(norma).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, v)| (k, norma(v))).collect()),
        v => v,
    }
}

/// A reader whose answers hold the awkward values: control characters, quotes, non-ASCII, NaN, infinities, `-0.0`, numbers at both ends of the range.
/// The points of a trace text are the doubles the reader gave (the sign of zero included), NaN and infinities being null: read here with `f64::from_str`, because serde_json's own
/// reader is not exact to the last bit (no `float_roundtrip`) while `JSON.parse` is.
fn punti_identici(testo: &str, x: &Chromatogram) {
    let (t_txt, i_txt) = testo
        .split_once("\"time\":[")
        .unwrap()
        .1
        .split_once("],\"intensity\":[")
        .unwrap();
    for (txt, orig) in [
        (t_txt, &x.time),
        (i_txt.strip_suffix("]}").unwrap(), &x.intensity),
    ] {
        let letti: Vec<&str> = txt.split(',').collect();
        assert_eq!(letti.len(), orig.len());
        for (l, o) in letti.iter().zip(orig.iter()) {
            if o.is_finite() {
                assert_eq!(l.parse::<f64>().unwrap().to_bits(), o.to_bits(), "{o:e}");
            } else {
                assert_eq!(*l, "null", "{o:e}");
            }
        }
    }
}

/// The texts of the session are what `serde_json::json!` wrote before (kept here as the reference): same parsed values for every function, and the numbers read back bit for bit.
#[test]
fn i_testi_json_valgono_quelli_di_serde_json() {
    use serde_json::json;
    let parse = |t: String| -> Value { norma(serde_json::from_str(&t).expect("valid JSON")) };
    let json = |v: Value| norma(v);
    let mut s = Sessioni::default();
    let h = s.inserisci(Box::new(Duro));
    let mut r = Duro;

    let vecchio: Vec<_> = r
        .campioni()
        .iter()
        .map(|c| json!({ "id": c.id, "name": c.nome, "scans": c.scansioni }))
        .collect();
    assert_eq!(
        parse(s.campioni(h).unwrap()),
        json(json!({ "formato": "duro", "campioni": vecchio }))
    );

    let vecchio: Vec<_> = r
        .intestazione(0, 0)
        .unwrap()
        .iter()
        .map(|f| json!({ "name": f.nome, "value": f.valore, "unit": f.unita, "cv": f.cv }))
        .collect();
    assert_eq!(
        parse(s.intestazione(h, 0, 0).unwrap()),
        json(Value::Array(vecchio))
    );

    let vecchio: Vec<_> = r
        .cromatogrammi(0)
        .unwrap()
        .iter()
        .map(|x| json!({ "id": x.id, "kind": format!("{:?}", x.tipo), "unit": x.unita }))
        .collect();
    assert_eq!(
        parse(s.cromatogrammi(h, 0).unwrap()),
        json(Value::Array(vecchio))
    );

    let x = r.cromatogramma(0, STRANI).unwrap();
    let mut vecchio = json!({ "id": x.id, "kind": format!("{:?}", x.kind), "q1": x.q1, "q3": x.q3, "unit": x.unit, "time": x.time, "intensity": x.intensity });
    let testo = s.cromatogramma(h, 0, STRANI).unwrap();
    let mut nuovo = parse(testo.clone());
    punti_identici(&testo, &x);
    for v in [&mut vecchio, &mut nuovo] {
        let m = v.as_object_mut().unwrap();
        m.remove("time");
        m.remove("intensity");
    }
    assert_eq!(nuovo, json(vecchio));

    let t = r.testata_mzml(0).unwrap().unwrap();
    let vecchio = json!({
        "run": t.run, "sources": t.sources, "instrument": t.instrument, "components": t.components,
        "software": [t.software.0, t.software.1], "acquired": t.acquired, "serial": t.serial,
        "source_sha1": t.source_sha1, "source_cv": t.source_cv, "sample": t.sample,
        "run_params": t.run_params, "content": t.content, "extra_software": t.extra_software,
        "centroided": t.centroided,
    });
    assert_eq!(parse(s.testata(h, 0).unwrap()), json(vecchio));
    assert_eq!(s.testata(h, 1).unwrap(), "null");
}

/// Speed of `cromatogramma` on a trace of 1M points (run with `cargo test -p mzlab-lettori-wasm --release -- --ignored --nocapture`).
#[test]
#[ignore]
fn misura_cromatogramma_un_milione_di_punti() {
    struct Lungo(Vec<f64>);
    impl Lettore for Lungo {
        fn formato(&self) -> Formato {
            Formato::Altro("lungo")
        }
        fn campioni(&self) -> &[Campione] {
            &[]
        }
        fn indice(&mut self, _: usize) -> Result<&[VoceScansione]> {
            Ok(&[])
        }
        fn scansione(&mut self, _: usize, _: usize) -> Result<Spectrum> {
            Err(mzlab_core::Error::new("error.badIndex"))
        }
        fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
            Ok(vec![])
        }
        fn cromatogrammi(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
            Ok(vec![])
        }
        fn cromatogramma(&mut self, _: usize, id: &str) -> Result<Chromatogram> {
            Ok(Chromatogram {
                index: 0,
                id: id.into(),
                kind: ChromKind::Tic,
                q1: None,
                q3: None,
                unit: "u".into(),
                time: self.0.clone(),
                intensity: self.0.clone(),
            })
        }
        fn metodo(&mut self, _: usize) -> Result<String> {
            Ok("{}".into())
        }
    }
    let punti: Vec<f64> = (0..1_000_000)
        .map(|i| i as f64 * 0.0013 + (i % 97) as f64 / 7.0)
        .collect();
    let mut s = Sessioni::default();
    let h = s.inserisci(Box::new(Lungo(punti)));
    let t = std::time::Instant::now();
    let n = s.cromatogramma(h, 0, "TIC").unwrap().len();
    println!(
        "cromatogramma 1M points: {} ms, {} bytes",
        t.elapsed().as_millis(),
        n
    );
}
