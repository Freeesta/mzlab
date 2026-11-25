Read docs/coda/BRIEF.md.

# FILE-METHOD: Method parameters of enhanced-product-ion scans, and a method check in file items

**Goal.** `file method` prints CE, CES, DP and the range segments of enhanced-product-ion scans from the `.wiff` method stream, the converter writes them as cvParams in the mzML, and `file items` flags a sample whose name carries an m/z different from the precursor of its MS2 method. This row replaces the earlier METHOD-CHECK.

**Why.** Method parameters do not reach the mzML (no CES, CE only as one value). A file named for one mass had another precursor and it was found only after acquisition. `ce_spread` of `file items` is null with a single CE.

**Read first.** `docs/agenti/method.md` (`method/1`, `method_wiff.rs`), `docs/agenti/wiff-full-spec.md`, `docs/agenti/spectrum-rows.md`, `crates/mzlab-cli/src/items_cmd.rs`, `crates/mzlab-core/src/cespread.rs`, `crates/mzlab-cli/src/convert.rs`.

**Already on main (checked 25 Nov, re-check before coding).** The OLE method stream of `.wiff` is read into `method/1` for some fields. The lab has already decoded more of that stream; the decoding notes are private: ask the coordinator for counts and format facts ONLY, never paste private data. No file or name from the lab goes in this repo.

**Done means.** 1. Confirm on main which fields `method_wiff.rs` already gives. 2. Add CE, CES, DP and range segments, with a synthetic `.wiff` fixture (`tools/wiff_riferimento.py`). 3. `file method FILE` (`--json`, schema, exit codes, catalogs). 4. The converter writes cvParams; a round-trip test. 5. `file items` gains a `method_check` member: a number in the sample name that differs from the method precursor beyond a tolerance, plus the CE spread, fill time and Q1 resolution. The rule and tolerance are logged. Parity site=app=CLI.

**Out of scope.** Other vendors; changing existing mzML bytes of files without the new fields.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
