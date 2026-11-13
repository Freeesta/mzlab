//! What a timsTOF run says about itself outside its spectra: `GlobalMetadata`, the properties of the first frame and the calibrations become the `metodo/1` document and the header of
//! the mzML we write. Every key of `GlobalMetadata` goes into `campi` (nothing is lost); names, serials and paths are marked `identita`.

use super::tables::Data;

use mzlab_core::method::{self, obj, text, Source, SrcFile, V};

use mzlab_core::method_fields::Field;

use mzlab_core::mzmlwrite::Header;

/// The most fields kept from a run.
const FIELDS_MAX: usize = 2000;

impl BrukerHeader {
    pub fn read(d: &Data, tsf: bool) -> BrukerHeader {
        let m = |k: &str| {
            d.meta
                .get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let mut t = BrukerHeader {
            model: m("InstrumentName"),
            series: m("InstrumentSerialNumber"),
            maker: m("InstrumentVendor"),
            software: m("AcquisitionSoftware").map(|s| (s, m("AcquisitionSoftwareVersion"))),
            firmware: m("AcquisitionFirmwareVersion"),
            created: m("AcquisitionDateTime"),
            user: m("OperatorName"),
            sample: m("SampleName"),
            tsf,
            damaged: d.damaged.clone(),
            ..BrukerHeader::default()
        };
        let mut add = |c: Field, id: bool| {
            if t.fields.len() < FIELDS_MAX {
                t.fields.push(if id { c.id() } else { c });
            }
        };
        for (k, v) in d.meta.iter().filter(|(_, v)| !v.trim().is_empty()) {
            let c = Field::new("bruker:global", k, v.trim());
            let c = match k.as_str() {
                "OperatorName" => c.map("operatore.utente"),
                "SampleName" => c.map("campione.nome"),
                "InstrumentName" => c.map("strumento.modello"),
                "InstrumentSerialNumber" => c.map("strumento.serie"),
                _ => c,
            };
            add(c, identity(k));
        }
        for (k, v, u) in &d.proprieta {
            add(Field::new("bruker:proprieta", k, v).unit(u), identity(k));
        }
        for (k, v) in &d.tims {
            add(Field::new("bruker:TimsCalibration", k, v), false);
        }
        if let Some(c) = d.frames.first().and_then(|f| d.cal.get(&f.cal)) {
            add(
                Field::new("bruker:MzCalibration", "ModelType", c.model),
                false,
            );
            add(
                Field::new("bruker:MzCalibration", "DigitizerTimebase", c.timebase),
                false,
            );
            add(
                Field::new("bruker:MzCalibration", "DigitizerDelay", c.delay),
                false,
            );
            for (i, x) in c.c.iter().enumerate() {
                add(
                    Field::new("bruker:MzCalibration", &format!("C{i}"), x),
                    false,
                );
            }
        }
        mzlab_core::sortc::slice_by(&mut t.fields, |a, b| {
            (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
        });
        t
    }

    /// The `metodo/1` document. `files`: the members read, with the SHA-1 of the database when it was computed.
    pub fn document(&self, files: Vec<SrcFile>) -> String {
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
            ("schema", Some(V::s(method::SCHEMA))),
            ("origine", method::origin(&src, self.created.as_deref())),
            ("campione", obj(vec![("nome", s(&self.sample))])),
            (
                "strumento",
                obj(vec![
                    ("produttore", s(&self.maker)),
                    ("modello", s(&self.model)),
                    ("serie", s(&self.series)),
                    ("firmware", s(&self.firmware)),
                    ("software", sw.map(|x| V::Arr(vec![x]))),
                ]),
            ),
            ("operatore", obj(vec![("utente", s(&self.user))])),
            (
                "campi",
                Some(V::Arr(self.fields.iter().filter_map(Field::json).collect())),
            ),
            ("avvisi", self.notes(nondec)),
        ]);
        doc.map_or_else(String::new, |d| d.json())
    }

    /// The notes of the method document: the 1/K0 not decoded (TDF), then each damaged side table; none for a TSF with nothing damaged.
    fn notes(&self, nondec: Option<V>) -> Option<V> {
        let db = if self.tsf {
            "analysis.tsf"
        } else {
            "analysis.tdf"
        };
        let mut out: Vec<V> = nondec.filter(|_| !self.tsf).into_iter().collect();
        let damaged = self.damaged.iter().map(|t| format!("{db}: table {t}"));
        let uncal = (self.uncalibrated > 0).then(|| {
            format!(
                "{db}: {} spectra whose frame has no usable MzCalibration",
                self.uncalibrated
            )
        });
        out.extend(damaged.chain(uncal).filter_map(|d| {
            obj(vec![
                ("chiave", Some(V::s("meta.danneggiato"))),
                ("dove", text(&d)),
            ])
        }));
        (!out.is_empty()).then_some(V::Arr(out))
    }

    pub fn header(&self, name: &str, files: &[(String, Option<String>)], doc: String) -> Header {
        let mut h = Header {
            run: name
                .trim_end_matches('/')
                .trim_end_matches(".d")
                .trim_end_matches(".D")
                .to_string(),
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            instrument: self.model.clone().map(|m| ("MS:1000031".to_string(), m)),
            serial: self.series.clone(),
            acquired: self.created.clone(),
            sources: files.iter().map(|f| f.0.clone()).collect(),
            source_sha1: files.iter().filter_map(|f| f.1.clone()).collect(),
            // every timsTOF is a quadrupole-TOF: the scan table reads the analyzer (high resolution) from these, the file does not name them
            components: [
                ("analyzer", "MS:1000081", "quadrupole"),
                ("analyzer", "MS:1000084", "time-of-flight"),
                ("detector", "MS:1000114", "microchannel plate detector"),
            ]
            .iter()
            .map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string()))
            .collect(),
            ..Header::default()
        };
        // the generic term `software`: the writer keeps its CV name and puts the program the file names in a userParam (`header_term`)
        if let Some((n, v)) = &self.software {
            h.extra_software.push((
                "MS:1000531".into(),
                n.clone(),
                v.clone().unwrap_or_default(),
            ));
        }
        h.run_params.push((method::USERPARAM.to_string(), doc));
        h
    }
}

#[derive(Default)]
pub struct BrukerHeader {
    pub fields: Vec<Field>,
    pub model: Option<String>,
    pub series: Option<String>,
    pub maker: Option<String>,
    pub software: Option<(String, Option<String>)>,
    pub firmware: Option<String>,
    pub created: Option<String>,
    pub user: Option<String>,
    pub sample: Option<String>,
    pub tsf: bool,
    /// Side tables found damaged (`Data::damaged`): named in `avvisi` as `meta.danneggiato`.
    pub damaged: Vec<&'static str>,
    /// Spectra left out because their frame has no usable `MzCalibration`: counted in `avvisi` as `meta.danneggiato`.
    pub uncalibrated: usize,
}

fn identity(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
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
