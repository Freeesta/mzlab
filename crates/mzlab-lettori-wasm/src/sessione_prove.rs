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
    let i: serde_json::Value = serde_json::from_str(&s.indice(h, 0).unwrap()).unwrap();
    assert_eq!(i[0]["id"], "scan=0");
    let sc = s.scansione(h, 0, 0).unwrap();
    assert_eq!((sc.mz, sc.intensita), (vec![100.0, 200.0], vec![1.5, 2.5]));
    assert!(sc.meta.contains("\"level\":1"));
    assert!(s.intestazione(h, 0, 0).unwrap().contains("MS:1"));
    assert!(s.cromatogrammi(h, 0).unwrap().contains("Tic"));
    assert!(s
        .cromatogramma(h, 0, "TIC")
        .unwrap()
        .contains("\"intensity\":[3.0,4.0]"));
    assert_eq!(s.metodo(h, 0).unwrap(), "{}");
    s.chiudi(h);
    assert_eq!(s.campioni(h).err().map(|e| e.key), Some("error.badIndex"));
}
