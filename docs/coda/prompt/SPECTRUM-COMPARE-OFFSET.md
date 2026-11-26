Read docs/coda/BRIEF.md.

# SPECTRUM-COMPARE-OFFSET: spectrum compare with an m/z offset per side

**Goal.** `spectrum compare` takes `--a-mz-offset` and `--b-mz-offset` (`DA` or `auto:MZ[@RT]`, as `spectrum get --bin nominal --mz-offset`): each side's offset is taken off before its nominal bins, so a sample and a standard measured with different calibrations land in the same bins.

**Why.** On unit-resolution profile data a fragment sits between two integers (106.7 against 107.0); without the offset the two spectra split the same ion into different bins and the cosine drops for a reason that is not chemistry.

**Read first.** `crates/mzlab-cli/src/spectrum_compare.rs` (`side`, `pick`), `crates/mzlab-cli/src/spectrum_bins.rs` and `crates/mzlab-cli/src/mz_offset.rs` (how `spectrum get` passes the offset to the route), `crates/mzlab-lib/src/speccmp.rs`, `docs/agenti/spectral-match.md` section 6. Start with `grep -n 'offset' docs/agenti/INDICE.md`.

**Already on main (check before coding).** `spectrum compare` exists (`spectrum-compare/1`); the route answer of each side is the input of `speccmp::compare_text`, so the offset belongs in the CLI query only, and the site and the app need only to pass the same route query. Parity test: `crates/mzlab-desktop/tests/parity_spectrum_compare.rs`.

**Done means.** 1. Both flags, with the offset used (and how `auto` found it) in each side's block of the document and on stderr. 2. A synthetic pair whose second side is shifted by a known offset: the cosine without the flag is lower, with it equals the unshifted one. 3. Schema, help, catalog and `find_tests.rs` row updated; parity unchanged or extended. 4. Exit codes pinned in a test.

**Out of scope.** Library search; MS2 of different precursors; a high-resolution bin rule (row NOMINAL-BIN-HR).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
