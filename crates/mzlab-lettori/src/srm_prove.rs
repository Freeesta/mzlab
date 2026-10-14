// The tests of srm.rs (a reader that is not one: `Finto`).
use super::*;
use mzlab_core::error::Error;

struct Finto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
}

fn voce(k: usize, rt: f64, filtro: &str, q1: f64) -> VoceScansione {
    VoceScansione {
        id: format!("scan={k}"),
        rt,
        livello: 2,
        polarita: 1,
        tic: 0.0,
        picco_base: None,
        filtro: filtro.into(),
        precursore: Some(q1),
        posizione: None,
    }
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
        // Even scans: precursor 300 with products 100 and 150; odd ones: precursor 400 with product 200.
        let (mz, y): (Vec<f64>, Vec<f32>) = if k % 2 == 0 {
            (vec![100.0, 150.001], vec![10.0 + k as f32, 5.0])
        } else {
            (vec![200.0], vec![7.0])
        };
        let r = &self.indice[k];
        Ok(Spectrum {
            index: k,
            id: r.id.clone(),
            level: 2,
            rt: r.rt,
            polarity: 1,
            profile: false,
            filter: r.filtro.clone(),
            precursor: r.precursore,
            mz,
            intensity: y,
            wide: None,
        })
    }
    fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
        Ok(vec![])
    }
    fn cromatogrammi(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(vec![])
    }
    fn cromatogramma(&mut self, _: usize, _: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn metodo(&mut self, _: usize) -> Result<String> {
        Ok(String::new())
    }
}

fn finto(filtro: &str) -> SrmDaScansioni {
    let indice = (0..4)
        .map(|k| {
            voce(
                k,
                k as f64 * 0.1,
                filtro,
                if k % 2 == 0 { 300.0 } else { 400.0 },
            )
        })
        .collect();
    SrmDaScansioni::new(Box::new(Finto {
        campioni: vec![Campione {
            id: 1,
            nome: "a".into(),
            scansioni: 4,
        }],
        indice,
    }))
}

#[test]
fn una_traccia_per_transizione() {
    let mut l = finto("+ c SRM ms2 300.0@cid25.0 [100.0-100.0]");
    let v = l.cromatogrammi(0).unwrap();
    let ids: Vec<&str> = v.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "SRM SIC Q1=300 Q3=100",
            "SRM SIC Q1=300 Q3=150.001",
            "SRM SIC Q1=400 Q3=200"
        ]
    );
    assert!(v.iter().all(|c| c.tipo == ChromKind::Srm));
    let t = l.cromatogramma(0, "SRM SIC Q1=300 Q3=100").unwrap();
    assert_eq!(t.time, [0.0, 0.2]);
    assert_eq!(t.intensity, [10.0, 12.0]);
    assert_eq!((t.q1, t.q3), (Some(300.0), Some(100.0)));
    let (q1, q3, _, _) = mzlab_core::mzmlhead::srm_id(&t.id);
    assert_eq!((q1, q3), (Some(300.0), Some(100.0)));
}

#[test]
fn senza_filtro_srm_niente_tracce() {
    let mut l = finto("+ c Full ms2 300.0@cid25.0 [50.0-400.0]");
    assert!(l.cromatogrammi(0).unwrap().is_empty());
    assert!(!ha_scansioni_srm(&mut l, 0));
}
