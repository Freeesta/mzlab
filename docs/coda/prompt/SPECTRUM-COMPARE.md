Read docs/coda/BRIEF.md.

# SPECTRUM-COMPARE: spectrum compare A B: cosine of two MS2 spectra

**Goal.** `spectrum compare A B` returns the cosine similarity of two MS2 spectra at the same collision energy (a sample against a standard), with the matched ions listed.

**Why.** An agent now compares fragment lists by hand and cannot say how much of the standard is explained.

**Read first.** `docs/agenti/spectral-match.md` (the engine already scores spectra), `crates/mzlab-cli/src/msn_graph_compare.rs` (graph compare, a different command), the merged `--bin nominal` (`nominalbin.rs`), `crates/mzlab-cli/src/spectrum_cmd.rs`. Start with `grep -n 'spectral matching' docs/agenti/INDICE.md`, then `grep -rn 'fn cosine' crates --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** A multi-library spectral matching engine exists (`libmatch/1`); reuse its cosine, do not write another. `spectrum compare` is a new subcommand name: at most 10 characters (`compare` is 7).

**Done means.** 1. Inputs: two spectrum selections (file, rt or scan, precursor); both binned at nominal mass. 2. Output: cosine, matched ions with both intensities, unmatched ions of each side, the CE of each and a warning when they differ; tolerance as a `--set`. 3. Synthetic spectra with a known cosine. Parity site=app=CLI.

**Out of scope.** Library search; MS2 of different precursors.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
