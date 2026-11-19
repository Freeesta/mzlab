//! The `msn` group: the fragmentation graph of an MSn infusion, the library of MSn trees and the fragmentation rules.

use super::{pre, Entry};

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("msn graph", "mzlab-cli msn graph FILE.mzML [--formula ION] [...]   the fragmentation graph of an MSn infusion", |a| crate::msn_graph::cmd(a.into_iter())).schema("msn-graph/1"),
    Entry::new("msn complete", "mzlab-cli msn complete GRAPH.json ...   the graph completed with the missing nodes", |a| crate::msn_graph::cmd(pre("complete", a).into_iter())),
    Entry::new("msn compare", "mzlab-cli msn compare GRAPH.json GRAPH.json ...   two graphs side by side", |a| crate::msn_graph::cmd(pre("compare", a).into_iter())),
    Entry::new("msn figure", "mzlab-cli msn figure GRAPH.json ...   the figure of a graph", |a| crate::msn_graph::cmd(pre("figure", a).into_iter())),
    Entry::new("msn lib add", "mzlab-cli msn lib add ...   add trees to the library of MSn trees", |a| crate::msn_lib::cmd(pre("add", a))),
    Entry::new("msn lib build", "mzlab-cli msn lib build ...   build the library of MSn trees", |a| crate::msn_lib::cmd(pre("build", a))),
    Entry::new("msn lib export", "mzlab-cli msn lib export ...   export the library of MSn trees", |a| crate::msn_lib::cmd(pre("export", a))),
    Entry::new("msn lib search", "mzlab-cli msn lib search ...   search the library of MSn trees", |a| crate::msn_lib::cmd(pre("search", a))),
    Entry::new("msn rules", "mzlab-cli msn rules RECORD... [--rules] [--decoy SEED]   the fragmentation rules judged on library spectra", crate::msn_rules::run),
    Entry::new("msn rules sum", "mzlab-cli msn rules sum LINES...   the rule lines of several runs summed", |a| crate::msn_rules::run(pre("sum", a))),
];
