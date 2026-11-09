//! The named compression choices of the mzML download (page, desktop app and CLI share this one table).

use super::Options;

/// The three choices of the download dialog (the one table of the page, the desktop app and the CLI): `standard` = zlib (the default), `smallest` = zlib and
/// MS-Numpress, `none` = arrays not compressed. `None` for any other name.
pub fn compression_options(kind: &str) -> Option<Options> {
    let base = Options::default();
    match kind {
        "standard" => Some(base),
        "smallest" => Some(Options {
            numpress: true,
            ..base
        }),
        "none" => Some(Options { livello: 0, ..base }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_map_to_the_options_and_standard_is_the_default() {
        let d = Options::default();
        let s = compression_options("standard").unwrap();
        assert_eq!((s.livello, s.numpress), (d.livello, false));
        let m = compression_options("smallest").unwrap();
        assert_eq!((m.livello, m.numpress), (d.livello, true));
        let n = compression_options("none").unwrap();
        assert_eq!((n.livello, n.numpress), (0, false));
        assert!(compression_options("zip").is_none());
    }
}
