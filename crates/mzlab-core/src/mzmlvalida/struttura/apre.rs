//! Walk: what an opening tag does (lists, spectra, arrays, texts to capture) and the values kept from the cvParams.

use super::{Cur, Frame, Walk};
use crate::mzmlvalida::array::Arr;
use crate::mzmlvalida::elementi::{self, Attrs};
use crate::mzmlvalida::{ChromInfo, SpecInfo};

impl<'a> Walk<'a> {
    pub(super) fn start(&mut self, name: &str, a: &Attrs, pos: u64) {
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
                let seconds = crate::mzml::is_seconds(a.get("unitName"), a.get("unitAccession"));
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
}
