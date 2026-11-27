Read docs/coda/BRIEF.md.

# WIFF-PICK-INDEX-AHEAD: Build the index of a ticked sample while the dialog is open

**Goal.** The sample chooser (`mzlab/web/wiff-choose.js`) already copies the ticked samples ahead (`mzlab/web/rust-prefetch.js`, `docs/agenti/wiff-sample-choice.md`, «Copy ahead»); what is still done after «Open selected» is the index (`lr_index` builds `idx/<stub name>.mzidx`). Do that step ahead too, so the opening is instant.

**Read first.** `docs/agenti/wiff-sample-choice.md`, `docs/agenti/wiff-range-copy.md`, `wiffToMzml` in `mzlab/web/shell.js` (`wiffName`, `sample_labels`), `indexJob` in `mzlab/web/rust-nativo.js`.

**Why it is not done.** The stub name, and so the index name, depends on the whole selection (`wiffName` over every chosen sample) and there is no rename op for an index, so an index built at tick time would be under a name that may change.

**Done means.** (1) Either an index name that does not depend on the selection, or a worker op that adopts an index under the final name (no copy of the bytes). (2) An e2e shows the index exists before Open and is reused (same bytes, not rebuilt); untick and X remove it. (3) Memory limits of `memory-limits.md` hold with several samples ticked; at most 2 builds at once. (4) The app and CLI have nothing to prefetch (they read in place): say so, no parity gap.

**Out of scope.** The copy ahead (done), the dialog layout, the default ticks.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
