//! The error log of a `.raw` (`RawFileReader::error_log`): what the instrument reported while it acquired (an unstable spray, a failed calibration, a warning of the electronics, the
//! configuration of the ETD reagent), as the entries of `metodo/1.registro`: `t_min` (minutes), `testo` as the file says it and `livello` when the text begins with it.

use crate::metodo::{num, obj, text, V};
use opentfraw::error_log::ErrorEntry;

const LIVELLI: [&str; 3] = ["Warning", "Info", "Error"];

/// The level a text announces at its start (`Warning: …`), as written in the list above.
fn livello(testo: &str) -> Option<&'static str> {
    let t = testo.trim_start();
    LIVELLI.into_iter().find(|l| {
        t.get(..l.len()).is_some_and(|p| p.eq_ignore_ascii_case(l))
            && !t[l.len()..].starts_with(|c: char| c.is_alphanumeric())
    })
}

pub fn voci(log: &[ErrorEntry]) -> Vec<V> {
    log.iter()
        .filter_map(|e| {
            obj(vec![
                ("t_min", num(Some(f64::from(e.time)))),
                ("testo", text(&e.message)),
                ("livello", livello(&e.message).map(V::s)),
            ])
        })
        .collect()
}

#[cfg(test)]
mod tests;
