//! The session interface of a module, in plain Rust (the `wasm_bindgen` functions of `lib.rs` only convert): a reader stays open under a handle, and the engine's external reader asks it for the
//! index, a scan, a chromatogram, the header of a scan and the method. Answers are JSON or typed arrays; an error is `{error_key, params}`.
use std::collections::HashMap;

use mzlab_core::lettori::Lettore;
use mzlab_core::Result;

/// A scan: m/z and intensities as `f64`, and the rest (id, level, RT, polarity, profile, filter, precursor) as JSON.
pub struct Scansione {
    pub mz: Vec<f64>,
    pub intensita: Vec<f64>,
    pub meta: String,
}

#[derive(Default)]
pub struct Sessioni {
    aperti: HashMap<u32, Box<dyn Lettore>>,
    prossimo: u32,
}

impl Sessioni {
    /// Keeps `l` open and gives its handle.
    pub fn inserisci(&mut self, l: Box<dyn Lettore>) -> u32 {
        self.prossimo += 1;
        self.aperti.insert(self.prossimo, l);
        self.prossimo
    }

    pub fn chiudi(&mut self, h: u32) {
        self.aperti.remove(&h);
    }

    fn lettore(&mut self, h: u32) -> Result<&mut Box<dyn Lettore>> {
        self.aperti.get_mut(&h).ok_or_else(|| mzlab_core::Error::new("error.badIndex").with("h", h))
    }

    /// `{formato, campioni:[{id,name,scans}]}` of the reader.
    pub fn campioni(&mut self, h: u32) -> Result<String> {
        let l = self.lettore(h)?;
        let c: Vec<_> = l.campioni().iter().map(|c| serde_json::json!({ "id": c.id, "name": c.nome, "scans": c.scansioni })).collect();
        Ok(serde_json::json!({ "formato": l.formato().id(), "campioni": c }).to_string())
    }

    /// The index of sample `c`: one object per scan.
    pub fn indice(&mut self, h: u32, c: usize) -> Result<String> {
        let v: Vec<_> = self
            .lettore(h)?
            .indice(c)?
            .iter()
            .map(|r| {
                let pos = r.posizione.map(|p| serde_json::json!([p.membro, p.offset, p.lunghezza]));
                serde_json::json!({ "id": r.id, "rt": r.rt, "level": r.livello, "polarity": r.polarita, "tic": r.tic, "base": r.picco_base, "filter": r.filtro, "precursor": r.precursore, "pos": pos })
            })
            .collect();
        Ok(serde_json::Value::Array(v).to_string())
    }

    pub fn scansione(&mut self, h: u32, c: usize, k: usize) -> Result<Scansione> {
        let s = self.lettore(h)?.scansione(c, k)?;
        let meta = serde_json::json!({ "id": s.id, "level": s.level, "rt": s.rt, "polarity": s.polarity, "profile": s.profile, "filter": s.filter, "precursor": s.precursor }).to_string();
        let intensita = (0..s.mz.len()).map(|i| s.intensity_f64(i)).collect();
        Ok(Scansione { mz: s.mz, intensita, meta })
    }

    pub fn intestazione(&mut self, h: u32, c: usize, k: usize) -> Result<String> {
        let v: Vec<_> = self
            .lettore(h)?
            .intestazione(c, k)?
            .iter()
            .map(|f| serde_json::json!({ "name": f.nome, "value": f.valore, "unit": f.unita, "cv": f.cv }))
            .collect();
        Ok(serde_json::Value::Array(v).to_string())
    }

    /// The chromatograms of sample `c` without their points.
    pub fn cromatogrammi(&mut self, h: u32, c: usize) -> Result<String> {
        let v: Vec<_> = self.lettore(h)?.cromatogrammi(c)?.iter().map(|x| serde_json::json!({ "id": x.id, "kind": format!("{:?}", x.tipo), "unit": x.unita })).collect();
        Ok(serde_json::Value::Array(v).to_string())
    }

    /// One chromatogram with its points: `{id, kind, q1, q3, unit, time, intensity}`.
    pub fn cromatogramma(&mut self, h: u32, c: usize, id: &str) -> Result<String> {
        let x = self.lettore(h)?.cromatogramma(c, id)?;
        Ok(serde_json::json!({ "id": x.id, "kind": format!("{:?}", x.kind), "q1": x.q1, "q3": x.q3, "unit": x.unit, "time": x.time, "intensity": x.intensity }).to_string())
    }

    /// The `metodo/1` document of sample `c`.
    pub fn metodo(&mut self, h: u32, c: usize) -> Result<String> {
        self.lettore(h)?.metodo(c)
    }
}

#[cfg(test)]
mod tests {
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
            Ok(Spectrum { index: k, id: format!("scan={k}"), level: 1, rt: 0.5, polarity: 1, profile: false, filter: "f".into(), precursor: None, mz: vec![100.0, 200.0], intensity: vec![1.5, 2.5], wide: None })
        }
        fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
            Ok(vec![Campo { nome: "a".into(), valore: "b".into(), unita: None, cv: Some("MS:1".into()) }])
        }
        fn cromatogrammi(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
            Ok(vec![VoceCromatogramma { id: "TIC".into(), tipo: ChromKind::Tic, unita: "u".into() }])
        }
        fn cromatogramma(&mut self, _: usize, id: &str) -> Result<Chromatogram> {
            Ok(Chromatogram { index: 0, id: id.into(), kind: ChromKind::Tic, q1: None, q3: None, unit: "u".into(), time: vec![0.0, 1.0], intensity: vec![3.0, 4.0] })
        }
        fn metodo(&mut self, _: usize) -> Result<String> {
            Ok("{}".into())
        }
    }

    #[test]
    fn la_sessione_risponde_e_chiude() {
        let mut s = Sessioni::default();
        let fin = Finto { campioni: vec![Campione { id: 1, nome: "a".into(), scansioni: 1 }], indice: vec![VoceScansione { id: "scan=0".into(), rt: 0.5, livello: 1, polarita: 1, tic: 4.0, picco_base: Some(200.0), filtro: "f".into(), precursore: None, posizione: None }] };
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
        assert!(s.cromatogramma(h, 0, "TIC").unwrap().contains("\"intensity\":[3.0,4.0]"));
        assert_eq!(s.metodo(h, 0).unwrap(), "{}");
        s.chiudi(h);
        assert_eq!(s.campioni(h).err().map(|e| e.key), Some("error.badIndex"));
    }
}
