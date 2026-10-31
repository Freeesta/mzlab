//! Walk: what a closing tag checks (`count`, required children, the index texts, a whole array, a whole spectrum or chromatogram).

use super::{Walk, REQUIRED};
use crate::mzmlvalida::array::Kind;
use crate::mzmlvalida::elementi::{self, push};
use crate::mzmlvalida::{Item, Severity};

impl<'a> Walk<'a> {
    pub(super) fn end(&mut self, name: &str) {
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
        let declared = arr
            .array_length
            .or(self.cur.as_ref().and_then(|c| c.declared))
            .unwrap_or(0);
        match arr.decode(&mut self.inf, declared) {
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
                .find(|t| {
                    crate::mzmlvalida::cv::get().is(t, crate::mzmlvalida::cv::Class::ChromType)
                })
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
