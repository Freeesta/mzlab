//! Commands on samples, peaks and manual integrations.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "samples", aliases: &["campioni"], help: "mzlab-cli samples vars|group ...   how the file names split the files, as JSON (samples --help)", run: crate::samples_cmd::run },
    Entry { name: "peak-edges", aliases: &["peaks-edges"], help: "mzlab-cli peak-edges TRACE.csv --at MIN [--set NAME=VALUE...]   the limits of a low-resolution peak on a trace (peak-edges --help)", run: crate::peaks_edges_cmd::run },
    Entry { name: "manual-table", aliases: &[], help: "mzlab-cli manual-table FILE... --set windows=PATH   the integrations drawn by hand on transition / SIM traces, as CSV (manual-table --help); peak-split FILE... --set tr=J --set a=MIN --set b=MIN: one of them split at its valleys (peak-split --help)", run: crate::manual_table_cmd::run },
    Entry { name: "peak-split", aliases: &[], help: "", run: crate::plain::peak_split_cmd::run },
    Entry { name: "peak-auto", aliases: &[], help: "mzlab-cli peak-auto TRACE.csv [--from MIN --to MIN] [--set NAME=VALUE...]   every peak of a high-resolution trace (peak-auto --help); peak-area: one peak from zero or a line (peak-area --help); peak-window: where a dragged window end goes (peak-window --help)", run: crate::peaks_auto_cmd::run },
    Entry { name: "peak-area", aliases: &[], help: "", run: crate::plain::peak_area_cmd::run },
    Entry { name: "peak-window", aliases: &[], help: "", run: crate::plain::peak_window_cmd::run },
    Entry { name: "explain-missing", aliases: &[], help: "mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv   where a table loses each expected product (explain-missing --help)", run: crate::flow_explain::run },
];
