//! The samples of a dataset as the site and the app list them (`samples_json`): each with why it has nothing to show, so that a sample the reader
//! refuses carries its reason and never looks like an injection that acquired nothing (H6-15). `list_samples` is the superset `samples_list/1`
//! that a chooser needs (kinds, polarity, times, TIC, method, sizes), from the index alone.

use super::Reader;
use crate::json::Out;

/// What a reader says of one sample from its index, without decoding a scan. Every field is absent when the reader does not know it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Summary {
    /// `ms`, `ms2`, `sim`, `mrm`, `dad`, in that order.
    pub kinds: Vec<&'static str>,
    /// The scan types the sample's experiments acquire as readable labels (`Full scan`, `SIM`, `MRM`, `Product ion`, ..., `DAD`), each once, in the order of the
    /// experiments; the text the chooser and `sample list` show.
    pub types: Vec<&'static str>,
    /// `+`, `-` or `both`.
    pub polarity: Option<&'static str>,
    /// First and last retention time (minutes).
    pub rt: Option<(f64, f64)>,
    /// Retention time and height of the highest TIC row, and the sum of all rows.
    pub tic: Option<(f64, f64, f64)>,
    pub method: Option<String>,
    /// Scan rows the index places beyond the end of the data file (a copy cut short).
    pub missing: usize,
    /// The index has no scan, no chromatogram block and no optical data: an injection that acquired nothing.
    pub empty: Option<bool>,
}

/// What `Reader::listing` gives: one `Summary` per sample (empty for a reader that gives none) and, for a pair, the bytes of the files.
#[derive(Debug, Clone, Default)]
pub struct Listing {
    /// Bytes of the whole dataset.
    pub bytes: u64,
    /// `(bytes of the data file, where its indexed part ends)`.
    pub scan: Option<(u64, u64)>,
    pub samples: Vec<Summary>,
    /// `(sample id, start, end)` of each sample's section in the data file, ascending by start (`start == end` for a sample with no section): what a range copy
    /// (`readers::ranges`) keeps of the data file. Empty for a reader whose samples are not byte sections of one file.
    pub sections: Vec<(u32, u64, u64)>,
}

/// The listing of the engine (`samples_json`, kept byte for byte for its callers): the page and the app read it.
pub fn samples_json(l: &mut dyn Reader) -> String {
    write(l, false)
}

/// The superset `samples_list/1`: `samples_json` plus `schema`, `bytes`, `scan` and, per sample, `kinds`, `types`, `polarity`, `rt`, `tic`, `method`, `missing`.
/// A reader that opened the dataset without reading its data file (`readers::open_listing`) lists from the index alone.
pub fn list_samples(l: &mut dyn Reader) -> String {
    write(l, true)
}

/// The opening of the document: `{"format":..` as the page has always had it, or `samples_list/1` with its schema, `bytes` and `scan`.
fn head(o: &mut Out, l: &dyn Reader, rich: bool, facts: &Listing) {
    if rich {
        o.raw("{\"schema\":\"samples_list/1\",\"format\":");
    } else {
        o.raw("{\"format\":");
    }
    o.str(&format!("{:?}", l.format()));
    if rich {
        o.raw(",\"bytes\":").int(facts.bytes as i64);
        if let Some((total, end)) = facts.scan {
            o.raw(",\"scan\":{\"bytes\":")
                .int(total as i64)
                .raw(",\"indexed_end\":")
                .int(end as i64)
                .raw("}");
        }
    }
}

/// Why sample `i` has nothing to show (`refused`, `inferred`, `empty`), or `None`. `(found, n)`: the samples the index lists and all of them; `sum` its summary.
fn why_of(
    l: &mut dyn Reader,
    i: usize,
    (found, n): (usize, usize),
    sum: &Summary,
) -> Option<&'static str> {
    let refused = l.refused(i).is_some();
    // a reader that listed without reading its data file says it itself: its chromatograms would be read from that file
    let none = !refused
        && l.samples()[i].scansioni == 0
        && sum
            .empty
            .unwrap_or_else(|| l.chromatograms(i).is_ok_and(|v| v.is_empty()));
    [
        (refused, "refused"),
        (i + found >= n && found > 0, "inferred"),
        (none, "empty"),
    ]
    .into_iter()
    .find(|w| w.0)
    .map(|w| w.1)
}

/// The ids of the samples that acquired nothing (`why` = `empty` of `list_samples`): the ones a chooser never ticks. Read from the same listing as the page's.
pub fn empty_ids(l: &mut dyn Reader) -> Vec<u32> {
    let (found, _) = l.unindexed();
    let n = l.samples().len();
    let facts = l.listing();
    let mut ids = Vec::new();
    for i in 0..n {
        let sum = facts.samples.get(i).cloned().unwrap_or_default();
        if why_of(l, i, (found, n), &sum) == Some("empty") {
            ids.push(l.samples()[i].id);
        }
    }
    ids
}

/// JSON `{"format", "samples": [{"id", "name", "scans", "size"?, "why"?, "refused"?}], "unindexed"?: {"recovered", "skipped"}}`. `why` is `refused`
/// (with the error as `refused`), `inferred` (rebuilt from sections the index does not list: the last `recovered` samples) or `empty` (no scans, no chromatograms).
fn write(l: &mut dyn Reader, rich: bool) -> String {
    let (found, skipped) = l.unindexed();
    let n = l.samples().len();
    let facts = if rich {
        l.listing()
    } else {
        Listing::default()
    };
    let mut o = Out::default();
    head(&mut o, &*l, rich, &facts);
    o.raw(",\"samples\":[");
    for i in 0..n {
        let refused = l.refused(i);
        // a refused sample promises nothing in the rich listing: the engine still reads its index rows, the app's reader drops them, and both say the same
        let quiet = rich && refused.is_some();
        let sum = if quiet {
            Summary::default()
        } else {
            facts.samples.get(i).cloned().unwrap_or_default()
        };
        let why = why_of(l, i, (found, n), &sum);
        let s = &l.samples()[i];
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(i64::from(s.id))
            .raw(",\"name\":")
            .str(&s.name)
            .raw(",\"scans\":")
            .int(if quiet { 0 } else { s.scansioni as i64 });
        if let Some(dim) = s.size {
            o.raw(",\"size\":").int(if quiet { 0 } else { dim as i64 });
        }
        if rich {
            summary_json(&mut o, &sum);
        }
        if let Some(w) = why {
            o.raw(",\"why\":").str(w);
        }
        if let Some(e) = refused {
            o.raw(",\"refused\":").raw(&e.to_json());
        }
        o.raw("}");
    }
    o.raw("]");
    // sections of the data file the container's index does not list (an incomplete copy): the page says it once
    if found + skipped > 0 {
        o.raw(",\"unindexed\":{\"recovered\":")
            .int(found as i64)
            .raw(",\"skipped\":")
            .int(skipped as i64)
            .raw("}");
    }
    o.raw("}");
    o.0
}

/// `kinds`, `types`, `polarity`, `rt`, `tic`, `method` and `missing` of a sample, each only when known (`missing` only when above 0).
fn summary_json(o: &mut Out, s: &Summary) {
    if !s.kinds.is_empty() {
        o.raw(",\"kinds\":[");
        for (i, k) in s.kinds.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k);
        }
        o.raw("]");
    }
    if !s.types.is_empty() {
        o.raw(",\"types\":[");
        for (i, k) in s.types.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k);
        }
        o.raw("]");
    }
    if let Some(p) = s.polarity {
        o.raw(",\"polarity\":").str(p);
    }
    if let Some((a, b)) = s.rt {
        o.raw(",\"rt\":[").num(a).raw(",").num(b).raw("]");
    }
    if let Some((rt, apex, sum)) = s.tic {
        o.raw(",\"tic\":{\"apex_rt\":")
            .num(rt)
            .raw(",\"apex\":")
            .num(apex)
            .raw(",\"sum\":")
            .num(sum)
            .raw("}");
    }
    if let Some(m) = &s.method {
        o.raw(",\"method\":").str(m);
    }
    if s.missing > 0 {
        o.raw(",\"missing\":").int(s.missing as i64);
    }
}
