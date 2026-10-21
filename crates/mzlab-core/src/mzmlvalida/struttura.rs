//! The pass over the file: a stack of the open elements (for the paths and the `count` of each list), the spectrum or chromatogram being read, and the
//! checks that need the whole element (arrays against `defaultArrayLength`, order of m/z, required terms).

use std::collections::HashSet;
use std::io::Read;

use quick_xml::events::Event;
use quick_xml::Reader;

use super::array::{Arr, Kind};
use super::elementi::{self, push, Attrs};
use super::indice::{Idx, Tee};
use super::{ChromInfo, Item, Problem, Report, Severity, SpecInfo};
use crate::error::Result;
use crate::mzmlarray::Inflate;

/// The pass stops after this many findings: a file that is wrong everywhere says so in the first thousand lines.
const MAX_PROBLEMS: usize = 1000;

/// Children each element must have (the schema's `minOccurs="1"` ones that a writer can forget).
const REQUIRED: [(&str, &[&str]); 4] = [
    (
        "mzML",
        &[
            "cvList",
            "fileDescription",
            "softwareList",
            "instrumentConfigurationList",
            "dataProcessingList",
            "run",
        ],
    ),
    ("indexedmzML", &["mzML"]),
    ("fileDescription", &["fileContent"]),
    ("binaryDataArray", &["binary"]),
];

struct Frame {
    name: String,
    /// `name`, or `name[index]` for a spectrum and a chromatogram.
    label: String,
    /// Children that count for `count` (everything but the parameters).
    kids: usize,
    declared: Option<usize>,
    seen: Vec<String>,
}

/// The spectrum or chromatogram being read.
#[derive(Default)]
struct Cur {
    chrom: bool,
    terms: Vec<String>,
    declared: Option<usize>,
    /// (kind, values, `arrayLength` of the array) of each array that decoded.
    arrays: Vec<(Kind, Vec<f64>, Option<usize>)>,
    failed: bool,
    spec: SpecInfo,
    chromatogram: ChromInfo,
}

struct Walk<'a> {
    probs: Vec<Problem>,
    stack: Vec<Frame>,
    cvs: HashSet<String>,
    ids: HashSet<String>,
    cur: Option<Cur>,
    arr: Option<Arr>,
    idx: Idx,
    inf: Inflate,
    text: String,
    cap: bool,
    spectra: usize,
    chroms: usize,
    root: Option<String>,
    sink: &'a mut dyn FnMut(Item),
}

pub(super) fn run<R: Read>(reader: R, sink: &mut dyn FnMut(Item)) -> Result<Report> {
    let mut xml = Reader::from_reader(Tee::new(reader));
    let mut buf = Vec::new();
    let mut w = Walk::new(sink);
    let mut stopped = false;
    loop {
        let pos = xml.buffer_position();
        let ev = match xml.read_event_into(&mut buf) {
            Ok(ev) => ev,
            Err(e) => {
                w.fatal(format!("{e} at byte {}", xml.buffer_position()));
                stopped = true;
                break;
            }
        };
        match ev {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let name = e.name().into_inner();
                if name == "fileChecksum" {
                    w.idx.computed = Some(xml.get_ref().hex());
                }
                w.start(name, &Attrs::of(e), pos);
                if matches!(ev, Event::Empty(_)) {
                    w.end(name);
                }
            }
            Event::End(ref e) => w.end(e.name().into_inner()),
            Event::Text(ref t) if w.cap => w.text.push_str(&t.xml10_content()),
            _ => {}
        }
        buf.clear();
        if w.probs.len() >= MAX_PROBLEMS {
            push(
                &mut w.probs,
                Severity::Warning,
                "",
                "limit",
                format!("stopped after {MAX_PROBLEMS} findings"),
            );
            stopped = true;
            break;
        }
    }
    Ok(w.finish(stopped))
}

impl<'a> Walk<'a> {
    fn new(sink: &'a mut dyn FnMut(Item)) -> Self {
        Walk {
            probs: Vec::new(),
            stack: Vec::new(),
            cvs: HashSet::new(),
            ids: HashSet::new(),
            cur: None,
            arr: None,
            idx: Idx::default(),
            inf: Inflate::new(),
            text: String::new(),
            cap: false,
            spectra: 0,
            chroms: 0,
            root: None,
            sink,
        }
    }

    fn path(&self) -> String {
        self.stack
            .iter()
            .fold(String::new(), |s, f| s + "/" + &f.label)
    }

    fn err(&mut self, path: &str, code: &'static str, message: String) {
        push(&mut self.probs, Severity::Error, path, code, message);
    }

    fn fatal(&mut self, message: String) {
        let path = self.path();
        self.err(&path, "xml", message);
    }

    fn finish(mut self, stopped: bool) -> Report {
        if !stopped {
            if !self.stack.is_empty() {
                self.fatal(format!(
                    "unexpected end of file with {} elements open",
                    self.stack.len()
                ));
            } else if self.root.is_none() {
                self.err("", "required", "no mzML or indexedmzML element".into());
            }
            let mut out = Vec::new();
            self.idx.check(&mut out);
            self.probs.append(&mut out);
        }
        Report {
            problems: self.probs,
            spectra: self.spectra,
            chromatograms: self.chroms,
            indexed: self.idx.present,
        }
    }

    fn start(&mut self, name: &str, a: &Attrs, pos: u64) {
        let is_param = matches!(name, "cvParam" | "userParam" | "referenceableParamGroupRef");
        let mut ordinal = 0;
        if let Some(p) = self.stack.last_mut() {
            if !is_param {
                p.kids += 1;
                ordinal = p.kids - 1;
            }
            p.seen.push(name.to_string());
        } else {
            self.root = Some(name.to_string());
            if !matches!(name, "mzML" | "indexedmzML") {
                self.err(
                    &format!("/{name}"),
                    "root",
                    format!("the root element is {name}, not mzML or indexedmzML"),
                );
            }
        }
        let label = match name {
            "spectrum" | "chromatogram" => format!("{name}[{ordinal}]"),
            _ => name.to_string(),
        };
        let parent = self.path();
        for r in elementi::required_attributes(name) {
            if a.get(r).is_none() {
                self.err(
                    &format!("{parent}/{label}"),
                    "attribute",
                    format!("{name} has no {r} attribute"),
                );
            }
        }
        if name == "cvParam" {
            elementi::cv_param(a, &self.cvs, &parent, &mut self.probs);
            self.param(a);
        }
        self.stack.push(Frame {
            name: name.into(),
            label,
            kids: 0,
            declared: a.num("count"),
            seen: Vec::new(),
        });
        self.open(name, a, pos, ordinal);
    }

    /// What an opening tag starts: a list, a spectrum, an array, a text to capture.
    fn open(&mut self, name: &str, a: &Attrs, pos: u64, ordinal: usize) {
        match name {
            "indexedmzML" => self.idx.present = true,
            "cv" => {
                self.cvs.insert(a.get("id").unwrap_or("").to_string());
            }
            "spectrum" | "chromatogram" => self.open_item(name == "chromatogram", a, pos, ordinal),
            "precursor" if self.cur.as_ref().is_some_and(|c| !c.chrom) => {
                if let Some(c) = self.cur.as_mut() {
                    c.spec.precursors.push((None, None));
                }
            }
            "binaryDataArray" => {
                self.arr = Some(Arr {
                    encoded_length: a.num("encodedLength"),
                    array_length: a.num("arrayLength"),
                    ..Default::default()
                });
            }
            "indexList" => self.idx.list_at = Some(pos),
            "index" => self.idx.list = a.get("name").unwrap_or("").to_string(),
            "offset" => {
                self.idx.id_ref = a.get("idRef").unwrap_or("").to_string();
                self.capture();
            }
            "binary" | "indexListOffset" | "fileChecksum" => self.capture(),
            _ => {}
        }
    }

    fn capture(&mut self) {
        self.text.clear();
        self.cap = true;
    }

    fn open_item(&mut self, chrom: bool, a: &Attrs, pos: u64, ordinal: usize) {
        let name = if chrom { "chromatogram" } else { "spectrum" };
        let path = self.path() + &format!("/{name}[{ordinal}]");
        let id = a.get("id").unwrap_or("").to_string();
        if a.num::<usize>("index").is_some_and(|i| i != ordinal) {
            self.err(
                &path,
                "index",
                format!(
                    "index is {} and the {name} is number {ordinal}",
                    a.get("index").unwrap_or("")
                ),
            );
        }
        if !self.ids.insert(format!("{name}:{id}")) {
            self.err(
                &path,
                "duplicateId",
                format!("two {name}s have the id \"{id}\""),
            );
        }
        if self.idx.present {
            let list = if chrom {
                &mut self.idx.seen_chroms
            } else {
                &mut self.idx.seen_spectra
            };
            list.push((id.clone(), pos));
        }
        let mut cur = Cur {
            chrom,
            declared: a.num("defaultArrayLength"),
            ..Default::default()
        };
        cur.spec = SpecInfo {
            index: ordinal,
            id: id.clone(),
            ..Default::default()
        };
        cur.chromatogram = ChromInfo {
            index: ordinal,
            id,
            ..Default::default()
        };
        self.cur = Some(cur);
    }

    /// The value of a cvParam that the pass keeps: terms of the open element, level, polarity, time, precursors.
    fn param(&mut self, a: &Attrs) {
        let acc = a.get("accession").unwrap_or("").to_string();
        let n = self.stack.len();
        let at = |k: usize| {
            n.checked_sub(k)
                .and_then(|i| self.stack.get(i))
                .map_or("", |f| f.name.as_str())
        };
        let (parent, grand) = (at(1).to_string(), at(2).to_string());
        let value = a.num::<f64>("value");
        if parent == "binaryDataArray" {
            if let Some(arr) = self.arr.as_mut() {
                arr.terms.push(acc);
            }
            return;
        }
        let Some(cur) = self.cur.as_mut() else { return };
        match (parent.as_str(), grand.as_str(), acc.as_str()) {
            ("spectrum" | "chromatogram", _, _) => {
                match acc.as_str() {
                    "MS:1000511" => cur.spec.level = value.map_or(0, |v| v as u8),
                    "MS:1000130" => cur.spec.polarity = 1,
                    "MS:1000129" => cur.spec.polarity = -1,
                    _ => {}
                }
                cur.terms.push(acc);
            }
            ("scan", _, "MS:1000016") => {
                let seconds = a.get("unitAccession") == Some("UO:0000010");
                cur.spec.rt = value.map(|v| if seconds { v / 60.0 } else { v });
            }
            ("selectedIon", _, "MS:1000744") => {
                if let Some(p) = cur.spec.precursors.last_mut() {
                    p.0 = value;
                }
            }
            ("selectedIon", _, "MS:1000041") => {
                if let Some(p) = cur.spec.precursors.last_mut() {
                    p.1 = value.map(|v| v as i32);
                }
            }
            ("isolationWindow", "precursor", "MS:1000827") if cur.chrom => {
                cur.chromatogram.q1 = value
            }
            ("isolationWindow", "product", "MS:1000827") if cur.chrom => {
                cur.chromatogram.q3 = value
            }
            _ => {}
        }
    }

    fn end(&mut self, name: &str) {
        let path = self.path();
        let Some(f) = self.stack.pop() else { return };
        if f.declared.is_some_and(|d| d != f.kids) {
            let d = f.declared.unwrap_or(0);
            self.err(
                &path,
                "count",
                format!("{name} count is {d} and it holds {}", f.kids),
            );
        }
        for (parent, needed) in REQUIRED {
            for r in needed
                .iter()
                .filter(|r| name == parent && !f.seen.iter().any(|s| s == **r))
            {
                self.err(&path, "required", format!("{name} has no {r}"));
            }
        }
        match name {
            "binary" => {
                self.cap = false;
                if let Some(a) = self.arr.as_mut() {
                    a.text = std::mem::take(&mut self.text);
                }
            }
            "offset" | "indexListOffset" | "fileChecksum" => self.close_index_text(name, &path),
            "binaryDataArray" => self.finish_array(&path),
            "spectrum" | "chromatogram" => self.finish_item(&path),
            _ => {}
        }
    }

    fn close_index_text(&mut self, name: &str, path: &str) {
        self.cap = false;
        let text = std::mem::take(&mut self.text);
        if name == "fileChecksum" {
            self.idx.checksum = Some(text);
            return;
        }
        let Ok(n) = text.trim().parse::<u64>() else {
            self.err(path, "index.offset", format!("{name} is not a number"));
            return;
        };
        match (name, self.idx.list.as_str()) {
            ("indexListOffset", _) => self.idx.list_offset = Some(n),
            (_, "spectrum") => self.idx.spectra.push((self.idx.id_ref.clone(), n)),
            (_, "chromatogram") => self.idx.chroms.push((self.idx.id_ref.clone(), n)),
            _ => {}
        }
    }

    fn finish_array(&mut self, path: &str) {
        let Some(arr) = self.arr.take() else { return };
        let mut probs = Vec::new();
        elementi::required_terms("binaryDataArray", &arr.terms, path, &mut probs);
        let have = arr.text.trim().len();
        if arr.encoded_length.is_some_and(|n| n != have) {
            push(
                &mut probs,
                Severity::Error,
                path,
                "encodedLength",
                format!(
                    "encodedLength is {} and the text has {have} characters",
                    arr.encoded_length.unwrap_or(0)
                ),
            );
        }
        match arr.decode(&mut self.inf) {
            Ok(v) => {
                if let Some(c) = self.cur.as_mut() {
                    c.arrays.push((arr.kind(), v, arr.array_length));
                }
            }
            Err(e) => {
                push(
                    &mut probs,
                    Severity::Error,
                    path,
                    "array.decode",
                    format!("the array does not decode: {e}"),
                );
                if let Some(c) = self.cur.as_mut() {
                    c.failed = true;
                }
            }
        }
        self.probs.append(&mut probs);
    }

    fn finish_item(&mut self, path: &str) {
        let Some(mut cur) = self.cur.take() else {
            return;
        };
        let element = if cur.chrom {
            "chromatogram"
        } else {
            "spectrum"
        };
        let mut probs = Vec::new();
        elementi::required_terms(element, &cur.terms, path, &mut probs);
        let wrong = cur
            .arrays
            .iter()
            .find(|(_, v, al)| al.or(cur.declared).is_some_and(|n| n != v.len()));
        if let (false, Some((_, v, al))) = (cur.failed, wrong) {
            let n = al.or(cur.declared).unwrap_or(0);
            push(
                &mut probs,
                Severity::Error,
                path,
                "arrayLength",
                format!(
                    "defaultArrayLength is {n} and an array has {} values",
                    v.len()
                ),
            );
        }
        let order = cur
            .arrays
            .iter()
            .find(|(k, v, _)| *k == Kind::Mz && v.windows(2).any(|w| w[1] < w[0]));
        if order.is_some() {
            push(
                &mut probs,
                Severity::Error,
                path,
                "mz.order",
                "the m/z array is not in ascending order".into(),
            );
        }
        self.probs.append(&mut probs);
        let mut take = |k: Kind| {
            cur.arrays
                .iter_mut()
                .find(|(kind, _, _)| *kind == k)
                .map(|(_, v, _)| std::mem::take(v))
                .unwrap_or_default()
        };
        if cur.chrom {
            cur.chromatogram.kind = cur
                .terms
                .iter()
                .find(|t| super::cv::get().is(t, super::cv::Class::ChromType))
                .cloned()
                .unwrap_or_default();
            cur.chromatogram.time = take(Kind::Time);
            cur.chromatogram.intensity = take(Kind::Intensity);
            self.chroms += 1;
            (self.sink)(Item::Chromatogram(cur.chromatogram));
        } else {
            cur.spec.mz = take(Kind::Mz);
            cur.spec.intensity = take(Kind::Intensity);
            self.spectra += 1;
            (self.sink)(Item::Spectrum(cur.spec));
        }
    }
}
