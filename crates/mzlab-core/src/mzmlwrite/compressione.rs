//! The named compression choices of the mzML download (page, desktop app and CLI share this one table).

use super::Opzioni;

/// The three choices of the download dialog (the one table of the page, the desktop app and the CLI): `standard` = zlib (the default), `smallest` = zlib and
/// MS-Numpress, `none` = arrays not compressed. `None` for any other name.
pub fn opzioni_compressione(kind: &str) -> Option<Opzioni> {
    let base = Opzioni::default();
    match kind {
        "standard" => Some(base),
        "smallest" => Some(Opzioni {
            numpress: true,
            ..base
        }),
        "none" => Some(Opzioni { livello: 0, ..base }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_map_to_the_options_and_standard_is_the_default() {
        let d = Opzioni::default();
        let s = opzioni_compressione("standard").unwrap();
        assert_eq!((s.livello, s.numpress), (d.livello, false));
        let m = opzioni_compressione("smallest").unwrap();
        assert_eq!((m.livello, m.numpress), (d.livello, true));
        let n = opzioni_compressione("none").unwrap();
        assert_eq!((n.livello, n.numpress), (0, false));
        assert!(opzioni_compressione("zip").is_none());
    }
}
