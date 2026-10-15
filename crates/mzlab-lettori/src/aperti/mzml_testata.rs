//! The scanner of tags and the readers of the header and of the parameters of an mzML for `mzml_numpress.rs` (the header of the original file, the cvParams of a spectrum).

use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};

/// The most cvParams of a spectrum kept as header fields.
const CAMPI_MAX: usize = 64;

// ---- a tiny scanner of tags: the header of an mzML is cut before `<run` (and so not a document), and the module may not take a new XML crate ----

pub(super) struct Tag {
    pub(super) nome: String,
    pub(super) attr: Vec<(String, String)>,
    pub(super) chiusura: bool,
    pub(super) vuoto: bool,
}

impl Tag {
    pub(super) fn get(&self, n: &str) -> &str {
        self.attr
            .iter()
            .find(|(k, _)| k == n)
            .map_or("", |(_, v)| v.as_str())
    }
}

pub(super) fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&#9;", "\t")
        .replace("&#13;", "\r")
        .replace("&#10;", "\n")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The tags of `s` in order (declarations, comments and text are skipped). A tag ends at the first `>` outside quotes.
pub(super) fn tag_di(s: &str) -> Vec<Tag> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s.get(i..).and_then(|r| r.find('<')) {
        let a = i + p + 1;
        let mut q = a;
        let mut citato = 0u8;
        while let Some(&c) = b.get(q) {
            match (citato, c) {
                (0, b'"') | (0, b'\'') => citato = c,
                (x, y) if x != 0 && x == y => citato = 0,
                (0, b'>') => break,
                _ => {}
            }
            q += 1;
        }
        let Some(corpo) = s.get(a..q) else { break };
        i = q + 1;
        if corpo.starts_with(['?', '!']) {
            continue;
        }
        let (chiusura, corpo) = match corpo.strip_prefix('/') {
            Some(r) => (true, r),
            None => (false, corpo),
        };
        let (vuoto, corpo) = match corpo.strip_suffix('/') {
            Some(r) => (true, r),
            None => (false, corpo),
        };
        let mut parti = corpo.splitn(2, |c: char| c.is_whitespace());
        let nome = parti.next().unwrap_or("").to_string();
        let mut attr = Vec::new();
        let mut resto = parti.next().unwrap_or("");
        while let Some(e) = resto.find('=') {
            let chiave = resto[..e].trim().to_string();
            let v = resto[e + 1..].trim_start();
            let Some(virgolette) = v.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                break;
            };
            let Some(fine) = v[1..].find(virgolette) else {
                break;
            };
            attr.push((chiave, unescape(&v[1..1 + fine])));
            resto = &v[fine + 2..];
        }
        out.push(Tag {
            nome,
            attr,
            chiusura,
            vuoto,
        });
    }
    out
}

/// The cvParams of a spectrum header (everything before `<binaryDataArrayList`) and what the writer cannot say with its own fields.
pub(super) struct Parametro {
    pub(super) acc: String,
    pub(super) nome: String,
    pub(super) valore: String,
    pub(super) unita: Option<String>,
}

/// The cvParams that `mzmlwrite` writes itself, from the fields of the `Spectrum`.
const SCRITTI: [&str; 20] = [
    "MS:1000511",
    "MS:1000579",
    "MS:1000580",
    "MS:1000130",
    "MS:1000129",
    "MS:1000128",
    "MS:1000127",
    "MS:1000504",
    "MS:1000505",
    "MS:1000285",
    "MS:1000582",
    "MS:1000528",
    "MS:1000527",
    "MS:1000512",
    "MS:1000016",
    "MS:1000795",
    "MS:1000501",
    "MS:1000500",
    "MS:1000827",
    "MS:1000744",
];

pub(super) fn parametri(spettro: &[u8]) -> Vec<Parametro> {
    let t = String::from_utf8_lossy(spettro);
    let fine = t.find("<binaryDataArrayList").unwrap_or(t.len());
    tag_di(&t[..fine])
        .into_iter()
        .filter(|g| !g.chiusura && g.nome == "cvParam")
        .take(CAMPI_MAX)
        .map(|g| Parametro {
            acc: g.get("accession").to_string(),
            nome: g.get("name").to_string(),
            valore: g.get("value").to_string(),
            unita: Some(g.get("unitName").to_string()).filter(|u| !u.is_empty()),
        })
        .collect()
}

/// What goes into the converted spectrum besides the fields of `Spectrum`: the scan window and, as userParams, every other cvParam with a value (collision energy, isolation width, injection time…).
pub(super) fn extra_di(spettro: &[u8]) -> SpectrumExtra {
    let mut e = SpectrumExtra::default();
    let (mut lo, mut hi) = (None, None);
    for p in parametri(spettro) {
        let numero = p.valore.parse::<f64>().ok();
        match p.acc.as_str() {
            "MS:1000501" => lo = numero,
            "MS:1000500" => hi = numero,
            a if SCRITTI.contains(&a) => {}
            _ if p.valore.is_empty() => {}
            a => e.params.push((format!("{a} {}", p.nome), p.valore)),
        }
    }
    e.window = lo.zip(hi);
    e
}

// ---- the header of the original mzML ----

/// A `referenceableParamGroup` or a component of the instrument: its id and its (accession, name, value) triples.
type Gruppo = (String, Vec<(String, String, String)>);

/// Where the walk over the header stands.
#[derive(Default)]
struct Stato {
    gruppi: Vec<Gruppo>,
    ic_visti: usize,
    componente_con_cv: bool,
    /// The `software` being read: its version and its first term.
    soft: Option<(String, Option<(String, String)>)>,
}

/// A `cvParam` of the header, by the element that holds it.
fn cv_param(h: &mut Header, st: &mut Stato, genitore: &str, g: &Tag) {
    let (a, n, v) = (g.get("accession"), g.get("name"), g.get("value"));
    match genitore {
        "fileContent" if a != "MS:1000579" => h.content.push((a.into(), n.into())),
        "sourceFile" if a == "MS:1000569" => h.source_sha1.push(v.into()),
        "sourceFile" => {
            if h.sources.len() == 1 && !h.source_cv.iter().any(|(x, _)| x == a) {
                h.source_cv.push((a.into(), n.into()));
            }
        }
        "referenceableParamGroup" => {
            if let Some((_, cvs)) = st.gruppi.last_mut() {
                cvs.push((a.into(), n.into(), v.into()));
            }
        }
        "instrumentConfiguration" if st.ic_visti == 1 => modello(h, a, n, v),
        "source" | "analyzer" | "detector" if st.ic_visti == 1 && !st.componente_con_cv => {
            st.componente_con_cv = true;
            h.components
                .push((genitore.to_string(), a.into(), n.into()));
        }
        "software" => {
            if let Some((_, c @ None)) = st.soft.as_mut() {
                *c = Some((a.into(), n.into()));
            }
        }
        _ => {}
    }
}

/// The header of the mzML we write: the instrument, the sources with their SHA-1, the software and the run of the original, and mzLab as the program that wrote this file.
pub(super) fn testata_da(head: &str, nome: &str, limiti: &Limiti) -> Result<Header> {
    let mut h = Header {
        run: nome.rsplit_once('.').map_or(nome, |(s, _)| s).to_string(),
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    };
    let mut pila: Vec<String> = Vec::new();
    let mut st = Stato::default();
    for g in tag_di(head) {
        if g.chiusura {
            if pila.pop().as_deref() == Some("software") {
                if let Some((v, Some((a, nm)))) = st.soft.take() {
                    h.extra_software.push((a, nm, v));
                }
                st.soft = None;
            }
            continue;
        }
        let genitore = pila.last().cloned().unwrap_or_default();
        match g.nome.as_str() {
            "referenceableParamGroup" => st.gruppi.push((g.get("id").to_string(), Vec::new())),
            "instrumentConfiguration" => st.ic_visti += 1,
            "sourceFile" => h.sources.push(g.get("name").to_string()),
            "sample" if h.sample.is_none() => {
                h.sample = Some((g.get("id").to_string(), g.get("name").to_string()))
            }
            "software" => st.soft = Some((g.get("version").to_string(), None)),
            "run" => {
                h.run = g.get("id").to_string();
                h.acquired = Some(g.get("startTimeStamp").to_string()).filter(|t| !t.is_empty());
            }
            "source" | "analyzer" | "detector" if st.ic_visti == 1 => st.componente_con_cv = false,
            "referenceableParamGroupRef"
                if st.ic_visti == 1 && genitore == "instrumentConfiguration" =>
            {
                let r = g.get("ref");
                if let Some((_, cvs)) = st.gruppi.iter().find(|(id, _)| id == r) {
                    for (a, n, v) in cvs {
                        modello(&mut h, a, n, v);
                    }
                }
            }
            "userParam" if genitore == "run" => {
                h.run_params
                    .push((g.get("name").to_string(), g.get("value").to_string()));
            }
            "cvParam" => cv_param(&mut h, &mut st, &genitore, &g),
            _ => {}
        }
        if !g.vuoto {
            limiti.profondita_ok(pila.len() + 1)?;
            pila.push(g.nome);
        }
    }
    Ok(h)
}

/// A cvParam of the instrument configuration: the serial number, else the first term is the model.
fn modello(h: &mut Header, acc: &str, nome: &str, valore: &str) {
    if acc == "MS:1000529" {
        h.serial = Some(valore.to_string()).filter(|s| !s.is_empty());
    } else if h.instrument.is_none() {
        h.instrument = Some((acc.to_string(), nome.to_string()));
    }
}
