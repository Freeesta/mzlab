//! What a timsTOF run says about itself outside its spectra: `GlobalMetadata`, the properties of the first frame and the calibrations become the `metodo/1` document and the header of
//! the mzML we write. Every key of `GlobalMetadata` goes into `campi` (nothing is lost); names, serials and paths are marked `identita`.

use super::tabelle::Dati;
use mzlab_core::metodo::{self, obj, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Campo;
use mzlab_core::mzmlwrite::Header;

/// The most fields kept from a run.
const CAMPI_MAX: usize = 2000;

#[derive(Default)]
pub struct Testata {
    pub campi: Vec<Campo>,
    pub modello: Option<String>,
    pub serie: Option<String>,
    pub produttore: Option<String>,
    pub software: Option<(String, Option<String>)>,
    pub firmware: Option<String>,
    pub creato: Option<String>,
    pub utente: Option<String>,
    pub campione: Option<String>,
    pub tsf: bool,
}

fn identita(nome: &str) -> bool {
    let n = nome.to_ascii_lowercase();
    [
        "operator",
        "serial",
        "path",
        "directory",
        "computer",
        "user",
    ]
    .iter()
    .any(|k| n.contains(k))
}

impl Testata {
    pub fn leggi(d: &Dati, tsf: bool) -> Testata {
        let m = |k: &str| {
            d.meta
                .get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let mut t = Testata {
            modello: m("InstrumentName"),
            serie: m("InstrumentSerialNumber"),
            produttore: m("InstrumentVendor"),
            software: m("AcquisitionSoftware").map(|s| (s, m("AcquisitionSoftwareVersion"))),
            firmware: m("AcquisitionFirmwareVersion"),
            creato: m("AcquisitionDateTime"),
            utente: m("OperatorName"),
            campione: m("SampleName"),
            tsf,
            ..Testata::default()
        };
        let mut aggiungi = |c: Campo, id: bool| {
            if t.campi.len() < CAMPI_MAX {
                t.campi.push(if id { c.id() } else { c });
            }
        };
        for (k, v) in d.meta.iter().filter(|(_, v)| !v.trim().is_empty()) {
            let c = Campo::new("bruker:global", k, v.trim());
            let c = match k.as_str() {
                "OperatorName" => c.map("operatore.utente"),
                "SampleName" => c.map("campione.nome"),
                "InstrumentName" => c.map("strumento.modello"),
                "InstrumentSerialNumber" => c.map("strumento.serie"),
                _ => c,
            };
            aggiungi(c, identita(k));
        }
        for (k, v, u) in &d.proprieta {
            aggiungi(Campo::new("bruker:proprieta", k, v).unit(u), identita(k));
        }
        for (k, v) in &d.tims {
            aggiungi(Campo::new("bruker:TimsCalibration", k, v), false);
        }
        if let Some(c) = d.frames.first().and_then(|f| d.cal.get(&f.cal)) {
            aggiungi(
                Campo::new("bruker:MzCalibration", "ModelType", c.modello),
                false,
            );
            aggiungi(
                Campo::new("bruker:MzCalibration", "DigitizerTimebase", c.timebase),
                false,
            );
            aggiungi(
                Campo::new("bruker:MzCalibration", "DigitizerDelay", c.delay),
                false,
            );
            for (i, x) in c.c.iter().enumerate() {
                aggiungi(
                    Campo::new("bruker:MzCalibration", &format!("C{i}"), x),
                    false,
                );
            }
        }
        mzlab_core::sortc::slice_by(&mut t.campi, |a, b| {
            (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
        });
        t
    }

    /// The `metodo/1` document. `files`: the members read, with the SHA-1 of the database when it was computed.
    pub fn documento(&self, files: Vec<SrcFile>) -> String {
        let src = Source {
            format: if self.tsf { "bruker-tsf" } else { "bruker-tdf" },
            files,
            sample: None,
        };
        let s = |t: &Option<String>| t.as_deref().and_then(text);
        let sw = self
            .software
            .as_ref()
            .and_then(|(n, v)| obj(vec![("nome", text(n)), ("versione", s(v))]));
        let nondec = obj(vec![
            ("chiave", Some(V::s("meta.nonDecodificato"))),
            (
                "dove",
                text("TimsCalibration: 1/K0 of each scan (the spectra carry their scan range)"),
            ),
        ]);
        let doc = obj(vec![
            ("schema", Some(V::s(metodo::SCHEMA))),
            ("origine", metodo::origin(&src, self.creato.as_deref())),
            ("campione", obj(vec![("nome", s(&self.campione))])),
            (
                "strumento",
                obj(vec![
                    ("produttore", s(&self.produttore)),
                    ("modello", s(&self.modello)),
                    ("serie", s(&self.serie)),
                    ("firmware", s(&self.firmware)),
                    ("software", sw.map(|x| V::Arr(vec![x]))),
                ]),
            ),
            ("operatore", obj(vec![("utente", s(&self.utente))])),
            (
                "campi",
                Some(V::Arr(self.campi.iter().filter_map(Campo::json).collect())),
            ),
            (
                "avvisi",
                (!self.tsf).then(|| V::Arr(nondec.into_iter().collect())),
            ),
        ]);
        doc.map_or_else(String::new, |d| d.json())
    }

    pub fn header(&self, nome: &str, files: &[(String, Option<String>)], doc: String) -> Header {
        let mut h = Header {
            run: nome
                .trim_end_matches('/')
                .trim_end_matches(".d")
                .trim_end_matches(".D")
                .to_string(),
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            instrument: self.modello.clone().map(|m| ("MS:1000031".to_string(), m)),
            serial: self.serie.clone(),
            acquired: self.creato.clone(),
            sources: files.iter().map(|f| f.0.clone()).collect(),
            source_sha1: files.iter().filter_map(|f| f.1.clone()).collect(),
            ..Header::default()
        };
        if let Some((n, v)) = &self.software {
            h.extra_software.push((
                "MS:1000799".into(),
                n.clone(),
                v.clone().unwrap_or_default(),
            ));
        }
        h.run_params.push((metodo::USERPARAM.to_string(), doc));
        h
    }
}
