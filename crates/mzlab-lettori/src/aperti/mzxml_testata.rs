//! The header of an mzXML (everything before the first `<scan`): the instrument, the files the data come from, the processing, the operator. It becomes the `metodo/1` document and the
//! header of the mzML we write; every attribute of every element goes into `campi` (nothing is lost), the identifying ones (operator, paths) marked `identita`.

use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;
use mzlab_core::metodo::{self, obj, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Campo;
use mzlab_core::mzmlwrite::Header;

use super::mzxml_scan::{attr, each, nome, tags, Tag};

/// The most fields kept from a header.
const CAMPI_MAX: usize = 2000;

#[derive(Default)]
pub(super) struct Cabina {
    pub(super) campi: Vec<Campo>,
    pub(super) produttore: Option<String>,
    pub(super) modello: Option<String>,
    pub(super) ionizzazione: Option<String>,
    pub(super) analizzatore: Option<String>,
    pub(super) rivelatore: Option<String>,
    /// (name, version) of the programs that acquired and processed the data.
    pub(super) software: Vec<(String, String)>,
    /// (name, SHA-1) of the `parentFile` elements.
    pub(super) file: Vec<(String, String)>,
    pub(super) utente: Option<String>,
    /// The `centroided` of `dataProcessing`.
    pub(super) centroidi: Option<bool>,
}

fn mappa(chiave: &str) -> Option<&'static str> {
    match chiave {
        "msManufacturer" => Some("strumento.produttore"),
        "msModel" => Some("strumento.modello"),
        "msIonisation" => Some("ms.sorgente.tipo"),
        _ => None,
    }
}

/// Whether a file name is a path of the acquisition computer.
fn percorso(n: &str) -> bool {
    n.contains(['/', '\\'])
}

/// The last component of a path of either kind.
fn ultimo(n: &str) -> &str {
    n.rsplit(['/', '\\']).next().unwrap_or(n)
}

/// The fields of one element: `category`/`value` pairs (`msModel`…) are one field, any other attribute is one.
fn campi_di(o: &str, n: &str, g: Tag, dest: &mut Vec<Campo>) {
    let (cat, val) = (attr(g, "category"), attr(g, "value"));
    if !cat.is_empty() && !val.is_empty() {
        let f = Campo::new(o, cat, val);
        dest.push(match mappa(cat) {
            Some(m) => f.map(m),
            None => f,
        });
        return;
    }
    each(g, &mut |k, v| {
        let f = Campo::new(o, k, v);
        let identita = n == "operator" || (n == "parentFile" && k == "fileName" && percorso(v));
        dest.push(if identita { f.id() } else { f });
    });
}

fn annota(c: &mut Cabina, n: &str, g: Tag) {
    let v = |k: &str| Some(attr(g, k).trim().to_string()).filter(|v| !v.is_empty());
    match n {
        "msManufacturer" => c.produttore = v("value"),
        "msModel" => c.modello = v("value"),
        "msIonisation" => c.ionizzazione = v("value"),
        "msMassAnalyzer" => c.analizzatore = v("value"),
        "msDetector" => c.rivelatore = v("value"),
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
            c.utente = (!n.is_empty()).then(|| n.join(" "));
        }
        "dataProcessing" => c.centroidi = v("centroided").map(|x| x == "1" || x == "true"),
        _ => {}
    }
}

/// The cabin of `head`, the text of the file before its first scan.
pub(super) fn leggi(head: &str, limiti: &Limiti) -> Result<Cabina> {
    let mut c = Cabina::default();
    let mut pila: Vec<&str> = Vec::new();
    let mut errore = None;
    tags(head, &mut |g, _| {
        let n = nome(g);
        if n.starts_with('/') {
            pila.pop();
            return true;
        }
        if c.campi.len() < CAMPI_MAX && n != "mzXML" {
            let mut o = String::from("mzxml:");
            for p in &pila {
                o.push_str(p);
                o.push('/');
            }
            o.push_str(n);
            campi_di(&o, n, g, &mut c.campi);
        }
        annota(&mut c, n, g);
        if !g.ends_with('/') {
            if let Err(e) = limiti.profondita_ok(pila.len() + 1) {
                errore = Some(e);
                return false;
            }
            pila.push(n);
        }
        true
    });
    errore.map_or(Ok(c), Err)
}

/// The `metodo/1` document of the file `nome` (`dim` bytes, `sha1` when read).
pub(super) fn documento(c: &Cabina, nome: &str, dim: u64, sha1: Option<String>) -> String {
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
                ("produttore", s(&c.produttore)),
                ("modello", s(&c.modello)),
                ("software", Some(V::Arr(software))),
            ]),
        ),
        ("operatore", obj(vec![("utente", s(&c.utente))])),
        (
            "ms",
            obj(vec![("sorgente", obj(vec![("tipo", s(&c.ionizzazione))]))]),
        ),
        (
            "campi",
            Some(V::Arr(c.campi.iter().filter_map(Campo::json).collect())),
        ),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}

/// The header of the mzML: the instrument, the files the data come from (this file when the mzXML names none), the programs, and `doc` in the `<run>`.
pub(super) fn header(c: &Cabina, nome: &str, sha1: Option<String>, doc: String) -> Header {
    let mut h = Header {
        run: nome.rsplit_once('.').map_or(nome, |(s, _)| s).to_string(),
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        instrument: c.modello.clone().map(|m| ("MS:1000031".to_string(), m)),
        ..Header::default()
    };
    for (k, valore, acc, nome_cv) in [
        ("source", &c.ionizzazione, "MS:1000008", "ionization type"),
        (
            "analyzer",
            &c.analizzatore,
            "MS:1000443",
            "mass analyzer type",
        ),
        ("detector", &c.rivelatore, "MS:1000026", "detector type"),
    ] {
        if let Some(v) = valore {
            h.components
                .push((k.to_string(), acc.to_string(), format!("{nome_cv}: {v}")));
        }
    }
    for (n, sha) in &c.file {
        h.sources.push(ultimo(n).to_string());
        h.source_sha1.push(sha.clone());
    }
    if h.sources.is_empty() {
        h.sources.push(nome.to_string());
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
