Read docs/coda/BRIEF.md.

# FIRST-SAMPLE-HINTS: `quant targets --sample` and the first-sample hint

**Goal.** A user of a multi-sample `.wiff` reaches the right sample: `quant targets` gains `--sample LIST` as `features` has it, and `items`, `background`, `dad_ms` and `peak` (one sample by design) hint at `--sample` and `sample list`.

**Why.** #3717 made `features` name columns by sample and added a note to `quant targets` that it uses the first sample. A poorly signalled feature needs a hint that names the command, plus a test of the hint.

**Read first.** `grep -n 'quant targets' docs/agenti/INDICE.md`, then `docs/agenti/quant-targets.md` and `wiff-sample-choice.md`; `crates/mzlab-cli/src/quant_targets_cmd.rs` (`first_sample_notes`, `note.quant.firstSample`); `sample_select.rs`; `sample_label.rs`.

**Done means.** (1) `quant targets --sample LIST`: members `FILE@ID`, labels from `sample_label.rs`, `quant-targets-plan/1` and its schema, agent catalogs (`commands --json` stays under its guard), parity site/app/CLI with a byte-for-byte test whose wasm leg fails, not skips, in CI. (2) For the other four commands, a stderr hint (a lang key, en plus contesto) naming `--sample`/`sample list` where the file holds several samples, with a test of the hint and of the exit code. (3) Every automatic choice is logged.

**Out of scope.** Column labels on collision (row COLLISION-LABEL); changing what the four commands compute; real data.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
