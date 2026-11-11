//! The mzML header of a `.raw` (`testata`) and the light one the scan table needs (`testata_tabella`); used by `rawlettore` through `Lettore`.

use super::RawReader;
use crate::error::Result;
use crate::method::{self, sha1_hex};
use crate::mzmlwrite::Header;

/// The header of the mzML: the instrument, the run terms the Thermo converter writes (`opentfraw.*`), the start of the acquisition, the SHA-1 of the
/// file and the `metodo/1` document, so that the download of a `.raw` loses none of them.
fn file_header(l: &mut RawReader) -> Header {
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
    let document = crate::raw::metodo_sha1(&l.raw, &mut l.src, &l.name, sha1.clone(), &l.notes);
    let acquired = t
        .start_timestamp
        .or_else(|| crate::method_raw::fields::clock(l.raw.header.audit_start.time));
    let mut run_params: Vec<(String, String)> = t.extra;
    run_params.push((method::USERPARAM.to_string(), document));
    Header {
        run,
        sources: vec![l.name.clone()],
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
fn table_header(l: &mut RawReader) -> Header {
    let t = opentfraw::mzml::run_terms(&l.raw);
    Header {
        sources: vec![l.name.clone()],
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

pub(super) fn complete(l: &mut RawReader, c: usize) -> Result<Option<Header>> {
    l.one(c)?;
    Ok(Some(file_header(l)))
}

pub(super) fn leggera(l: &mut RawReader, c: usize) -> Result<Option<Header>> {
    l.one(c)?;
    Ok(Some(table_header(l)))
}
