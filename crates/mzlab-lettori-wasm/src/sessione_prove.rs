// The tests of sessione.rs.
use super::*;
use mzlab_core::lettori::{Campione, Campo, Formato, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{ChromKind, Chromatogram, Spectrum};

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
    assert!(s
        .cromatogramma(h, 0, "TIC")
        .unwrap()
        .contains("\"intensity\":[3.0,4.0]"));
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
