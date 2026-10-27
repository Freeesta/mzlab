//! `--set INSTANCE.NAME=VALUE` of `mzlab-cli flow run` and `convert`: the code lives in `mzlab_core::flow` (the page applies the same lines). Used by `flow_run.rs`,
//! `convert.rs` and the rule tests.

pub(crate) use mzlab_core::flow::apply_set;
