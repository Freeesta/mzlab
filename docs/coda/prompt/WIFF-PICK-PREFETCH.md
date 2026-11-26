Read docs/coda/BRIEF.md.

# WIFF-PICK-PREFETCH: Start loading a ticked sample before "Open selected"

**Goal.** The owner asked (relayed by another session, to be confirmed by the owner) that a sample starts loading in the background as soon as it is ticked in the "Choose samples" dialog (`mzlab/web/wiff-choose.js`), so that "Open selected" opens instantly. Default-ticked samples start when the dialog opens; unticking cancels or discards the work; the X (or Esc) cancels everything and frees memory; "Open selected" reuses what is loaded (no second read). The dialog is never blocked (a small per-row progress mark through `lang/` keys). Light: sequential or at most 2 parallel loads, no new dependency, files stay on the computer.

**Read first.** `docs/agenti/wiff-sample-choice.md`, `docs/agenti/wiff-range-copy.md`, `docs/agenti/native-reading.md`, `wiffToMzml` in `mzlab/web/shell.js`.

**Done means.** (1) The loading logic is in Rust/wasm (the reader and the range copy), JS only triggers and draws. (2) An e2e ticks a sample and sees loading start before Open, and sees untick and X cancel it. (3) Memory limits of `memory-limits.md` still hold with several samples ticked. (4) Parity: the app route does the same.

**Out of scope.** Changing the dialog layout, the default ticks or the listing.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
