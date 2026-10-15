//! The `metodo/1` document (`metodo.rs`) of an Agilent `.D` run: the sample and the sequence row, the instrument (model, serial number, firmware, acquisition
//! program), the people and the computer, the EI source with its tune values, the scan range or the SIM groups of the method, and in `campi` every name/value of
//! the side files (`AgilentD::metadata`) with the section that carries it. The method report, the run start and the converter log go whole to `grezzo`. A block of
//! the folder that is not decoded is named in `avvisi`.

use super::AgilentD;
use crate::metodo::{self, num, obj, text, Source, V};
use crate::metodo_campi::{Campo, Ident};

const GRAW_MAX: usize = 256 * 1024;
/// The files whose text goes whole to `grezzo` (origin, title).
const TEXTS: [(&str, &str); 3] = [
    ("acqmeth.txt", "Method GC/MS"),
    ("runstart.txt", "Run start"),
    ("IndexedDataConverter.log", "Converter log"),
];
const SEQ: &str = "AcqData/sequence.xml";
const SEQ_ROW: &str = "SequenceTableDataSet/Sequence/";

impl AgilentD {
    fn seq(&self, name: &str) -> Option<&str> {
        self.field(SEQ, &format!("{SEQ_ROW}{name}"))
    }

    fn seq_number(&self, name: &str) -> Option<f64> {
        self.seq(name)?.trim().replace(',', ".").parse().ok()
    }

    /// The first computer the audit trail names.
    fn computer(&self) -> Option<&str> {
        self.field(
            "AcqData-CumulativeAuditTrail.xml",
            "AuditTrailDataSet/AuditTrailEntry/ComputerName",
        )
    }

    fn operator(&self) -> Option<&str> {
        self.field(SEQ, "SequenceTableDataSet@SequenceOperator")
            .filter(|v| !v.trim().is_empty())
            .or_else(|| self.field("AcqData/sample_info.xml", "OperatorName"))
    }
}

fn campione(d: &AgilentD) -> Option<V> {
    obj(vec![
        ("nome", d.sample_name().and_then(text)),
        ("vial", d.seq("Vial").and_then(text)),
        ("iniezione_uL", num(d.seq_number("InjectorVolume"))),
        ("metodo_file", d.seq("AcqMethodFileName").and_then(text)),
        ("diluizione", num(d.seq_number("Dilution"))),
    ])
}

fn strumento(d: &AgilentD) -> Option<V> {
    let dev = d.device.as_ref();
    let model = dev.map(|v| format!("{} {}", v.model, v.name));
    let soft = d.acquisition_software().and_then(|(_, name, version)| {
        obj(vec![("nome", text(&name)), ("versione", text(&version))])
    });
    obj(vec![
        ("produttore", Some(V::s("Agilent Technologies"))),
        ("modello", model.as_deref().and_then(text)),
        ("serie", dev.and_then(|v| text(&v.serial))),
        ("firmware", dev.and_then(|v| text(&v.firmware))),
        ("software", soft.map(|s| V::Arr(vec![s]))),
    ])
}

fn param(id: &str, label: &str, value: f64, unit: &str) -> Option<V> {
    obj(vec![
        ("id", text(id)),
        ("etichetta", text(label)),
        ("valore", num(Some(value))),
        ("unita", text(unit)),
    ])
}

/// The tune values of the method report that are numbers (`tune EMISSION`, …): the settings of the EI source and of the analyser.
fn tune(d: &AgilentD) -> Vec<V> {
    d.metadata
        .iter()
        .filter(|i| i.origin == "acqmeth.txt")
        .filter_map(|i| {
            let id = i.name.strip_prefix("tune ")?;
            let v: f64 = i.value.split_whitespace().next()?.parse().ok()?;
            param(id, "", v, "")
        })
        .collect()
}

/// One experiment for a scan method (its mass range), one per group for a SIM method (start time, resolution, and each ion with its dwell time).
fn experiments(d: &AgilentD) -> Vec<V> {
    let Some(m) = &d.method else { return vec![] };
    if m.mode == "SIM" {
        return m
            .groups
            .iter()
            .enumerate()
            .filter_map(|(k, g)| {
                let mut p = vec![param("start", "Group start time", g.start_min, "min")];
                p.push(
                    g.resolution
                        .and_then(|r| param("resolution", "Resolution", r, "")),
                );
                for (n, (mz, dwell)) in g.ions.iter().enumerate() {
                    p.push(param(&format!("ion{}", n + 1), "SIM ion", *mz, "m/z"));
                    p.push(param(
                        &format!("dwell{}", n + 1),
                        "Dwell time",
                        *dwell,
                        "ms",
                    ));
                }
                obj(vec![
                    ("indice", Some(V::Num((k + 1) as f64))),
                    ("tipo", Some(V::s("SIM"))),
                    ("parametri", Some(V::Arr(p.into_iter().flatten().collect()))),
                ])
            })
            .collect();
    }
    let range = m.low_mass.zip(m.high_mass);
    let mut p = vec![
        m.solvent_delay_min
            .and_then(|v| param("solvent_delay", "Solvent delay", v, "min")),
        m.threshold
            .and_then(|v| param("threshold", "Threshold", v, "")),
    ];
    p.retain(Option::is_some);
    obj(vec![
        ("indice", Some(V::Num(1.0))),
        ("mz", range.map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)]))),
        ("parametri", Some(V::Arr(p.into_iter().flatten().collect()))),
    ])
    .into_iter()
    .collect()
}

fn ms(d: &AgilentD) -> Option<V> {
    obj(vec![
        (
            "sorgente",
            obj(vec![
                ("tipo", d.is_gc().then(|| V::s("EI"))),
                ("parametri", Some(V::Arr(tune(d)))),
            ]),
        ),
        (
            "periodi",
            obj(vec![("esperimenti", Some(V::Arr(experiments(d))))]).map(|p| V::Arr(vec![p])),
        ),
    ])
}

/// Where a field is carried by a section of the document (`mappato`), by origin and name.
fn mapped(origin: &str, name: &str) -> Option<&'static str> {
    match (origin, name) {
        ("AcqData/Devices.xml", n) if n.ends_with("/SerialNumber") => Some("strumento.serie"),
        ("AcqData/Devices.xml", n) if n.ends_with("/FirmwareVersion") => Some("strumento.firmware"),
        ("AcqData/Devices.xml", n) if n.ends_with("/ModelNumber") || n.ends_with("/Name") => {
            Some("strumento.modello")
        }
        ("AcqData/Contents.xml", "Contents/AcqSoftwareVersion") => Some("strumento.software"),
        ("AcqData/Contents.xml", "Contents/AcquiredTime") => Some("origine.file_creato"),
        (SEQ, "SequenceTableDataSet@SequenceOperator") => Some("operatore.utente"),
        ("AcqData/sample_info.xml", "OperatorName") => Some("operatore.utente"),
        (SEQ, n) if n.ends_with("/SampleName") => Some("campione.nome"),
        (SEQ, n) if n.ends_with("/Vial") => Some("campione.vial"),
        (SEQ, n) if n.ends_with("/InjectorVolume") => Some("campione.iniezione_uL"),
        (SEQ, n) if n.ends_with("/AcqMethodFileName") => Some("campione.metodo_file"),
        (SEQ, n) if n.ends_with("/Dilution") => Some("campione.diluizione"),
        ("AcqData-CumulativeAuditTrail.xml", n) if n.ends_with("/ComputerName") => {
            Some("operatore.computer")
        }
        ("acqmeth.txt", n) if n.starts_with("tune ") => Some("ms.sorgente.parametri"),
        _ => None,
    }
}

/// A value that is a Windows path (`D:\MassHunter\…`).
fn is_path(v: &str) -> bool {
    let b = v.as_bytes();
    (b.len() > 3 && b[0].is_ascii_alphabetic() && &b[1..3] == b":\\") || v.starts_with("\\\\")
}

/// Fields that name a person or a place by their kind, whatever the value.
fn is_identity(origin: &str, name: &str) -> bool {
    name.ends_with("/SerialNumber")
        || name.ends_with("/ComputerName")
        || name.ends_with("/User")
        || name.ends_with("OperatorName")
        || (origin == SEQ && name == "SequenceTableDataSet@SequenceOperator")
}

fn ident(d: &AgilentD) -> Ident {
    Ident {
        computer: d.computer().unwrap_or("").to_string(),
        paths: d
            .metadata
            .iter()
            .filter(|i| is_path(&i.value))
            .map(|i| i.value.clone())
            .collect(),
        names: d
            .device
            .iter()
            .map(|v| v.serial.clone())
            .filter(|s| !s.is_empty())
            .collect(),
    }
}

fn fields(d: &AgilentD, ident: &Ident) -> Vec<V> {
    let mut all: Vec<Campo> = d
        .metadata
        .iter()
        .filter(|i| !(i.name == "text" && TEXTS.iter().any(|(o, _)| *o == i.origin)))
        .map(|i| {
            let mut c = Campo::new(&i.origin, &i.name, &i.value);
            c.mappato = mapped(&i.origin, &i.name);
            c.identita = is_identity(&i.origin, &i.name) || ident.names(&i.value);
            c
        })
        .collect();
    all.retain(|c| !c.valore.is_empty());
    crate::sortc::slice_by(&mut all, |a, b| {
        (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
    });
    all.iter().filter_map(Campo::json).collect()
}

/// The texts the folder holds whole (method report, run start, converter log), cut to `GRAW_MAX` in all.
fn raw_texts(d: &AgilentD) -> Vec<V> {
    let mut left = GRAW_MAX;
    let mut out = Vec::new();
    for (origin, title) in TEXTS {
        let Some(t) = d
            .metadata
            .iter()
            .find(|i| i.origin == origin && i.name == "text")
        else {
            continue;
        };
        let t = &t.value;
        let mut end = t.len().min(left);
        while !t.is_char_boundary(end) {
            end -= 1;
        }
        let part = t.get(..end).unwrap_or("");
        left = left.saturating_sub(part.len());
        out.extend(obj(vec![
            ("sezione", Some(V::s(title))),
            ("testo", text(part)),
        ]));
    }
    out
}

/// The blocks the reader keeps only as bytes and SHA-1 (`undecoded bytes` in the metadata).
fn warnings(d: &AgilentD) -> Vec<V> {
    let mut seen: Vec<&str> = Vec::new();
    for i in d.metadata.iter().filter(|i| i.name == "undecoded bytes") {
        if !seen.contains(&i.origin.as_str()) {
            seen.push(&i.origin);
        }
    }
    seen.into_iter()
        .filter_map(|o| {
            obj(vec![
                ("chiave", Some(V::s("meta.nonDecodificato"))),
                ("dove", text(o)),
            ])
        })
        .collect()
}

impl AgilentD {
    /// The `metodo/1` document of the run. Never an error: what is missing is absent.
    pub fn metodo(&self, src: &Source) -> String {
        let ident = ident(self);
        let created = self
            .acquired()
            .map(|t| t.chars().take(19).collect::<String>());
        let doc = obj(vec![
            ("schema", Some(V::s(metodo::SCHEMA))),
            ("origine", metodo::origin(src, created.as_deref())),
            ("campione", campione(self)),
            ("strumento", strumento(self)),
            (
                "operatore",
                obj(vec![
                    ("utente", self.operator().and_then(text)),
                    ("computer", self.computer().and_then(text)),
                ]),
            ),
            ("ms", ms(self)),
            ("campi", Some(V::Arr(fields(self, &ident)))),
            ("grezzo", Some(V::Arr(raw_texts(self)))),
            ("avvisi", Some(V::Arr(warnings(self)))),
        ]);
        doc.map_or_else(String::new, |d| d.json())
    }
}
