Read docs/coda/BRIEF.md.

# FILE-METHOD-FIELDS: Decode fill time and Q1 resolution of a `.wiff` method

**Goal.** `mzlab-cli file method` (`file_method/1`, `crates/mzlab-core/src/filemethod.rs`) lists fill time and Q1 resolution as `null` with reason `unknown` for `sciex-wiff`: nobody has shown which method-stream field holds them. Decode them and fill the two keys; write the fill time as the PSI-MS `ion injection time` cvParam (MS:1000927) in the mzML of the same sample.

**Needs first (owner).** Facts about the real files, counts and format facts only, never names or data: which field of the experiment header or the method stream holds fill time and Q1 resolution, in which unit, and how many samples and experiments carry each. Without them stop: the PR body says what is missing and the row stays.

**Read first.** `grep -n 'method' docs/agenti/INDICE.md`, `docs/agenti/method.md`, `crates/mzlab-core/src/methodinfo.rs` and `methodinfo/` (the reader of the method streams, the `damaged` list), `crates/mzlab-core/src/filemethod.rs` (`reason`), `tools/wiff_dispersione.py` (synthetic fixtures; extend them with the new fields), `crates/mzlab-core/src/mzmlwrite.rs` (`SpectrumExtra`).

**Done means.** `method/1` and `file_method/1` carry the values (schemas regenerated), unknown stays `null` with a reason, a damaged stream is counted, the mzML writes the cvParam in every build, a golden test on synthetic data, the wasm leg of the parity test (`tests/test_file_method_wasm.py`) fails in CI when its tools are missing, caps by the CI size step only.

**Out of scope.** Other vendors; changing any existing key.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
