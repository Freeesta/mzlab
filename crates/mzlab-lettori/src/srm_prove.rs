// The tests of srm.rs (a reader that is not one: `Finto`).
use super::*;
use mzlab_core::error::Error;

struct Finto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    metodo: String,
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
        let (mz, y): (Vec<f64>, Vec<f32>) = if k.is_multiple_of(2) {
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
        Ok(self.metodo.clone())
    }
}

fn finto(filtro: &str) -> SrmDaScansioni {
    finto_con(filtro, 1, "")
}

/// Four scans with `filtro` and polarity `pol`; `metodo` is the `metodo/1` document the reader gives.
fn finto_con(filtro: &str, pol: i8, metodo: &str) -> SrmDaScansioni {
    let indice = (0..4)
        .map(|k| {
            let mut v = voce(
                k,
                k as f64 * 0.1,
                filtro,
                if k.is_multiple_of(2) { 300.0 } else { 400.0 },
            );
            v.polarita = pol;
            v
        })
        .collect();
    SrmDaScansioni::new(Box::new(Finto {
        campioni: vec![Campione {
            id: 1,
            nome: "a".into(),
            scansioni: 4,
        }],
        indice,
        metodo: metodo.into(),
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
            "SRM SIC Q1=300 Q3=100 ce=25",
            "SRM SIC Q1=300 Q3=150.001 ce=25",
            "SRM SIC Q1=400 Q3=200 ce=25"
        ]
    );
    assert!(v.iter().all(|c| c.tipo == ChromKind::Srm));
    let t = l.cromatogramma(0, "SRM SIC Q1=300 Q3=100 ce=25").unwrap();
    assert_eq!(t.time, [0.0, 0.2]);
    assert_eq!(t.intensity, [10.0, 12.0]);
    assert_eq!((t.q1, t.q3), (Some(300.0), Some(100.0)));
    let (q1, q3, ce, _) = mzlab_core::mzmlhead::srm_id(&t.id);
    assert_eq!((q1, q3, ce), (Some(300.0), Some(100.0), Some(25.0)));
}

/// A TSQ filter as the vendor writes it: no collision energy in it, the product window in brackets.
const FILTRO_TSQ: &str = "+ c ESI SRM ms2 300.000 [100.000-100.000, 150.000-150.002]";

const METODO_TSQ: &str = r#"{"ms":{"periodi":[{"esperimenti":[{"transizioni":[
{"q1":300,"q3":100,"ce_eV":18.5,"polarita":"negative"},
{"q1":300,"q3":150.001,"ce_eV":30,"polarita":"negative"},
{"q1":400,"q3":200,"ce_eV":22,"polarita":"negative"}]}]}]}}"#;

#[test]
fn filtro_tsq_senza_ce_prende_ce_e_polarita_dal_metodo() {
    let mut l = finto_con(FILTRO_TSQ, 1, METODO_TSQ);
    let ids: Vec<String> = l
        .cromatogrammi(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(
        ids,
        [
            "SRM SIC Q1=300 Q3=100 ce=18.5 polarity=negative",
            "SRM SIC Q1=300 Q3=150.001 ce=30 polarity=negative",
            "SRM SIC Q1=400 Q3=200 ce=22 polarity=negative"
        ]
    );
}

#[test]
fn filtro_tsq_senza_metodo_non_inventa_il_ce() {
    let mut l = finto_con(FILTRO_TSQ, 1, "");
    let ids: Vec<String> = l
        .cromatogrammi(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids[0], "SRM SIC Q1=300 Q3=100");
    let mut l = finto_con(FILTRO_TSQ, -1, "not json");
    assert_eq!(
        l.cromatogrammi(0).unwrap()[0].id,
        "SRM SIC Q1=300 Q3=100 polarity=negative"
    );
}

#[test]
fn transizione_duplicata_nel_metodo_resta_come_le_scansioni() {
    let m = r#"{"ms":{"periodi":[{"esperimenti":[{"transizioni":[
{"q1":300,"q3":100,"ce_eV":10,"polarita":"positive"},
{"q1":300,"q3":100,"ce_eV":40,"polarita":"negative"}]}]}]}}"#;
    let mut l = finto_con(FILTRO_TSQ, 1, m);
    assert_eq!(l.cromatogrammi(0).unwrap()[0].id, "SRM SIC Q1=300 Q3=100");
}

#[test]
fn crm_conta_come_srm() {
    let mut l = finto("+ c ESI CRM ms2 300.0@cid25.0 [100.0-100.0]");
    assert_eq!(l.cromatogrammi(0).unwrap().len(), 3);
}

#[test]
fn senza_filtro_srm_niente_tracce() {
    let mut l = finto("+ c Full ms2 300.0@cid25.0 [50.0-400.0]");
    assert!(l.cromatogrammi(0).unwrap().is_empty());
    assert!(!ha_scansioni_srm(&mut l, 0));
}
