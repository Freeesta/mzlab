//! The index row of an mzXML scan from the attributes of its `<scan>`: level, polarity, filter, retention time, TIC and precursor; used by `mzxml`.

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Position, ScanEntry};

use super::mzxml::number;
use super::mzxml_indice::Row;
use super::mzxml_scan::{self as scan, attr, Head, Tag};

fn polarity(t: &str) -> i8 {
    match t.trim() {
        "+" | "positive" | "Positive" => 1,
        "-" | "negative" | "Negative" => -1,
        _ => 0,
    }
}

/// The filter of a scan: its `filterLine`, with the scan type in front for SRM/MRM scans whose line does not say it (the SRM traces are found by it).
fn filter(t: Tag) -> String {
    let line = attr(t, "filterLine").trim();
    let kind = attr(t, "scanType").trim();
    let low = line.to_ascii_lowercase();
    let srm = ["srm", "mrm", "crm"]
        .iter()
        .any(|x| kind.eq_ignore_ascii_case(x));
    if srm && !low.contains("srm") && !low.contains("mrm") {
        format!("{kind} {line}").trim().to_string()
    } else {
        line.to_string()
    }
}

/// The index entry of scan `k` from its attributes.
pub(super) fn entry(k: usize, r: Row, t: &Head) -> Result<ScanEntry> {
    let length = u32::try_from(r.end - r.start).map_err(|_| Error::memory_budget())?;
    let num = attr(t.scan, "num").trim();
    Ok(ScanEntry {
        id: if num.is_empty() {
            format!("scan={}", k + 1)
        } else {
            format!("scan={num}")
        },
        rt: scan::minutes(attr(t.scan, "retentionTime")).unwrap_or(0.0),
        livello: number(t.scan, "msLevel").map_or(1, |l| l.clamp(1.0, 255.0) as u8),
        polarita: polarity(attr(t.scan, "polarity")),
        tic: number(t.scan, "totIonCurrent").unwrap_or(0.0),
        base_peak: number(t.scan, "basePeakMz"),
        filtro: filter(t.scan),
        precursore: t.precursori.iter().rev().find_map(|p| p.1.parse().ok()),
        position: Some(Position {
            membro: 0,
            offset: r.start,
            lunghezza: length,
        }),
    })
}
