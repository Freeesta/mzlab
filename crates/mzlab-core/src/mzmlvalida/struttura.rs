//! The pass over the file: a stack of the open elements (for the paths and the `count` of each list), the spectrum or chromatogram being read, and the
//! checks that need the whole element (arrays against `defaultArrayLength`, order of m/z, required terms).

use std::collections::HashSet;
use std::io::Read;

use quick_xml::events::Event;
use quick_xml::Reader;

use super::array::{Arr, Kind};
use super::elementi::{push, Attrs};
use super::indice::{Idx, Tee};
use super::{ChromInfo, Item, Problem, Report, Severity, SpecInfo};
use crate::error::Result;
use crate::mzmlarray::Inflate;

mod apre;
mod base;
mod chiude;

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
