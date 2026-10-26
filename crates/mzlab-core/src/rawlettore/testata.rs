//! The mzML header of a `.raw` (`testata`) and the light one the scan table needs (`testata_tabella`); used by `rawlettore` through `Lettore`.

use super::LettoreRaw;
use crate::error::Result;
use crate::metodo::{self, sha1_hex};
use crate::mzmlwrite::Header;

/// The header of the mzML: the instrument, the run terms the Thermo converter writes (`opentfraw.*`), the start of the acquisition, the SHA-1 of the
/// file and the `metodo/1` document, so that the download of a `.raw` loses none of them.
fn testata(l: &mut LettoreRaw) -> Header {
    if l.sha1.is_none() {
        let len = l.src.len;
        l.sha1 = Some(sha1_hex(&mut l.src.inner, len).ok());
    }
    let sha1 = l.sha1.clone().flatten();
    let t = opentfraw::mzml::run_terms(&l.raw);
    let run = l
        .campioni
        .first()
        .map(|c| c.nome.clone())
        .unwrap_or_default();
    let document = crate::raw::metodo_sha1(&l.raw, &mut l.src, &l.nome, sha1.clone());
    let acquired = t
        .start_timestamp
        .or_else(|| crate::metodo_raw::campi::clock(l.raw.header.audit_start.time));
    let mut run_params: Vec<(String, String)> = t.extra;
    run_params.push((metodo::USERPARAM.to_string(), document));
    Header {
        run,
        sources: vec![l.nome.clone()],
        source_sha1: sha1.into_iter().collect(),
        source_cv: vec![
            ("MS:1000563".into(), "Thermo RAW format".into()),
            ("MS:1000768".into(), "Thermo nativeID format".into()),
        ],
        instrument: Some(t.instrument),
        components: t
            .analyzers
            .into_iter()
            .map(|(a, n)| ("analyzer".to_string(), a.to_string(), n.to_string()))
            .collect(),
        acquired,
        run_params,
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        ..Header::default()
    }
}

/// The fields of `testata` that `scantable::Head::da_testata` reads (sources, instrument, analyzers), without the SHA-1 of the file or the `metodo/1`
/// document: the table is the same, the header costs a fraction.
fn testata_tabella(l: &mut LettoreRaw) -> Header {
    let t = opentfraw::mzml::run_terms(&l.raw);
    Header {
        sources: vec![l.nome.clone()],
        source_cv: vec![
            ("MS:1000563".into(), "Thermo RAW format".into()),
            ("MS:1000768".into(), "Thermo nativeID format".into()),
        ],
        instrument: Some(t.instrument),
        components: t
            .analyzers
            .into_iter()
            .map(|(a, n)| ("analyzer".to_string(), a.to_string(), n.to_string()))
            .collect(),
        ..Header::default()
    }
}

pub(super) fn completa(l: &mut LettoreRaw, c: usize) -> Result<Option<Header>> {
    l.one(c)?;
    Ok(Some(testata(l)))
}

pub(super) fn leggera(l: &mut LettoreRaw, c: usize) -> Result<Option<Header>> {
    l.one(c)?;
    Ok(Some(testata_tabella(l)))
}
