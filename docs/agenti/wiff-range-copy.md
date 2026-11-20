# Range copy of a `.wiff.scan` into OPFS (PR 5 of the wiff chooser plan)

Plan, dialog and `--sample`: `wiff-sample-choice.md`.

## What it does
- When the chooser leaves samples out, the page copies to OPFS only the byte ranges its chosen samples need, not the whole `.wiff.scan`. The `.wiff` (container, small) is always copied whole; the skipped sections stay zeros in the OPFS file (`truncate(size)` after the ranges), so every offset the reader uses is unchanged.
- One planner in Rust, `mzlab_core::readers::ranges::plan` (`Listing.sections` from `wiff/listing.rs`: id, start, end of each section, from the container and the `Idx` index, the data file is never read): the ranges are the head before the first section, the chosen sections (a section runs to the start of the next one, the last to the indexed end) and the tail after the indexed part (sections the index lacks, padding); touching ranges are joined. An id the file does not have is `err.sample.unknown`. A reader with no section positions gives every member whole.
- Same bytes from three routes: engine `.wasm` `sample_ranges(names, sizes, read, text)` (worker op `natplan`, page `nativePlan`), CLI `mzlab-cli sample select FILE --sample LIST --ranges [--json]` (schema `sample-select/1` plus `bytes`, `total`, `tag`, `members[].ranges`; exit 0, 2, 4, 5), and the app. Test `tests/test_sample_ranges_parita.py` (Node wasm = CLI bytes, errors, one sample leaves the rest out).
- JS only wires I/O: `shell.js` asks the plan for a partial choice and puts `ranges` in `mzlabNat`; `rust-nativo.js` `copyMember` writes a member with ranges through `putSparse` (`archivio.js`) and takes no fingerprints from the copy. If the plan fails the whole pair is copied and a note says so (`pick.rangeFailed`).

## Proof
`crates/mzlab-cli/src/sample_ranges_tests.rs`: a `.wiff.scan` that holds only the planned ranges (zeros elsewhere, synthetic `tools/wiff_grande.py`) converts the chosen samples to the bytes of the whole pair, except the two identity lines (below).

## Not here (owner)
- Q4, identity of a partial copy: the mzML `sourceFile` SHA-1 (and the `fileChecksum` over it) is taken over the bytes given, so a partial copy gives other values than the whole pair; the test asserts exactly those two lines differ, nothing else. How a partial copy is identified is not decided here. Meanwhile the engine hashes the sparse file when it indexes it.
- The app reads files in place and copies nothing, so it has no OPFS range copy; it only prints the same plan (`--ranges`). Technical limit, not a gap.
- Browser quota: whether a sparse OPFS file is counted by its size or by the bytes written depends on the browser; not measured (no browser here), `ensureRoom` counts the bytes written.
- Screenshots: no UI change (the dialog is unchanged), so none. The owner's Analyst «Open Data File» screenshot is still pending.
