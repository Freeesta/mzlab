//! The index row of an mzXML scan from the attributes of its `<scan>`: level, polarity, filter, retention time, TIC and precursor; used by `mzxml`.

use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::{Posizione, VoceScansione};

use super::mzxml::numero;
use super::mzxml_indice::Riga;
use super::mzxml_scan::{self as scan, attr, Tag, Testa};

fn polarita(t: &str) -> i8 {
    match t.trim() {
        "+" | "positive" | "Positive" => 1,
        "-" | "negative" | "Negative" => -1,
        _ => 0,
    }
}

/// The filter of a scan: its `filterLine`, with the scan type in front for SRM/MRM scans whose line does not say it (the SRM traces are found by it).
fn filtro(t: Tag) -> String {
    let linea = attr(t, "filterLine").trim();
    let tipo = attr(t, "scanType").trim();
    let basso = linea.to_ascii_lowercase();
    let srm = ["srm", "mrm", "crm"]
        .iter()
        .any(|x| tipo.eq_ignore_ascii_case(x));
    if srm && !basso.contains("srm") && !basso.contains("mrm") {
        format!("{tipo} {linea}").trim().to_string()
    } else {
        linea.to_string()
    }
}

/// The index entry of scan `k` from its attributes.
pub(super) fn voce(k: usize, r: Riga, t: &Testa) -> Result<VoceScansione> {
    let lunghezza = u32::try_from(r.end - r.start).map_err(|_| Error::memory_budget())?;
    let num = attr(t.scan, "num").trim();
    Ok(VoceScansione {
        id: if num.is_empty() {
            format!("scan={}", k + 1)
        } else {
            format!("scan={num}")
        },
        rt: scan::minuti(attr(t.scan, "retentionTime")).unwrap_or(0.0),
        livello: numero(t.scan, "msLevel").map_or(1, |l| l.clamp(1.0, 255.0) as u8),
        polarita: polarita(attr(t.scan, "polarity")),
        tic: numero(t.scan, "totIonCurrent").unwrap_or(0.0),
        picco_base: numero(t.scan, "basePeakMz"),
        filtro: filtro(t.scan),
        precursore: t.precursori.iter().rev().find_map(|p| p.1.parse().ok()),
        posizione: Some(Posizione {
            membro: 0,
            offset: r.start,
            lunghezza,
        }),
    })
}
