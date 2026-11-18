//! Commands on spectra, channels and ions.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "msn-graph", aliases: &[], help: "mzlab-cli msn-graph FILE.mzML [--formula ION] [...]   the fragmentation graph of an MSn infusion (msn-graph --help)", run: |a| crate::msn_graph::cmd(a.into_iter()) },
    Entry { name: "channels", aliases: &[], help: "mzlab-cli channels FILE [--out PATH]   the channels of a file as JSON (channels --help); dda-window FILE --from MIN --to MIN: the MS2 of a window by precursor (dda-window --help)", run: crate::channels_cmd::run },
    Entry { name: "pair", aliases: &[], help: "mzlab-cli pair FILE... [--json]   the other half of a SCIEX pair (.wiff + .wiff.scan) found by name in the folder of the file; exit 1 when one is missing (pair --help)", run: crate::pair_cmd::run },
    Entry { name: "dad", aliases: &[], help: "mzlab-cli dad subtract|spectrum|trace|ms-at|delay ...   the DAD / PDA matrix as JSON for agents: A minus B, the UV spectrum and trace, the linked MS (dad --help)", run: crate::dad_cmd::run },
    Entry { name: "wiff", aliases: &[], help: "mzlab-cli wiff list FILE.wiff... [--inventory]   what a .wiff + .wiff.scan holds, sample by sample, with SHA-256 and a status as JSON; wiff verify OUT.wiff --against SRC.wiff...: whether an output equals its sources (wiff --help)", run: crate::wiff_cmd::run },
    Entry { name: "items", aliases: &[], help: "mzlab-cli items FILE [--out PATH]   what the page learns about a file as JSON: counts, ranges, ms2_exps with CE and its spread (items --help)", run: crate::items_cmd::run },
    Entry { name: "dda-window", aliases: &[], help: "", run: crate::plain::dda_window_cmd::run },
    Entry { name: "ion-pattern", aliases: &[], help: "mzlab-cli ion-pattern FORMULA ADDUCT [--out PATH]   the unit-resolution isotope pattern of an ion as JSON (ion-pattern --help); ion-mz TEXT [--adduct A]: the m/z of an ion with a signed shift (ion-mz --help)", run: crate::ion_pattern_cmd::run },
    Entry { name: "folder", aliases: &[], help: "", run: crate::folder_cmd::run },
    Entry { name: "elements", aliases: &[], help: "", run: crate::plain::elements_cmd::run },
    Entry { name: "ion-mz", aliases: &[], help: "", run: crate::plain::ion_mz_cmd::run },
    Entry { name: "match-mz", aliases: &[], help: "mzlab-cli match-mz X --tables FILE.json [--set hr=1 ppm=N]   what a number is compatible with: losses, adduct shifts, known contaminants, as candidates (match-mz --help)", run: crate::plain::match_mz_cmd::run },
    Entry { name: "ref-list", aliases: &[], help: "mzlab-cli ref-list FILE.csv [--name N] [--match X]   read a CSV list of reference ions as JSON (reflist/1) with line-numbered row errors; exit 1 if rows are wrong (ref-list --help)", run: crate::plain::ref_list_cmd::run },
    Entry { name: "background", aliases: &[], help: "", run: crate::plain::background_cmd::run },
];
