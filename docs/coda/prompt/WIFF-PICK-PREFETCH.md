Read docs/coda/BRIEF.md.

# WIFF-PICK-PREFETCH: Start loading a ticked sample before "Open selected"

**Goal.** The owner asked (relayed by the Coordinator) that a sample starts loading in the background as soon as it is ticked in the "Choose samples" dialog (`mzlab/web/wiff-choose.js`), so that "Open selected" opens instantly.
Default-ticked samples start when the dialog opens; unticking cancels or discards the work; the X (or Esc) cancels everything and frees memory; "Open selected" reuses what is loaded (no second read). The dialog is never blocked (a small per-row progress mark through `lang/` keys). Light: sequential or at most 2 parallel loads, no new dependency, files stay on the computer.

**Read first.** `docs/agenti/wiff-sample-choice.md`, `docs/agenti/wiff-range-copy.md`, `docs/agenti/native-reading.md`, `wiffToMzml` in `mzlab/web/shell.js`.

**Done means.** (1) The loading logic is in Rust/wasm (the reader and the range copy), JS only triggers and draws. (2) An e2e ticks a sample and sees loading start before Open, and sees untick and X cancel it. (3) Memory limits of `memory-limits.md` still hold with several samples ticked. (4) Parity: the app route does the same.

**Why it is its own PR.** Today the load of a sample starts only after the choice: `wiffToMzml` (`shell.js`) names each sample from the whole choice (`wiffName`, the `sample_labels` over every chosen sample), plans the byte ranges of the whole subset (`nativePlan`, a tag per subset) and `lr_put` / `lr_index` (`lr.js`) copy and index under that name. A load started at tick time knows none of that.
It needs 
(a) a per-sample range plan and a selection-independent OPFS name, (b) a worker step that adopts a loaded sample under its final name (there is no rename op), (c) cancel through `lr_cancel` + `lr_remove`, at most 2 copies at once through the existing `copies` queue, (d) its rows in the loading table. That is the load pipeline, not the dialog.

**Out of scope.** Changing the dialog layout, the default ticks or the listing.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
