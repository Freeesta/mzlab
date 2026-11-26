Read docs/coda/BRIEF.md.

# ITEMS-BACKGROUND-SAMPLE: `--sample ID` for `file items` and `trace background`

**Goal.** A user of a multi-sample `.wiff` reaches any sample with `file items FILE --sample ID` and `trace background FILE --sample ID [--blank FILE --blank-sample ID]`, as `dad ms-at|delay|peaks` and `trace chrom|xic` already do; an id the file does not have is exit 5 naming the ids present.

**Why.** FIRST-SAMPLE-HINTS added a stderr hint to these two commands (`note.sample.firstOnly`) that can only name `file convert --sample LIST`, because they read the first sample by design. With the flag the hint becomes `note.sample.firstChoose` (the one `dad` uses) and the detour through a converted mzML goes away.

**Read first.** `grep -n 'sample' docs/agenti/INDICE.md`, then `docs/agenti/wiff-sample-choice.md`; `crates/mzlab-cli/src/items_cmd.rs` (`open_all`, `build_index_of(.., "", ..)`), `background_cmd.rs`, `dad_ms.rs` (`sample_at`, the `NAME@ID` label), `sample_select.rs` (`members::first_hint`).

**Done means.** 1. `--sample ID` on both commands, labels as `dad_ms.rs` makes them, bytes without the flag unchanged (the first sample). 2. The hint of both switches to `note.sample.firstChoose` and goes away once `--sample` is given; a test of the hint and of the exit codes (0, 2, 5). 3. `items/1` names the sample read (its `sample` member, schema `items-1.json`) only if that adds a key opt-in; else the choice is on stderr. 4. Parity: check whether the site routes of items and background take a sample; if so a test compares CLI and wasm byte for byte (wasm leg fails, not skips, in CI), else the PR body states the limit. 5. Catalogs and help snapshot regenerated with their tools.

**Out of scope.** The label rule; what the two commands compute; `peak table` and `peak split`, which read every sample.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
