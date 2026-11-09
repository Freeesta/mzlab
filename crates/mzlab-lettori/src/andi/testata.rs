//! What an ANDI file says about itself outside its scans: the global attributes and the `instrument_*` variables. They become the `metodo/1` document and the header of the mzML we write;
//! every global attribute goes into `campi` (nothing is lost), the ones that identify a person, a serial or a path marked `identita`.

use super::netcdf::{Cdf, Val};
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::Limits;
use mzlab_core::metodo::{self, obj, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Field;
use mzlab_core::mzmlwrite::Header;

/// The most fields kept from a file.
const FIELDS_MAX: usize = 2000;

impl Testata {
    /// The attributes and the instrument variables of `cdf`.
    pub(super) fn read(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits) -> Result<Testata> {
        let mut t = Testata::default();
        for a in &cdf.att {
            let v = match &a.val {
                Val::Testo(s) => s.trim().to_string(),
                Val::Num(n) => n
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            };
            if v.is_empty() || t.fields.len() >= FIELDS_MAX {
                continue;
            }
            let c = Field::new("andi:global", &a.nome, &v);
            let c = mappa(&a.nome).map_or(c, |m| Field::new("andi:global", &a.nome, &v).map(m));
            t.fields.push(if identita(&a.nome) { c.id() } else { c });
        }
        t.titolo = cdf.text("experiment_title").map(str::to_string);
        t.created = cdf.text("experiment_date_time_stamp").map(str::to_string);
        t.user = cdf.text("operator_name").map(str::to_string);
        t.ionizzazione = cdf.text("test_ionization_mode").map(str::to_string);
        t.detector = cdf.text("test_detector_type").map(str::to_string);
        for v in cdf.var.iter().filter(|v| v.nome.starts_with("instrument_")) {
            let s = unico(cdf.stringhe(data, limits, v)?).unwrap_or_default();
            if s.is_empty() || t.fields.len() >= FIELDS_MAX {
                continue;
            }
            let c = Field::new("andi:instrument", &v.nome, &s);
            t.fields.push(if identita(&v.nome) { c.id() } else { c });
            let ch = Some(s);
            match v.nome.as_str() {
                "instrument_mfr" => t.maker = ch,
                "instrument_model" => t.model = ch,
                "instrument_serial_no" => t.series = ch,
                "instrument_sw_version" => t.software = ch,
                "instrument_fw_version" => t.firmware = ch,
                _ => {}
            }
        }
        mzlab_core::sortc::slice_by(&mut t.fields, |a, b| {
            (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
        });
        Ok(t)
    }

    /// The `metodo/1` document of the file `nome` (`dim` bytes, `sha1` when read).
    pub(super) fn document(&self, nome: &str, dim: u64, sha1: Option<String>) -> String {
        let src = Source {
            format: "andi-ms",
            files: vec![SrcFile {
                name: nome.to_string(),
                sha1,
                bytes: dim,
            }],
            sample: None,
        };
        let s = |t: &Option<String>| t.as_deref().and_then(text);
        let sw = obj(vec![
            ("nome", text("instrument")),
            ("versione", s(&self.software)),
        ]);
        let doc = obj(vec![
            ("schema", Some(V::s(metodo::SCHEMA))),
            ("origine", metodo::origin(&src, self.created.as_deref())),
            ("campione", obj(vec![("nome", s(&self.titolo))])),
            (
                "strumento",
                obj(vec![
                    ("produttore", s(&self.maker)),
                    ("modello", s(&self.model)),
                    ("serie", s(&self.series)),
                    ("firmware", s(&self.firmware)),
                    (
                        "software",
                        Some(V::Arr(
                            sw.into_iter().filter(|_| self.software.is_some()).collect(),
                        )),
                    ),
                ]),
            ),
            ("operatore", obj(vec![("utente", s(&self.user))])),
            (
                "ms",
                obj(vec![(
                    "sorgente",
                    obj(vec![("tipo", s(&self.ionizzazione))]),
                )]),
            ),
            (
                "campi",
                Some(V::Arr(self.fields.iter().filter_map(Field::json).collect())),
            ),
        ]);
        doc.map_or_else(String::new, |d| d.json())
    }

    /// The header of the mzML: the instrument, this file as the source, the acquisition time and `doc` in the `<run>`.
    pub(super) fn header(&self, name: &str, sha1: Option<String>, doc: String) -> Header {
        let mut h = Header {
            run: name.rsplit_once('.').map_or(name, |(s, _)| s).to_string(),
            software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
            instrument: self.model.clone().map(|m| ("MS:1000031".to_string(), m)),
            serial: self.series.clone(),
            acquired: self.created.as_deref().and_then(iso),
            sources: vec![name.to_string()],
            source_sha1: sha1.into_iter().collect(),
            ..Header::default()
        };
        for (k, v, acc, cv) in [
            (
                "source",
                &self.ionizzazione,
                "MS:1000008",
                "ionization type",
            ),
            ("detector", &self.detector, "MS:1000026", "detector type"),
        ] {
            if let Some(v) = v {
                h.components
                    .push((k.to_string(), acc.to_string(), format!("{cv}: {v}")));
            }
        }
        h.run_params.push((metodo::USERPARAM.to_string(), doc));
        h
    }
}

#[derive(Default)]
pub(super) struct Testata {
    pub(super) fields: Vec<Field>,
    pub(super) titolo: Option<String>,
    pub(super) created: Option<String>,
    pub(super) user: Option<String>,
    pub(super) maker: Option<String>,
    pub(super) model: Option<String>,
    pub(super) series: Option<String>,
    pub(super) software: Option<String>,
    pub(super) firmware: Option<String>,
    pub(super) ionizzazione: Option<String>,
    pub(super) detector: Option<String>,
}

/// The attribute names that identify a person, a computer or a path of the acquisition.
fn identita(name: &str) -> bool {
    matches!(
        name,
        "operator_name" | "instrument_serial_no" | "source_file_reference" | "source_file_name"
    ) || name.starts_with("external_file_ref")
        || name.contains("path")
}

/// Where a global attribute is also in the document.
fn mappa(name: &str) -> Option<&'static str> {
    match name {
        "experiment_title" => Some("campione.nome"),
        "operator_name" => Some("operatore.utente"),
        "test_ionization_mode" => Some("ms.sorgente.tipo"),
        _ => None,
    }
}

fn unico(v: Vec<String>) -> Option<String> {
    v.into_iter().find(|s| !s.is_empty())
}

/// `20190312141102+0000` as `2019-03-12T14:11:02+00:00` (None when the text is not that).
fn iso(t: &str) -> Option<String> {
    let (d, z) = t.split_at(t.find(['+', '-']).unwrap_or(t.len()));
    if d.len() != 14 || !d.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let zone = match z.len() {
        0 => String::new(),
        5 if z[1..].bytes().all(|c| c.is_ascii_digit()) => {
            format!("{}{}:{}", &z[..1], &z[1..3], &z[3..])
        }
        _ => return None,
    };
    Some(format!(
        "{}-{}-{}T{}:{}:{}{zone}",
        &d[..4],
        &d[4..6],
        &d[6..8],
        &d[8..10],
        &d[10..12],
        &d[12..]
    ))
}
