// The tests of esterno.rs: the answers of a fake module become the types of the contract (no JS).
use super::*;

struct Finto;

impl Modulo for Finto {
    fn campioni(&mut self) -> Result<String> {
        Ok(r#"{"formato":"thermo-raw","campioni":[{"id":1,"name":"a","scans":2}]}"#.into())
    }
    fn indice(&mut self, c: usize) -> Result<String> {
        if c != 0 {
            return Err(Error::new("error.badIndex"));
        }
        Ok(r#"[{"id":"scan=1","rt":0.1234567890123,"level":2,"polarity":-1,"tic":1e300,"base":null,"filter":"f","precursor":445.5,"pos":[0,4096,77]},
               {"id":"scan=2","rt":0.2,"level":1,"polarity":1,"tic":3.5,"base":200.25,"filter":"g","precursor":null,"pos":null}]"#.into())
    }
    fn scansione(&mut self, _: usize, _: usize) -> Result<(Vec<f64>, Vec<f64>, String)> {
        Ok((vec![100.0, 200.5], vec![1.5, 16777217.0], r#"{"id":"scan=1","level":1,"rt":0.5,"polarity":1,"profile":true,"filter":"f","precursor":null}"#.into()))
    }
    fn intestazione(&mut self, _: usize, _: usize) -> Result<String> {
        Ok(r#"[{"name":"a","value":"b","unit":"volt","cv":null}]"#.into())
    }
    fn cromatogrammi(&mut self, _: usize) -> Result<String> {
        Ok(r#"[{"id":"TIC","kind":"Tic","unit":"u"},{"id":"x","kind":"Nuovo","unit":""}]"#.into())
    }
    fn cromatogramma(&mut self, _: usize, id: &str) -> Result<String> {
        Ok(format!(r#"{{"id":"{id}","kind":"Srm","q1":300.1,"q3":null,"unit":"u","time":[0.0,1.5],"intensity":[3.0,4.0]}}"#))
    }
    fn metodo(&mut self, _: usize) -> Result<String> {
        Ok("{}".into())
    }
    fn testata(&mut self, _: usize) -> Result<String> {
        Ok(r#"{"run":"r","sources":["r.raw"],"instrument":["MS:1","Orbitrap"],"components":[["analyzer","MS:2","orbitrap"]],"software":["mzLab","1.0"],"acquired":"2026-01-01T00:00:00","serial":null,"source_sha1":["ab"],"source_cv":[["MS:3","Thermo RAW format"]],"sample":null,"run_params":[["k","v"]],"content":[],"extra_software":[],"centroided":true}"#.into())
    }
}

#[test]
fn le_risposte_diventano_i_tipi_del_contratto() {
    let mut l = LettoreEsterno::new(Finto).unwrap();
    assert_eq!(l.formato(), Formato::ThermoRaw);
    assert_eq!(l.campioni()[0].scansioni, 2);
    let i = l.indice(0).unwrap().to_vec();
    assert_eq!(i.len(), 2);
    // the numbers come back exactly (the index of the file is made of them)
    assert_eq!(i[0].rt, 0.1234567890123);
    assert_eq!(i[0].tic, 1e300);
    assert_eq!(i[0].posizione, Some(Posizione { membro: 0, offset: 4096, lunghezza: 77 }));
    assert_eq!((i[0].picco_base, i[0].precursore, i[0].polarita), (None, Some(445.5), -1));
    assert_eq!((i[1].picco_base, i[1].posizione), (Some(200.25), None));
    assert_eq!(l.indice(1).err().map(|e| e.key), Some("error.badIndex"));
    let s = l.scansione(0, 1).unwrap();
    assert_eq!((s.index, s.level, s.profile, s.mz.len()), (1, 1, true, 2));
    // 16777217 does not fit a 32-bit float: the scan keeps the 64-bit intensities
    assert_eq!(s.wide.as_deref(), Some(&[1.5, 16777217.0][..]));
    assert_eq!(l.intestazione(0, 0).unwrap()[0].unita.as_deref(), Some("volt"));
    let c = l.cromatogrammi(0).unwrap();
    assert_eq!((c[0].tipo, c[1].tipo), (ChromKind::Tic, ChromKind::Other));
    let x = l.cromatogramma(0, "SRM").unwrap();
    assert_eq!((x.kind, x.q1, x.q3, x.intensity), (ChromKind::Srm, Some(300.1), None, vec![3.0, 4.0]));
    assert_eq!(l.metodo(0).unwrap(), "{}");
    let t = l.testata_mzml(0).unwrap().unwrap();
    assert_eq!(t.instrument, Some(("MS:1".into(), "Orbitrap".into())));
    assert_eq!((t.software, t.sample, t.serial, t.centroided), (("mzLab".into(), "1.0".into()), None, None, true));
    assert_eq!(t.run_params, vec![("k".to_string(), "v".to_string())]);
}

#[test]
fn le_intensita_di_32_bit_non_diventano_larghe() {
    let s = RisposteScansioni::scansione(0, vec![1.0], vec![2.5], "{}").unwrap();
    assert!(s.wide.is_none());
    assert!(RisposteScansioni::scansione(0, vec![1.0, 2.0], vec![2.5], "{}").is_err());
}

#[test]
fn l_errore_del_modulo_conserva_chiave_e_parametri() {
    let e = errore_del_modulo(r#"{"error_key":"err.file.raw","params":{"detail":"member"}}"#);
    assert_eq!((e.key, e.params), ("err.file.raw", vec![("detail", "member".to_string())]));
    assert_eq!(errore_del_modulo("boom").key, "err.file.notAccepted");
    assert_eq!(statica("err.file.raw").as_ptr(), statica("err.file.raw").as_ptr());
}
