//! The route `flow/live/tables` of `rpc` (`flow_live`): the `tables/1` document of the run folder of the app (`mzlab_cli::flow_report::tables`, what `mzlab-cli flow tables` prints),
//! for the list of feature tables on the right of the Flow. No run yet is an empty list, not an error.

use mzlab_core::Result;

use crate::engine::State;
use crate::rpc::{io_error, text, Answer};

impl State {
    /// `{}` -> `tables/1` (`tables` and `skipped`) of the outputs of the last run.
    pub(crate) fn flow_tables(&self) -> Result<Answer> {
        let dir = self.flow_root().join("out");
        if !dir.is_dir() {
            return text(mzlab_feat::tables::document(&[], &[]));
        }
        let (doc, _) =
            mzlab_cli::flow_report::tables::document_of(&dir).map_err(|e| io_error(&e))?;
        text(doc)
    }
}
