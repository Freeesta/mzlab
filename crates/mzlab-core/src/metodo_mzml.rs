//! `metodo/1` of an mzML that mzLab did not write (`from_mzml_head`): what the header of the file says (instrument, serial, software, source files
//! with SHA-1, `run@startTimeStamp`, sample, scan settings, data processing) and the scan parameters that are the same in the first scans.
//! Only the start of the file is read: the header up to `<spectrumList` and at most `SCANS` spectra. Every field carries `origine = "mzml:<element>"`.
//! Spec: `docs/agenti/metodo.md`. The reference is the mzML 1.1 schema and the PSI-MS CV; nothing is copied from another program.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::metodo::{obj, origin, text, xml_unescape, Source, SrcFile, SCHEMA, V};
use crate::metodo_campi::Campo;
use crate::mzmlhead::{Metadata, Mzml};

/// The most spectra read for the parameters that are constant in the whole file.
const SCANS: usize = 64;
/// The most bytes read from the start of the file (header plus the sampled spectra).
const READ_MAX: u64 = 8 << 20;

/// Scan parameters worth a line in `ms`: (accession, id, label, unit).
const SCAN_PARAMS: [(&str, &str, &str, &str); 8] = [
    (
        "MS:1000927",
        "ion_injection_time",
        "ion injection time",
        "ms",
    ),
    ("MS:1000800", "resolving_power", "mass resolving power", ""),
    ("MS:1000045", "collision_energy", "collision energy", "eV"),
    (
        "MS:1000827",
        "isolation_target",
        "isolation window target m/z",
        "m/z",
    ),
    (
        "MS:1000828",
        "isolation_lower",
        "isolation window lower offset",
        "m/z",
    ),
    (
        "MS:1000829",
        "isolation_upper",
        "isolation window upper offset",
        "m/z",
    ),
    ("MS:1000501", "scan_lower", "scan window lower limit", "m/z"),
    ("MS:1000500", "scan_upper", "scan window upper limit", "m/z"),
];

/// Value of attribute `k` in the opening tag `t`.
fn attr(t: &str, k: &str) -> Option<String> {
    let key = format!(" {k}=\"");
    let p = t.find(&key)? + key.len();
    t.get(p..)?.split('"').next().map(xml_unescape)
}

/// The opening tags `<name ...>` of a block, as text.
fn tags<'a>(s: &'a str, name: &str) -> Vec<&'a str> {
    let open = format!("<{name} ");
    s.match_indices(&open)
        .filter_map(|(i, _)| {
            s.get(i..)
                .and_then(|t| t.find('>').and_then(|e| t.get(..e)))
        })
        .collect()
}

/// The text of the first `<name ...>...</name>` (or `<name .../>`) of `s`.
fn block<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let a = s.find(&format!("<{name} "))?;
    let t = s.get(a..)?;
    let close = format!("</{name}>");
    let end = t.find(&close).map_or_else(
        || t.find("/>").map_or(t.len(), |e| e + 2),
        |e| e + close.len(),
    );
    t.get(..end)
}

/// The text of every `<name ...>...</name>` of `s`.
fn blocks<'a>(s: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = vec![];
    let mut rest = s;
    while let Some(b) = block(rest, name) {
        out.push(b);
        let at = rest.find(b).map_or(rest.len(), |p| p + b.len());
        rest = rest.get(at.max(1)..).unwrap_or("");
    }
    out
}

/// `(accession, name, value)` of every `cvParam`/`userParam` of a block.
fn params(s: &str) -> Vec<(String, String, String)> {
    ["cvParam", "userParam"]
        .iter()
        .flat_map(|k| tags(s, k))
        .map(|t| {
            (
                attr(t, "accession").unwrap_or_default(),
                attr(t, "name").unwrap_or_default(),
                attr(t, "value").unwrap_or_default(),
            )
        })
        .collect()
}

fn software(meta: &Metadata) -> Vec<V> {
    meta.software
        .iter()
        .filter_map(|s| {
            let (n, v) = s.rsplit_once(' ').unwrap_or((s.as_str(), ""));
            obj(vec![("nome", text(n)), ("versione", text(v))])
        })
        .collect()
}

/// `campi` from the header: the scan settings, the sample, the processing steps and the instrument components, each with its own element as origin.
fn header_fields(head: &str) -> Vec<Campo> {
    let mut out = vec![];
    let mut add = |origin: &str, b: &str| {
        for (acc, name, value) in params(b) {
            let key = if name.is_empty() { acc } else { name };
            out.push(Campo::new(origin, &key, value));
        }
    };
    for b in blocks(head, "scanSettings") {
        add("mzml:scanSettings", b);
    }
    for b in blocks(head, "sample") {
        add("mzml:sample", b);
    }
    for b in blocks(head, "processingMethod") {
        add("mzml:dataProcessing", b);
    }
    for b in blocks(head, "target") {
        add("mzml:target", b);
    }
    if let Some(t) = head
        .find("<run ")
        .and_then(|p| head.get(p..))
        .and_then(|t| t.split('>').next())
    {
        for k in ["defaultInstrumentConfigurationRef", "sampleRef", "id"] {
            if let Some(v) = attr(t, k) {
                out.push(Campo::new("mzml:run", k, v));
            }
        }
    }
    out
}

/// The parameters of the sampled scans: for each MS level, the ones that never change go to `ms` and the ones that change become an interval in `campi`.
fn scan_part(scans: &str, fields: &mut Vec<Campo>) -> Option<V> {
    let mut levels: Vec<(u32, Vec<Vec<f64>>)> = vec![];
    for sp in blocks(scans, "spectrum") {
        let p = params(sp);
        let get = |acc: &str| p.iter().find(|x| x.0 == acc).map(|x| x.2.as_str());
        let Some(level) = get("MS:1000511").and_then(|v| v.parse::<u32>().ok()) else {
            continue;
        };
        let at = levels.iter().position(|l| l.0 == level).unwrap_or_else(|| {
            levels.push((level, vec![vec![]; SCAN_PARAMS.len()]));
            levels.len() - 1
        });
        for (i, pr) in SCAN_PARAMS.iter().enumerate() {
            if let (Some(v), Some(l)) = (
                get(pr.0).and_then(|v| v.parse::<f64>().ok()),
                levels.get_mut(at),
            ) {
                if let Some(slot) = l.1.get_mut(i) {
                    slot.push(v);
                }
            }
        }
    }
    crate::sortc::slice_by(&mut levels, |a, b| a.0.cmp(&b.0));
    let exps: Vec<V> = levels
        .iter()
        .filter_map(|(level, vals)| {
            let mut ps = vec![];
            for (pr, v) in SCAN_PARAMS.iter().zip(vals) {
                let (Some(lo), Some(hi)) = (
                    v.iter().copied().reduce(f64::min),
                    v.iter().copied().reduce(f64::max),
                ) else {
                    continue;
                };
                if lo == hi {
                    ps.push(obj(vec![
                        ("id", text(pr.1)),
                        ("etichetta", text(pr.2)),
                        ("valore", Some(V::Num(lo))),
                        ("unita", text(pr.3)),
                    ]));
                } else {
                    let key = format!("MS{level} {}", pr.2);
                    fields
                        .push(Campo::new("mzml:spectrum", &key, format!("{lo}..{hi}")).unit(pr.3));
                }
            }
            obj(vec![
                ("indice", Some(V::Num(f64::from(*level)))),
                (
                    "parametri",
                    Some(V::Arr(ps.into_iter().flatten().collect())),
                ),
            ])
        })
        .collect();
    obj(vec![(
        "periodi",
        obj(vec![("esperimenti", Some(V::Arr(exps)))]).map(|p| V::Arr(vec![p])),
    )])
}

/// The `metodo/1` document an mzML header describes. `head` = the file up to `<spectrumList`; `scans` = the first spectra (their text);
/// `name`/`bytes` = the file itself; `meta` = `Mzml::metadata`.
pub fn from_mzml_head(head: &str, scans: &str, name: &str, bytes: u64, meta: &Metadata) -> String {
    let mut files = vec![SrcFile {
        name: name.into(),
        sha1: None,
        bytes,
    }];
    for (n, sha) in crate::metodo::source_sha1s(head) {
        files.push(SrcFile {
            name: n,
            sha1: Some(sha),
            bytes: 0,
        });
    }
    let src = Source {
        format: "mzml",
        files,
        sample: None,
    };
    let created = head
        .find("<run ")
        .and_then(|p| head.get(p..))
        .and_then(|t| t.split('>').next())
        .and_then(|t| attr(t, "startTimeStamp"));
    let sample = blocks(head, "sample")
        .first()
        .and_then(|b| tags(b, "sample").first().copied().map(String::from));
    let sample_name = sample.as_deref().and_then(|t| attr(t, "name"));
    let mut fields = header_fields(head);
    let ms = scan_part(scans, &mut fields);
    crate::sortc::slice_by(&mut fields, |a, b| {
        (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
    });
    let doc = obj(vec![
        ("schema", Some(V::s(SCHEMA))),
        ("origine", origin(&src, created.as_deref())),
        (
            "campione",
            obj(vec![("nome", sample_name.as_deref().and_then(text))]),
        ),
        (
            "strumento",
            obj(vec![
                ("modello", text(&meta.instrument)),
                ("serie", text(&meta.serial)),
                ("software", Some(V::Arr(software(meta)))),
            ]),
        ),
        ("ms", ms),
        (
            "campi",
            Some(V::Arr(fields.iter().filter_map(Campo::json).collect())),
        ),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}

impl<R: ReadAt> Mzml<R> {
    /// The `metodo/1` of an mzML: the document it carries (`userParam mzLab metodo/1`) or, when it has none, what its header and first scans say.
    /// `name` is the name of the file; `None` when the file has no header to read.
    pub fn metodo(&mut self, name: &str, doc_max: u64) -> Result<Option<String>> {
        let run = self.run_head(doc_max)?;
        if let Some(doc) = crate::metodo::from_userparam(&run) {
            return Ok(Some(doc));
        }
        let mut want = (256u64 << 10).min(self.size);
        let probe = loop {
            let p = self.read(0, want)?;
            let ready =
                p.windows(13).any(|w| w == b"<spectrumList") && count(&p, b"</spectrum>") >= SCANS;
            if ready || want >= self.size.min(READ_MAX) {
                break p;
            }
            want = (want * 4).min(self.size.min(READ_MAX));
        };
        let all = String::from_utf8_lossy(&probe).into_owned();
        let cut = ["<spectrumList", "<chromatogramList"]
            .iter()
            .filter_map(|k| all.find(k))
            .min()
            .unwrap_or(all.len());
        let head = all.get(..cut).unwrap_or("");
        if !head.contains("<run ") {
            return Ok(None);
        }
        let scans = sampled(all.get(cut..).unwrap_or(""));
        let meta = self.metadata()?;
        Ok(Some(from_mzml_head(head, &scans, name, self.size, &meta)))
    }
}

fn count(p: &[u8], n: &[u8]) -> usize {
    p.windows(n.len()).filter(|w| *w == n).count()
}

/// The first complete spectra of the text, without their binary data.
fn sampled(s: &str) -> String {
    blocks(s, "spectrum")
        .into_iter()
        .filter(|b| b.ends_with("</spectrum>"))
        .take(SCANS)
        .map(|b| {
            format!(
                "{}</spectrum>",
                b.find("<binaryDataArrayList")
                    .and_then(|c| b.get(..c))
                    .unwrap_or(b)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
