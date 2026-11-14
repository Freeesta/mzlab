//! Commands on spectra, channels and ions.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "msn-graph", aliases: &[], help: "mzlab-cli msn-graph FILE.mzML [--formula ION] [...]   the fragmentation graph of an MSn infusion (msn-graph --help)", run: |a| crate::msn_graph::cmd(a.into_iter()) },
    Entry { name: "channels", aliases: &["canali"], help: "mzlab-cli channels FILE [--out PATH]   the channels of a file as JSON (channels --help); dda-window FILE --from MIN --to MIN: the MS2 of a window by precursor (dda-window --help)", run: crate::channels_cmd::run },
    Entry { name: "dda-window", aliases: &[], help: "", run: crate::plain::dda_window_cmd::run },
    Entry { name: "ion-pattern", aliases: &[], help: "mzlab-cli ion-pattern FORMULA ADDUCT [--out PATH]   the unit-resolution isotope pattern of an ion as JSON (ion-pattern --help); ion-mz TEXT [--adduct A]: the m/z of an ion with a signed shift (ion-mz --help)", run: crate::ion_pattern_cmd::run },
    Entry { name: "ion-mz", aliases: &[], help: "", run: crate::plain::ion_mz_cmd::run },
    Entry { name: "match-mz", aliases: &[], help: "mzlab-cli match-mz X --tables FILE.json [--set hr=1 ppm=N]   what a number is compatible with: losses, adduct shifts, known contaminants, as candidates (match-mz --help)", run: crate::plain::match_mz_cmd::run },
    Entry { name: "background", aliases: &[], help: "", run: crate::plain::background_cmd::run },
];
