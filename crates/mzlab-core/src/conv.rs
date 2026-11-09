//! `conv/1`: what a conversion carried into the mzML and what it left out (and the zlib implementation that wrote the arrays), written in the `<run>` by `readers_mzml::Conversione`.
//! Each reader declares what its format holds and the mzML does not (`Lettore::not_carried`), always with the reason.

use crate::error::Result;
use crate::readers::{Field, Reader};

/// Reasons shared by the readers: one text each, so a module carries it once.
pub const NON_DECODIFICATO: &str = "layout not decoded";
pub const WITHOUT_TERM: &str = "no mzML term";
pub const IDENTITA: &str = "identity, kept out of the mzML";
pub const IN_METODO: &str = "kept in metodo/1, not in the mzML";
pub const UNTRIED: &str = "no file at hand to validate";
pub const OUT_OF_MODEL: &str = "outside the mzML model";

/// Gaps of the Thermo readers.
pub const THERMO: &[(&str, &str)] = &[
    (
        "UV 3D field of a PDA detector (spectra per instant)",
        NON_DECODIFICATO,
    ),
    ("analog channels other than the pump pressure", IN_METODO),
    ("status log series and error log", IN_METODO),
    ("tune block of LTQ and TSQ files", UNTRIED),
    (
        "embedded LC method of Dionex, EASY-nLC, Accela, Surveyor and ACQUITY",
        IN_METODO,
    ),
    ("ion trap experiment settings (Scan Event N)", IN_METODO),
];

/// Gaps of the Agilent LC/MS readers.
pub const AGILENT_EXT: &[(&str, &str)] = &[
    (
        "sample type, GC method, effective values, quantification method and audit trail",
        NON_DECODIFICATO,
    ),
    (
        "DAD1 files, module traces (pressure, flow, temperature) and MSActual.bin",
        IN_METODO,
    ),
];

/// Scans looked at to count the fields: the first MS1 and the first MSn among the first rows.
const ROWS: usize = 64;

/// The `conv/1` document of sample `c`: reader, version, how many header fields of a scan have a CV term and how many only a name, and the declared gaps.
pub(crate) fn resoconto(l: &mut dyn Reader, c: usize) -> Result<String> {
    let mut seen = [false; 2];
    let mut ks = Vec::new();
    for (k, v) in l.index_without_tic(c)?.iter().take(ROWS).enumerate() {
        let i = usize::from(v.livello > 1);
        if !seen[i] {
            seen[i] = true;
            ks.push(k);
        }
    }
    let mut fields: Vec<Field> = Vec::new();
    for k in ks {
        fields.extend(l.scan_header(c, k)?);
    }
    let mut names: Vec<&str> = Vec::new();
    for f in &fields {
        if !names.contains(&f.nome.as_str()) {
            names.push(&f.nome);
        }
    }
    let with_cv = |n: &str| fields.iter().any(|f| f.nome == n && f.cv.is_some());
    let cv = names.iter().filter(|n| with_cv(n)).count();
    let mut s = String::from("{\"schema\":\"conv/1\",\"lettore\":\"");
    s += l.format().id();
    s += "\",\"versione\":\"";
    s += env!("CARGO_PKG_VERSION");
    s += "\",\"compressore\":\"zlib-rs\",\"campi_cv\":";
    s += &cv.to_string();
    s += ",\"campi_user\":";
    s += &(names.len() - cv).to_string();
    s += ",\"non_portati\":[";
    for (i, (campo, motivo)) in l.not_carried().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // The texts are static and carry no quote or backslash.
        s += "{\"campo\":\"";
        s += campo;
        s += "\",\"motivo\":\"";
        s += motivo;
        s += "\"}";
    }
    s += "]}";
    Ok(s)
}
