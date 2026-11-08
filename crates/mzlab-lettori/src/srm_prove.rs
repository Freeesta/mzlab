// The tests of srm.rs (a reader that is not one: `Finto`).
use super::*;
use mzlab_core::error::Error;

struct Finto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
    metodo: String,
}

fn entry(k: usize, rt: f64, filter: &str, q1: f64) -> VoceScansione {
    VoceScansione {
        id: format!("scan={k}"),
        rt,
        livello: 2,
        polarita: 1,
        tic: 0.0,
        picco_base: None,
        filtro: filter.into(),
        precursore: Some(q1),
        posizione: None,
    }
}

impl Lettore for Finto {
    fn not_carried(&self) -> &'static [(&'static str, &'static str)] {
        &[("campo finto", "motivo finto")]
    }
    fn format(&self) -> Formato {
        Formato::Altro("finto")
    }
    fn samples(&self) -> &[Campione] {
        &self.campioni
    }
    fn index(&mut self, _: usize) -> Result<&[VoceScansione]> {
        Ok(&self.indice)
    }
    fn scan(&mut self, _: usize, k: usize) -> Result<Spectrum> {
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
    fn scan_header(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
        Ok(vec![])
    }
    fn chromatograms(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(vec![])
    }
    fn chromatogram(&mut self, _: usize, _: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn method(&mut self, _: usize) -> Result<String> {
        Ok(self.metodo.clone())
    }
    fn scan_profile(&mut self, c: usize, k: usize) -> Result<Option<Spectrum>> {
        let mut s = self.scan(c, k)?;
        s.profile = true;
        Ok(Some(s))
    }
    fn table_header(&mut self, _: usize) -> Result<Option<Header>> {
        Ok(Some(Header {
            run: "leggera".into(),
            ..Header::default()
        }))
    }
}

fn finto(filter: &str) -> SrmDaScansioni {
    fake_with(filter, 1, "")
}

/// Four scans with `filtro` and polarity `pol`; `metodo` is the `metodo/1` document the reader gives.
fn fake_with(filter: &str, pol: i8, method: &str) -> SrmDaScansioni {
    let index = (0..4)
        .map(|k| {
            let mut v = entry(
                k,
                k as f64 * 0.1,
                filter,
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
            dimensione: None,
        }],
        indice: index,
        metodo: method.into(),
    }))
}

#[test]
fn one_trace_per_transition() {
    let mut l = finto("+ c SRM ms2 300.0@cid25.0 [100.0-100.0]");
    let v = l.chromatograms(0).unwrap();
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
    let t = l.chromatogram(0, "SRM SIC Q1=300 Q3=100 ce=25").unwrap();
    assert_eq!(t.time, [0.0, 0.2]);
    assert_eq!(t.intensity, [10.0, 12.0]);
    assert_eq!((t.q1, t.q3), (Some(300.0), Some(100.0)));
    let (q1, q3, ce, _) = mzlab_core::mzmlhead::srm_id(&t.id);
    assert_eq!((q1, q3, ce), (Some(300.0), Some(100.0), Some(25.0)));
}

/// A TSQ filter as the vendor writes it: no collision energy in it, the product window in brackets.
const TSQ_FILTER: &str = "+ c ESI SRM ms2 300.000 [100.000-100.000, 150.000-150.002]";

const METODO_TSQ: &str = r#"{"ms":{"periodi":[{"esperimenti":[{"transizioni":[
{"q1":300,"q3":100,"ce_eV":18.5,"polarita":"negative"},
{"q1":300,"q3":150.001,"ce_eV":30,"polarita":"negative"},
{"q1":400,"q3":200,"ce_eV":22,"polarita":"negative"}]}]}]}}"#;

#[test]
fn a_tsq_filter_without_ce_takes_ce_and_polarity_from_the_method() {
    let mut l = fake_with(TSQ_FILTER, 1, METODO_TSQ);
    let ids: Vec<String> = l
        .chromatograms(0)
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
fn a_tsq_filter_without_method_does_not_invent_the_ce() {
    let mut l = fake_with(TSQ_FILTER, 1, "");
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids[0], "SRM SIC Q1=300 Q3=100");
    let mut l = fake_with(TSQ_FILTER, -1, "not json");
    assert_eq!(
        l.chromatograms(0).unwrap()[0].id,
        "SRM SIC Q1=300 Q3=100 polarity=negative"
    );
}

#[test]
fn a_transition_duplicated_in_the_method_stays_as_the_scans() {
    let m = r#"{"ms":{"periodi":[{"esperimenti":[{"transizioni":[
{"q1":300,"q3":100,"ce_eV":10,"polarita":"positive"},
{"q1":300,"q3":100,"ce_eV":40,"polarita":"negative"}]}]}]}}"#;
    let mut l = fake_with(TSQ_FILTER, 1, m);
    assert_eq!(l.chromatograms(0).unwrap()[0].id, "SRM SIC Q1=300 Q3=100");
}

#[test]
fn crm_counts_as_srm() {
    let mut l = finto("+ c ESI CRM ms2 300.0@cid25.0 [100.0-100.0]");
    assert_eq!(l.chromatograms(0).unwrap().len(), 3);
}

#[test]
fn without_srm_filter_no_traces() {
    let mut l = finto("+ c Full ms2 300.0@cid25.0 [50.0-400.0]");
    assert!(l.chromatograms(0).unwrap().is_empty());
    assert!(!ha_scansioni_srm(&mut l, 0));
}

#[test]
fn the_report_of_the_not_carried_is_that_of_the_wrapped_reader() {
    assert_eq!(finto("").not_carried(), &[("campo finto", "motivo finto")]);
}

#[test]
fn the_profile_and_the_light_header_pass_through_the_wrapped_reader() {
    // a Thermo `.raw` keeps its profile next to the centroids: `--peaks profile` must reach it through the wrapper
    let mut l = finto("+ c SRM ms2 300.000 [99.5-100.5]");
    assert!(l.scan_profile(0, 0).unwrap().is_some_and(|s| s.profile));
    assert_eq!(l.table_header(0).unwrap().unwrap().run, "leggera");
}
