//! The samples of a dataset as the site and the app list them (`samples_json`): each with why it has nothing to show, so that a sample the reader
//! refuses carries its reason and never looks like an injection that acquired nothing (H6-15).

use super::Reader;
use crate::json::Out;

/// JSON `{"format", "samples": [{"id", "name", "scans", "size"?, "why"?, "refused"?, "noPrecursor"?}], "unindexed"?: {"recovered", "skipped"}}`. `why` is `refused`
/// (with the error as `refused`), `inferred` (rebuilt from sections the index does not list: the last `recovered` samples) or `empty` (no scans, no chromatograms).
pub fn samples_json(l: &mut dyn Reader) -> String {
    let (found, skipped) = l.unindexed();
    let n = l.samples().len();
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(&format!("{:?}", l.format()))
        .raw(",\"samples\":[");
    for i in 0..n {
        let refused = l.refused(i);
        let unread = l.unread_precursors(i);
        let none = refused.is_none()
            && l.samples()[i].scansioni == 0
            && l.chromatograms(i).is_ok_and(|v| v.is_empty());
        let why = [
            (refused.is_some(), "refused"),
            (i + found >= n && found > 0, "inferred"),
            (none, "empty"),
        ]
        .into_iter()
        .find(|w| w.0);
        let s = &l.samples()[i];
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(i64::from(s.id))
            .raw(",\"name\":")
            .str(&s.nome)
            .raw(",\"scans\":")
            .int(s.scansioni as i64);
        if let Some(dim) = s.size {
            o.raw(",\"size\":").int(dim as i64);
        }
        if let Some((_, w)) = why {
            o.raw(",\"why\":").str(w);
        }
        if let Some(e) = refused {
            o.raw(",\"refused\":").raw(&e.to_json());
        }
        // dependent (IDA) scans written without a precursor: the page gives the sample one calm note (`warn.wiff.idaPrecursor`)
        if unread > 0 {
            o.raw(",\"noPrecursor\":").int(unread as i64);
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
