//! The `peak` group: peaks of a trace, automatic or by hand, and the manual integration tables.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("peak auto", "mzlab-cli peak auto TRACE.csv|- [--from MIN --to MIN] [--baseline zero|edges|line] [--set NAME=VALUE...]   every high-resolution peak", crate::peaks_auto_cmd::run).schema("peak-auto/1"),
    Entry::new("peak auto-lr", "mzlab-cli peak auto-lr TRACE.csv|- [--from MIN --to MIN] [--split drop|valley] [--set NAME=VALUE...]   every peak of a low-resolution SIM/MRM trace", crate::peaks_auto_lr_cmd::run),
    Entry::new("peak edges", "mzlab-cli peak edges TRACE.csv|- --at MIN [--noise mad|rms|manual] [--set NAME=VALUE...]   limits of a low-resolution peak", crate::peaks_edges_cmd::run),
    Entry::new("peak area", "mzlab-cli peak area TRACE.csv|- --from MIN --to MIN [--baseline zero|line] [--set NAME=VALUE...]   one peak from zero or a line", crate::plain::peak_area_cmd::run),
    Entry::new("peak window", "mzlab-cli peak window TRACE.csv|- --end MIN --other MIN [--at MIN] [--step N]   where a dragged window end goes (--at or --step)", crate::plain::peak_window_cmd::run),
    Entry::new("peak split", "mzlab-cli peak split FILE... --tr J --a MIN --b MIN [--baseline zero|edges|line] [--split auto|MIN,...]   one peak split at its valleys", crate::plain::peak_split_cmd::run),
    Entry::new("peak table", "mzlab-cli peak table FILE... --windows PATH [--traces PATH] [--columns sn,fig,quality] [--json]   hand integrations of transition/SIM traces, as CSV (--traces: no FILEs)", crate::manual_table_cmd::run),
];
