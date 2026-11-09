//! How many datasets the conversion queue converts at once: the threads of `mzlab-cli convert` on the memory budget of the computer
//! (`budget_auto::convert_workers`), and the notice `conv_list` gives the page when the budget runs fewer than the default, the line the CLI prints.

use serde_json::{json, Value};

use super::Queue;

impl Queue {
    /// A queue on the default threads of a computer whose native budget is `mb`.
    #[cfg(test)]
    pub(crate) fn with_budget(mb: f64) -> Queue {
        Queue {
            auto_mb: mb,
            ..Queue::default()
        }
    }

    fn budget_mb(&self) -> f64 {
        if self.auto_mb > 0.0 {
            self.auto_mb
        } else {
            *self
                .native_mb
                .get_or_init(mzlab_core::budget_auto::native_auto_mb)
        }
    }

    pub(crate) fn limit(&self) -> usize {
        if self.workers > 0 {
            self.workers
        } else {
            mzlab_core::budget_auto::convert_workers(
                self.budget_mb(),
                mzlab_core::budget_auto::convert_wanted(),
            )
        }
    }

    /// The notice of a budget that runs fewer conversions at once than the default, while more datasets wait than it runs: the very one
    /// `mzlab-cli convert` prints.
    pub(super) fn notice(&self) -> Option<Value> {
        let n = (self.workers == 0)
            .then(|| {
                mzlab_cli::convert::fewer_workers(
                    self.budget_mb(),
                    mzlab_core::budget_auto::convert_wanted(),
                )
            })
            .flatten()?;
        let params: serde_json::Map<String, Value> = n
            .params
            .into_iter()
            .map(|(k, v)| (k.to_string(), Value::String(v)))
            .collect();
        Some(json!({ "key": n.key, "params": params }))
    }
}
