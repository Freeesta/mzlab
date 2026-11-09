//! The header of an mzXML (everything before the first `<scan`): the instrument, the files the data come from, the processing, the operator. It becomes the `metodo/1` document and the
//! header of the mzML we write; every attribute of every element goes into `campi` (nothing is lost), the identifying ones (operator, paths) marked `identita`.

use mzlab_core::error::Result;
use mzlab_core::lettori::Limits;
use mzlab_core::metodo::{self, obj, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Field;
use mzlab_core::mzmlwrite::Header;

use super::mzxml_scan::{attr, each, name, tags, Tag};

/// The most fields kept from a header.
const FIELDS_MAX: usize = 2000;

#[derive(Default)]
pub(super) struct Cabina {
    pub(super) fields: Vec<Field>,
    pub(super) maker: Option<String>,
    pub(super) model: Option<String>,
    pub(super) ionizzazione: Option<String>,
    pub(super) analizzatore: Option<String>,
    pub(super) detector: Option<String>,
    /// (name, version) of the programs that acquired and processed the data.
    pub(super) software: Vec<(String, String)>,
    /// (name, SHA-1) of the `parentFile` elements.
    pub(super) file: Vec<(String, String)>,
    pub(super) user: Option<String>,
    /// The `centroided` of `dataProcessing`.
    pub(super) centroidi: Option<bool>,
}

fn mappa(key: &str) -> Option<&'static str> {
    match key {
        "msManufacturer" => Some("strumento.produttore"),
        "msModel" => Some("strumento.modello"),
        "msIonisation" => Some("ms.sorgente.tipo"),
        _ => None,
    }
}

/// Whether a file name is a path of the acquisition computer.
fn file_path(n: &str) -> bool {
    n.contains(['/', '\\'])
}

/// The last component of a path of either kind.
fn last_one(n: &str) -> &str {
    n.rsplit(['/', '\\']).next().unwrap_or(n)
}

/// The fields of one element: `category`/`value` pairs (`msModel`…) are one field, any other attribute is one.
fn fields_of(o: &str, n: &str, g: Tag, dest: &mut Vec<Field>) {
    let (cat, val) = (attr(g, "category"), attr(g, "value"));
    if !cat.is_empty() && !val.is_empty() {
        let f = Field::new(o, cat, val);
        dest.push(match mappa(cat) {
            Some(m) => f.map(m),
            None => f,
        });
        return;
    }
    each(g, &mut |k, v| {
        let f = Field::new(o, k, v);
        let identita = n == "operator" || (n == "parentFile" && k == "fileName" && file_path(v));
        dest.push(if identita { f.id() } else { f });
    });
}

fn annotate(c: &mut Cabina, n: &str, g: Tag) {
    let v = |k: &str| Some(attr(g, k).trim().to_string()).filter(|v| !v.is_empty());
    match n {
        "msManufacturer" => c.maker = v("value"),
        "msModel" => c.model = v("value"),
        "msIonisation" => c.ionizzazione = v("value"),
        "msMassAnalyzer" => c.analizzatore = v("value"),
        "msDetector" => c.detector = v("value"),
        "software" => {
            if let Some(n) = v("name") {
                c.software.push((n, v("version").unwrap_or_default()));
            }
        }
        "parentFile" => c.file.push((
            v("fileName").unwrap_or_default(),
            v("fileSha1").unwrap_or_default(),
        )),
        "operator" => {
            let n: Vec<String> = [v("first"), v("last")].into_iter().flatten().collect();
            c.user = (!n.is_empty()).then(|| n.join(" "));
        }
        "dataProcessing" => c.centroidi = v("centroided").map(|x| x == "1" || x == "true"),
        _ => {}
    }
}

/// The cabin of `head`, the text of the file before its first scan.
pub(super) fn read(head: &str, limits: &Limits) -> Result<Cabina> {
    let mut c = Cabina::default();
    let mut pila: Vec<&str> = Vec::new();
    let mut error = None;
    tags(head, &mut |g, _| {
        let n = name(g);
        if n.starts_with('/') {
            pila.pop();
            return true;
        }
        if c.fields.len() < FIELDS_MAX && n != "mzXML" {
            let mut o = String::from("mzxml:");
            for p in &pila {
                o.push_str(p);
                o.push('/');
            }
            o.push_str(n);
            fields_of(&o, n, g, &mut c.fields);
        }
        annotate(&mut c, n, g);
        if !g.ends_with('/') {
            if let Err(e) = limits.profondita_ok(pila.len() + 1) {
                error = Some(e);
                return false;
            }
            pila.push(n);
        }
        true
    });
    error.map_or(Ok(c), Err)
}

/// The `metodo/1` document of the file `nome` (`dim` bytes, `sha1` when read).
pub(super) fn document(c: &Cabina, nome: &str, dim: u64, sha1: Option<String>) -> String {
    let src = Source {
        format: "mzxml",
        files: vec![SrcFile {
            name: nome.to_string(),
            sha1,
            bytes: dim,
        }],
        sample: None,
    };
    let software = c
        .software
        .iter()
        .filter_map(|(n, v)| obj(vec![("nome", text(n)), ("versione", text(v))]))
        .collect();
    let s = |t: &Option<String>| t.as_deref().and_then(text);
    let doc = obj(vec![
        ("schema", Some(V::s(metodo::SCHEMA))),
        ("origine", metodo::origin(&src, None)),
        (
            "strumento",
            obj(vec![
                ("produttore", s(&c.maker)),
                ("modello", s(&c.model)),
                ("software", Some(V::Arr(software))),
            ]),
        ),
        ("operatore", obj(vec![("utente", s(&c.user))])),
        (
            "ms",
            obj(vec![("sorgente", obj(vec![("tipo", s(&c.ionizzazione))]))]),
        ),
        (
            "campi",
            Some(V::Arr(c.fields.iter().filter_map(Field::json).collect())),
        ),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}

/// The header of the mzML: the instrument, the files the data come from (this file when the mzXML names none), the programs, and `doc` in the `<run>`.
pub(super) fn header(c: &Cabina, name: &str, sha1: Option<String>, doc: String) -> Header {
    let mut h = Header {
        run: name.rsplit_once('.').map_or(name, |(s, _)| s).to_string(),
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        instrument: c.model.clone().map(|m| ("MS:1000031".to_string(), m)),
        ..Header::default()
    };
    for (k, value, acc, cv_name) in [
        ("source", &c.ionizzazione, "MS:1000008", "ionization type"),
        (
            "analyzer",
            &c.analizzatore,
            "MS:1000443",
            "mass analyzer type",
        ),
        ("detector", &c.detector, "MS:1000026", "detector type"),
    ] {
        if let Some(v) = value {
            h.components
                .push((k.to_string(), acc.to_string(), format!("{cv_name}: {v}")));
        }
    }
    for (n, sha) in &c.file {
        h.sources.push(last_one(n).to_string());
        h.source_sha1.push(sha.clone());
    }
    if h.sources.is_empty() {
        h.sources.push(name.to_string());
        h.source_sha1.extend(sha1);
    }
    h.extra_software = c
        .software
        .iter()
        .map(|(n, v)| ("MS:1000531".to_string(), n.clone(), v.clone()))
        .collect();
    h.run_params.push((metodo::USERPARAM.to_string(), doc));
    h
}
