//! JSON text helpers of the log lines of the agent (a string quoted). Used by `agent`.

use mzlab_core::json::Out;

pub fn quoted(s: &str) -> String {
    let mut o = Out::default();
    o.str(s);
    o.0
}
