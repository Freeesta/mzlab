//! The `msn` group: the fragmentation graph of an MSn infusion, the library of MSn trees and the fragmentation rules.

use super::{pre, Entry};

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("msn graph", "mzlab-cli msn graph FILE.mzML [--formula ION] [--summary] [...]   the fragmentation graph of an MSn infusion", |a| crate::msn_graph::cmd(a.into_iter())).schema("msn-graph/1").exits(&[0, 1, 2, 4, 5, 6]),
    Entry::new("msn complete", "mzlab-cli msn complete GRAPH.json ...   the graph completed with the missing nodes", |a| crate::msn_graph::cmd(pre("complete", a).into_iter())).exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn compare", "mzlab-cli msn compare GRAPH.json GRAPH.json ...   two graphs side by side", |a| crate::msn_graph::cmd(pre("compare", a).into_iter())).exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn figure", "mzlab-cli msn figure GRAPH.json ...   the figure of a graph", |a| crate::msn_graph::cmd(pre("figure", a).into_iter())).schema("msn-figures/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn lib add", "mzlab-cli msn lib add LIB.json FILE.mzML --id ID --formula ION [--dry-run]   add an infusion to the MSn library", |a| crate::msn_lib::cmd(pre("add", a))).schema("msn-lib-op/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn lib build", "mzlab-cli msn lib build MANIFEST.tsv [--dry-run]   build the MSn library", |a| crate::msn_lib::cmd(pre("build", a))).schema("msn-lib-op/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn lib export", "mzlab-cli msn lib export LIB.json   export the library spectra", |a| crate::msn_lib::cmd(pre("export", a))).schema("msn-lib-export/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn lib search", "mzlab-cli msn lib search ...   search the library of MSn trees", |a| crate::msn_lib::cmd(pre("search", a))).schema("msn-search/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn rules", "mzlab-cli msn rules RECORD... [--rules] [--decoy SEED]   the fragmentation rules judged on library spectra", crate::msn_rules::run).schema("msn-rules/1").exits(&[0, 1, 2, 4, 6]),
    Entry::new("msn rules sum", "mzlab-cli msn rules sum DOC...   the msn-rules/1 documents summed", |a| crate::msn_rules::run(pre("sum", a))).schema("msn-rules-sum/1").exits(&[0, 1, 2, 4, 6]),
];
