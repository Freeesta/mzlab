//! `conv/1`: what a conversion carried into the mzML and what it left out, written in the `<run>` by `lettori_mzml::Conversione`.
//! Each reader declares what its format holds and the mzML does not (`Lettore::non_portati`), always with the reason.

use crate::error::Result;
use crate::lettori::{Campo, Lettore};

/// Reasons shared by the readers: one text each, so a module carries it once.
pub const NON_DECODIFICATO: &str = "layout not decoded";
pub const SENZA_TERMINE: &str = "no mzML term";
pub const IDENTITA: &str = "identity, kept out of the mzML";
pub const IN_METODO: &str = "kept in metodo/1, not in the mzML";
pub const NON_PROVATO: &str = "no file at hand to validate";
pub const FUORI_MODELLO: &str = "outside the mzML model";

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Scans looked at to count the fields: the first MS1 and the first MSn among the first rows.
const RIGHE: usize = 64;

/// The `conv/1` document of sample `c`: reader, version, how many header fields of a scan have a CV term and how many only a name, and the declared gaps.
pub(crate) fn resoconto(l: &mut dyn Lettore, c: usize) -> Result<String> {
    let mut visti = [false; 2];
    let mut ks = Vec::new();
    for (k, v) in l.indice(c)?.iter().take(RIGHE).enumerate() {
        let i = usize::from(v.livello > 1);
        if !visti[i] {
            visti[i] = true;
            ks.push(k);
        }
    }
    let mut campi: Vec<Campo> = Vec::new();
    for k in ks {
        campi.extend(l.intestazione(c, k)?);
    }
    let mut nomi: Vec<&str> = campi.iter().map(|f| f.nome.as_str()).collect();
    nomi.sort_unstable();
    nomi.dedup();
    let con_cv = |n: &str| campi.iter().any(|f| f.nome == n && f.cv.is_some());
    let cv = nomi.iter().filter(|n| con_cv(n)).count();
    let mut s = format!(
        "{{\"schema\":\"conv/1\",\"lettore\":\"{}\",\"versione\":\"{}\",\"campi_cv\":{cv},\"campi_user\":{},\"non_portati\":[",
        l.formato().id(),
        env!("CARGO_PKG_VERSION"),
        nomi.len() - cv
    );
    for (i, (campo, motivo)) in l.non_portati().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s += &format!("{{\"campo\":\"{}\",\"motivo\":\"{}\"}}", esc(campo), esc(motivo));
    }
    s += "]}";
    Ok(s)
}
