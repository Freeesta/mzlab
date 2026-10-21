//! Walk: the findings of the whole pass (`finish`) and the helpers every check uses.

use super::{Report, Walk};
use crate::mzmlarray::Inflate;
use crate::mzmlvalida::elementi::push;
use crate::mzmlvalida::indice::Idx;
use crate::mzmlvalida::{Item, Severity};
use std::collections::HashSet;

impl<'a> Walk<'a> {
    pub(super) fn new(sink: &'a mut dyn FnMut(Item)) -> Self {
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

    pub(super) fn path(&self) -> String {
        self.stack
            .iter()
            .fold(String::new(), |s, f| s + "/" + &f.label)
    }

    pub(super) fn err(&mut self, path: &str, code: &'static str, message: String) {
        push(&mut self.probs, Severity::Error, path, code, message);
    }

    pub(super) fn fatal(&mut self, message: String) {
        let path = self.path();
        self.err(&path, "xml", message);
    }

    pub(super) fn finish(mut self, stopped: bool) -> Report {
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
}
