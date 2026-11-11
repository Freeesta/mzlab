// The tests of external.rs: the answers of a fake module become the types of the contract (no JS).
use super::*;
use mzlab_core::readers::Position;

struct Fake;

impl Module for Fake {
    fn samples(&mut self) -> Result<String> {
        Ok(r#"{"formato":"thermo-raw","campioni":[{"id":1,"name":"a","scans":2}]}"#.into())
    }
    fn protocol(&mut self) -> Result<u32> {
        Ok(PROTOCOLLO)
    }
    fn index(&mut self, c: usize) -> Result<Vec<u8>> {
        if c != 0 {
            return Err(Error::new("error.badIndex"));
        }
        Ok(mzlab_core::transport::encode_index(&[
            ScanEntry {
                id: "scan=1".into(),
                rt: 0.1234567890123,
                livello: 2,
                polarita: -1,
                tic: 1e300,
                base_peak: None,
                filtro: "f".into(),
                precursore: Some(445.5),
                position: Some(Position {
                    membro: 0,
                    offset: 4096,
                    lunghezza: 77,
                }),
            },
            ScanEntry {
                id: "scan=2".into(),
                rt: 0.2,
                livello: 1,
                polarita: 1,
                tic: 3.5,
                base_peak: Some(200.25),
                filtro: "g".into(),
                precursore: None,
                position: None,
            },
        ]))
    }
    fn scan(&mut self, _: usize, _: usize) -> Result<Vec<u8>> {
        Ok(mzlab_core::transport::encode_scansione(&Spectrum {
            level: 1,
            rt: 0.5,
            polarity: 1,
            profile: true,
            mz: vec![100.0, 200.5],
            intensity: vec![1.5, 16777216.0],
            wide: Some(vec![1.5, 16777217.0]),
            ..Spectrum::default()
        }))
    }
    fn scan_header(&mut self, _: usize, _: usize) -> Result<String> {
        Ok(r#"[{"name":"a","value":"b","unit":"volt","cv":null}]"#.into())
    }
    fn chromatograms(&mut self, _: usize) -> Result<String> {
        Ok(r#"[{"id":"TIC","kind":"Tic","unit":"u"},{"id":"x","kind":"Nuovo","unit":""}]"#.into())
    }
    fn chromatogram(&mut self, _: usize, id: &str) -> Result<String> {
        Ok(format!(
            r#"{{"id":"{id}","kind":"Srm","q1":300.1,"q3":null,"unit":"u","time":[0.0,1.5],"intensity":[3.0,4.0]}}"#
        ))
    }
    fn method(&mut self, _: usize) -> Result<String> {
        Ok("{}".into())
    }
    fn missing(&mut self, _: usize) -> Result<u32> {
        Ok(3)
    }
    fn file_header(&mut self, _: usize) -> Result<String> {
        Ok(r#"{"run":"r","sources":["r.raw"],"instrument":["MS:1","Orbitrap"],"components":[["analyzer","MS:2","orbitrap"]],"software":["mzLab","1.0"],"acquired":"2026-01-01T00:00:00","serial":null,"source_sha1":["ab"],"source_cv":[["MS:3","Thermo RAW format"]],"sample":null,"run_params":[["k","v"]],"content":[],"extra_software":[],"centroided":true}"#.into())
    }
}

#[test]
fn the_replies_become_the_types_of_the_contract() {
    let mut l = ExternalReader::new(Fake).unwrap();
    assert_eq!(l.format(), Format::ThermoRaw);
    assert_eq!(l.samples()[0].scansioni, 2);
    let i = l.index(0).unwrap().to_vec();
    assert_eq!(i.len(), 2);
    // the numbers come back exactly (the index of the file is made of them)
    assert_eq!(i[0].rt, 0.1234567890123);
    assert_eq!(i[0].tic, 1e300);
    assert_eq!(
        i[0].position,
        Some(Position {
            membro: 0,
            offset: 4096,
            lunghezza: 77
        })
    );
    assert_eq!(
        (i[0].base_peak, i[0].precursore, i[0].polarita),
        (None, Some(445.5), -1)
    );
    assert_eq!((i[1].base_peak, i[1].position), (Some(200.25), None));
    assert_eq!(l.index(1).err().map(|e| e.key), Some("error.badIndex"));
    let s = l.scan(0, 1).unwrap();
    assert_eq!((s.index, s.level, s.profile, s.mz.len()), (1, 1, true, 2));
    // the id and the filter are the ones of the row of the index
    assert_eq!((s.id.as_str(), s.filter.as_str()), ("scan=2", "g"));
    // 16777217 does not fit a 32-bit float: the scan keeps the 64-bit intensities
    assert_eq!(s.wide.as_deref(), Some(&[1.5, 16777217.0][..]));
    assert_eq!(
        l.scan_header(0, 0).unwrap()[0].unita.as_deref(),
        Some("volt")
    );
    let c = l.chromatograms(0).unwrap();
    assert_eq!((c[0].tipo, c[1].tipo), (ChromKind::Tic, ChromKind::Other));
    let x = l.chromatogram(0, "SRM").unwrap();
    assert_eq!(
        (x.kind, x.q1, x.q3, x.intensity),
        (ChromKind::Srm, Some(300.1), None, vec![3.0, 4.0])
    );
    assert_eq!(l.method(0).unwrap(), "{}");
    // the count of a truncated file comes from the module; a module with none (the wrapper below) says 0
    assert_eq!(l.missing(0), 3);
    let t = l.mzml_header(0).unwrap().unwrap();
    assert_eq!(t.instrument, Some(("MS:1".into(), "Orbitrap".into())));
    assert_eq!(
        (t.software, t.sample, t.serial, t.centroided),
        (("mzLab".into(), "1.0".into()), None, None, true)
    );
    assert_eq!(t.run_params, vec![("k".to_string(), "v".to_string())]);
}

#[test]
fn a_module_of_another_protocol_is_refused() {
    struct Old(Fake);
    impl Module for Old {
        fn protocol(&mut self) -> Result<u32> {
            Ok(PROTOCOLLO - 1)
        }
        fn samples(&mut self) -> Result<String> {
            self.0.samples()
        }
        fn index(&mut self, c: usize) -> Result<Vec<u8>> {
            self.0.index(c)
        }
        fn scan(&mut self, c: usize, k: usize) -> Result<Vec<u8>> {
            self.0.scan(c, k)
        }
        fn scan_header(&mut self, c: usize, k: usize) -> Result<String> {
            self.0.scan_header(c, k)
        }
        fn chromatograms(&mut self, c: usize) -> Result<String> {
            self.0.chromatograms(c)
        }
        fn chromatogram(&mut self, c: usize, id: &str) -> Result<String> {
            self.0.chromatogram(c, id)
        }
        fn method(&mut self, c: usize) -> Result<String> {
            self.0.method(c)
        }
        fn file_header(&mut self, c: usize) -> Result<String> {
            self.0.file_header(c)
        }
    }
    let e = ExternalReader::new(Old(Fake)).err().unwrap();
    assert_eq!(
        (e.key, e.params),
        (
            "err.file.wiffLoad",
            vec![
                ("family", "thermo".to_string()),
                ("detail", "protocol".to_string())
            ]
        )
    );
}

#[test]
fn the_module_error_keeps_key_and_params() {
    let e = module_error(r#"{"error_key":"err.file.raw","params":{"detail":"member"}}"#);
    assert_eq!(
        (e.key, e.params),
        ("err.file.raw", vec![("detail", "member".to_string())])
    );
    assert_eq!(module_error("boom").key, "err.module.reply");
    assert_eq!(
        interned("err.file.raw").as_ptr(),
        interned("err.file.raw").as_ptr()
    );
}
