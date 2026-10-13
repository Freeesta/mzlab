//! `metodo/1`: the acquisition method of one sample (instrument, source, experiments, LC, detectors, log) as one JSON document, whatever file it comes from
//! (`.dam`, `.wiff`, the mzML that carries it). Spec: `docs/agenti/metodo.md`, schema `mzlab/web/schemi/metodo-1.json`. An unknown value is ABSENT (never null, never 0).
//! This file: the tree, the `.dam` adapter, the SHA-1 helpers and the `userParam` of the mzML; the `.wiff` adapter is `metodo_wiff.rs`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::methodinfo::{Kind, Lc, Method, Param};
use sha1::{Digest as _, Sha1};

pub const SCHEMA: &str = "metodo/1";
/// Name of the `userParam` of the mzML `<run>` that holds the document (`docs/agenti/metodo.md`).
pub const USERPARAM: &str = "mzLab metodo/1";
pub const READER: &str = "mzlab-core";

/// A JSON value of the document; objects keep the order they are built in.
#[derive(Debug, Clone, PartialEq)]
pub enum V {
    Num(f64),
    Bool(bool),
    Str(String),
    Arr(Vec<V>),
    Obj(Vec<(&'static str, V)>),
}

impl V {
    pub fn s(t: impl Into<String>) -> V {
        V::Str(t.into())
    }
    pub fn write(&self, o: &mut Out) {
        match self {
            V::Num(x) => drop(o.num(*x)),
            V::Bool(b) => drop(o.raw(if *b { "true" } else { "false" })),
            V::Str(t) => drop(o.str(t)),
            V::Arr(a) => {
                o.raw("[");
                for (i, v) in a.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" });
                    v.write(o);
                }
                o.raw("]");
            }
            V::Obj(f) => {
                o.raw("{");
                for (i, (k, v)) in f.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
                    v.write(o);
                }
                o.raw("}");
            }
        }
    }
    pub fn json(&self) -> String {
        let mut o = Out::default();
        self.write(&mut o);
        o.0
    }
    /// The value of a field of an object (tests, adapters).
    pub fn get(&self, k: &str) -> Option<&V> {
        match self {
            V::Obj(f) => f.iter().find(|(n, _)| *n == k).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// An object from `(name, value)` pairs, leaving out the absent ones and the empty lists/objects (an empty section does not exist).
pub fn obj(items: Vec<(&'static str, Option<V>)>) -> Option<V> {
    let f: Vec<(&'static str, V)> = items
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .filter(|(_, v)| {
            !matches!(v, V::Arr(a) if a.is_empty()) && !matches!(v, V::Obj(f) if f.is_empty())
        })
        .collect();
    (!f.is_empty()).then_some(V::Obj(f))
}

pub fn num(x: Option<f64>) -> Option<V> {
    x.filter(|v| v.is_finite()).map(V::Num)
}

pub fn text(t: &str) -> Option<V> {
    let t = t.trim();
    (!t.is_empty()).then(|| V::s(t))
}

/// One file the sample comes from: name, SHA-1 (when read) and size.
#[derive(Debug, Clone, Default)]
pub struct SrcFile {
    pub name: String,
    pub sha1: Option<String>,
    pub bytes: u64,
}

/// Where the document comes from (`origine`).
#[derive(Debug, Clone, Default)]
pub struct Source {
    pub format: &'static str,
    pub files: Vec<SrcFile>,
    pub sample: Option<u32>,
    /// `true`: leave out what identifies people (user, computer, serial number, paths).
    pub anonymous: bool,
    /// With `anonymous`: the secret of the pseudonyms (`pseudo`); without it the identity is left out.
    pub secret: Option<Vec<u8>>,
}

impl Source {
    /// The pseudonym of `value` as a `kind` when the document is anonymous and has a secret, else nothing.
    pub fn alias(&self, kind: &str, value: &str) -> Option<String> {
        crate::pseudo::alias(self.secret.as_deref(), kind, value)
    }
}

/// `origine`; `created` = when the source file was made, as the file says (local time of the instrument PC, no zone).
pub fn origin(src: &Source, created: Option<&str>) -> Option<V> {
    let files: Vec<V> = src
        .files
        .iter()
        .filter_map(|f| {
            obj(vec![
                ("nome", text(&f.name)),
                ("sha1", f.sha1.as_deref().and_then(text)),
                ("byte", (f.bytes > 0).then_some(V::Num(f.bytes as f64))),
            ])
        })
        .collect();
    obj(vec![
        ("formato", Some(V::s(src.format))),
        ("file", Some(V::Arr(files))),
        ("campione", src.sample.map(|n| V::Num(n as f64))),
        ("file_creato", created.and_then(text)),
        (
            "lettore",
            obj(vec![
                ("nome", Some(V::s(READER))),
                ("versione", Some(V::s(env!("CARGO_PKG_VERSION")))),
            ]),
        ),
    ])
}

/// Parameters sorted by id: the order the file stores them in is an accident of the writer (a `.dam` and the same method inside a `.wiff` differ).
fn params(p: &[Param]) -> V {
    let mut p: Vec<&Param> = p.iter().collect();
    p.sort_by(|a, b| a.id.cmp(&b.id));
    V::Arr(
        p.into_iter()
            .filter_map(|p| {
                obj(vec![
                    ("id", text(&p.id)),
                    ("etichetta", text(p.label)),
                    ("valore", num(Some(p.value))),
                    ("unita", text(p.unit)),
                ])
            })
            .collect(),
    )
}

/// The type only when the method data say it (a list of transitions); the name of a file is no evidence.
fn experiment_type(e: &crate::methodinfo::Experiment) -> Option<&'static str> {
    matches!(e.kind, Kind::Mrm(_)).then_some("MRM")
}

/// `ms`: the source and the experiments of a method (one period: the file does not say where a period ends). `source_type`: the kind of
/// source as the instrument reported it (the log of a `.wiff`: «Turbo Spray»); a method alone does not say, so it is absent then.
pub fn ms_section(m: &Method, source_type: Option<&str>) -> Option<V> {
    let exps: Vec<V> = m
        .experiments
        .iter()
        .filter_map(|e| {
            let (mz, transitions) = match &e.kind {
                Kind::Mrm(tr) => (
                    None,
                    V::Arr(
                        tr.iter()
                            .filter_map(|t| {
                                obj(vec![
                                    ("q1", num(Some(t.q1))),
                                    ("q3", num(Some(t.q3))),
                                    ("dwell_ms", num(Some(t.dwell))),
                                    ("ce_eV", num(t.ce)),
                                    ("nome", text(&t.name)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                Kind::Scan { range, .. } => (
                    range.map(|r| V::Arr(vec![V::Num(r[0]), V::Num(r[1])])),
                    V::Arr(vec![]),
                ),
            };
            let ce = match &e.kind {
                Kind::Scan { ce, .. } if !ce.is_empty() => {
                    Some(V::Arr(ce.iter().map(|&c| V::Num(c)).collect()))
                }
                _ => None,
            };
            obj(vec![
                ("indice", Some(V::Num(e.index as f64))),
                ("tipo", experiment_type(e).map(V::s)),
                ("mz", mz),
                ("ce_eV", ce),
                ("parametri", Some(params(&m.compound))),
                ("transizioni", Some(transitions)),
            ])
        })
        .collect();
    obj(vec![
        (
            "sorgente",
            obj(vec![
                ("tipo", source_type.and_then(text)),
                ("parametri", Some(params(&m.source))),
            ]),
        ),
        (
            "periodi",
            obj(vec![("esperimenti", Some(V::Arr(exps)))]).map(|p| V::Arr(vec![p])),
        ),
    ])
}

/// `lc` and `rivelatori` of a method (the LC method XML).
pub fn lc_sections(lc: &Lc) -> (Option<V>, Option<V>) {
    let grad: Vec<V> = lc
        .gradient
        .iter()
        .filter_map(|g| {
            obj(vec![
                ("t_min", num(Some(g.t))),
                ("B_pct", num(g.b)),
                ("flusso_mL_min", num(g.flow)),
            ])
        })
        .collect();
    let lc_v = obj(vec![
        ("gradiente", Some(V::Arr(grad))),
        ("flusso_mL_min", num(lc.flow)),
        ("B_iniziale_pct", num(lc.b0)),
        ("forno_C", num(lc.oven)),
        ("durata_min", num(lc.run_time)),
    ]);
    let p = &lc.pda;
    let channels: Vec<V> = lc
        .channels
        .iter()
        .filter_map(|(w, bw)| obj(vec![("nm", num(Some(*w))), ("banda_nm", num(*bw))]))
        .collect();
    let nm = match (p.start, p.stop) {
        (Some(a), Some(b)) => Some(V::Arr(vec![V::Num(a), V::Num(b)])),
        _ => None,
    };
    let det = obj(vec![
        ("tipo", Some(V::s("PDA"))),
        ("nm", nm),
        ("passo_nm", num(p.step)),
        ("frequenza_Hz", num(p.freq)),
        ("lampada", text(&p.lamp)),
        ("cella_C", num(p.cell)),
        ("fenditura_nm", num(p.slit)),
        ("canali", Some(V::Arr(channels))),
    ]);
    // a detector with only its type is no information: it exists when the method says something about it
    let det = det.filter(|d| matches!(d, V::Obj(f) if f.len() > 1));
    (lc_v, det.map(|d| V::Arr(vec![d])))
}

/// The document of a method read from a `.dam` (or from a method of a `.wiff`, without the rest of the sample).
pub fn from_dam(m: &Method, src: &Source) -> String {
    let (lc, det) = m.lc.as_ref().map_or((None, None), lc_sections);
    let file = src.files.first().map(|f| f.name.clone());
    let doc = obj(vec![
        ("schema", Some(V::s(SCHEMA))),
        ("origine", origin(src, None)),
        (
            "campione",
            obj(vec![("metodo_file", file.as_deref().and_then(text))]),
        ),
        ("ms", ms_section(m, None)),
        ("lc", lc),
        ("rivelatori", det),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}

/// SHA-1 (lower-case hex) of the first `size` bytes of `r`, read in blocks of 1 MB.
pub fn sha1_hex<R: ReadAt>(r: &mut R, size: u64) -> Result<String> {
    let mut h = Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut at = 0u64;
    while at < size {
        let n = (size - at).min(buf.len() as u64) as usize;
        r.read_at(at, &mut buf[..n])?;
        h.update(&buf[..n]);
        at += n as u64;
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `(file name, SHA-1)` of every `<sourceFile>` of an mzML header (as ProteoWizard writes it: the SHA-1 is the cvParam MS:1000569).
pub fn source_sha1s(head: &str) -> Vec<(String, String)> {
    let mut out = vec![];
    let mut rest = head;
    while let Some(p) = rest.find("<sourceFile ") {
        let t = &rest[p..];
        let end = t.find("</sourceFile>").unwrap_or(t.len());
        let block = &t[..end];
        let attr = |k: &str| {
            block
                .find(k)
                .and_then(|a| block[a + k.len()..].split('"').next())
                .map(String::from)
        };
        if let (Some(name), Some(a)) = (attr(" name=\""), block.find("MS:1000569")) {
            if let Some(v) = block[a..]
                .find("value=\"")
                .and_then(|b| block[a + b + 7..].split('"').next())
            {
                out.push((name, v.to_ascii_lowercase()));
            }
        }
        rest = &t[end..];
    }
    out
}

/// `sample=N` of the nativeID of an mzML id (ProteoWizard's SCIEX format: `sample=1 period=1 cycle=3 experiment=1`, or `SRM SIC … sample=1 …`).
pub fn sample_of_id(id: &str) -> Option<u32> {
    let p = id.find("sample=")?;
    id[p + 7..]
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
        .replace('\t', "&#9;")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&#9;", "\t")
        .replace("&#13;", "\r")
        .replace("&#10;", "\n")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// The `<userParam>` that carries the document in the `<run>` of an mzML.
pub fn userparam_xml(doc: &str) -> String {
    format!(
        "<userParam name=\"{USERPARAM}\" type=\"xsd:string\" value=\"{}\"/>",
        xml_escape(doc)
    )
}

/// The document an mzML carries (`None` when its header has none): what `userparam_xml` wrote.
pub fn from_userparam(head: &str) -> Option<String> {
    let p = head.find(&format!("name=\"{USERPARAM}\""))?;
    let t = &head[p..head[p..].find("/>").map_or(head.len(), |e| p + e)];
    let v = &t[t.find("value=\"")? + 7..];
    Some(xml_unescape(&v[..v.rfind('"')?]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sections_do_not_exist_and_unknown_is_absent() {
        let v = obj(vec![
            ("a", None),
            ("b", Some(V::Arr(vec![]))),
            ("c", num(Some(f64::NAN))),
        ]);
        assert_eq!(v, None);
        let v = obj(vec![
            ("a", Some(V::s("x"))),
            ("b", num(None)),
            ("c", num(Some(2.5))),
        ])
        .unwrap();
        assert_eq!(v.json(), "{\"a\":\"x\",\"c\":2.5}");
    }

    #[test]
    fn userparam_round_trips_any_text() {
        let doc = "{\"schema\":\"metodo/1\",\"x\":\"a \\\"q\\\" <b> & c\\nline\"}";
        let x = format!("<run id=\"r\">{}</run>", userparam_xml(doc));
        assert!(!x.contains("<b>"));
        assert_eq!(from_userparam(&x).as_deref(), Some(doc));
        assert_eq!(from_userparam("<run/>"), None);
    }

    #[test]
    fn sha1_of_a_buffer_and_the_source_files_of_a_header() {
        let mut data = b"abc".to_vec();
        assert_eq!(
            sha1_hex(&mut data, 3).unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        let head = "<fileDescription><sourceFileList count=\"2\"><sourceFile id=\"WIFF\" name=\"B_MRM.wiff\" location=\"file://x\">\n<cvParam cvRef=\"MS\" accession=\"MS:1000562\" name=\"ABI WIFF format\" value=\"\"/>\n<cvParam cvRef=\"MS\" accession=\"MS:1000569\" name=\"SHA-1\" value=\"EABDE1F5F7417181A0790BA25F8D0888530E7261\"/></sourceFile><sourceFile id=\"S\" name=\"B_MRM.wiff.scan\"><cvParam accession=\"MS:1000569\" name=\"SHA-1\" value=\"a047\"/></sourceFile></sourceFileList>";
        let s = source_sha1s(head);
        assert_eq!(
            s,
            vec![
                (
                    "B_MRM.wiff".to_string(),
                    "eabde1f5f7417181a0790ba25f8d0888530e7261".to_string()
                ),
                ("B_MRM.wiff.scan".to_string(), "a047".to_string())
            ]
        );
        assert_eq!(
            sample_of_id("SRM SIC Q1=1 Q3=2 sample=13 period=1"),
            Some(13)
        );
        assert_eq!(sample_of_id("TIC"), None);
    }
}
