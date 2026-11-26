Read docs/coda/BRIEF.md.

# SPECTRUM-COMPARE-SITE: a site card for comparing two MS2 scans

**Goal.** On the site, the user picks two MS2 spectra (a sample scan and a standard scan, or two windows) and sees the cosine, the matched ions with both intensities, the ions of each side the other lacks and the collision-energy warning, from the same `spectrum-compare/1` document the CLI prints.

**Why.** `spectrum compare` is reachable from the CLI, the app op `speccompare` and the `spectrum_compare` export of the `librerie` module, but no page calls it: the site half of the parity is an export without an interface.

**Read first.** `crates/mzlab-lib-wasm/src/lib.rs` (`spectrum_compare`), `crates/mzlab-lib/src/speccmp.rs`, `crates/mzlab-cli/src/spectrum_compare.rs` (how a side is built: the `rebin=nominal` answer of `/api/spectrum` plus the precursor and collision energies of the scans), `docs/agenti/wasm-caps.md` (`librerie` room), `.claude/rules/frontend.md`. Start with `grep -n 'spectrum' docs/agenti/INDICE.md`.

**Already on main (check before coding).** The function, the wasm export, the rpc op and the CLI command exist and are byte-for-byte tested. Only the interface is missing; JS builds the two sides and shows the document, no logic of its own.

**Done means.** 1. A card in the spectrum view that takes two selections and calls the export through the worker, in the app through the rpc op. 2. All text through `lang/` keys (the `warn.spectrum.*` and `err.spectrum.compare.*` keys exist). 3. An e2e test on a synthetic file with a known cosine. 4. The `librerie` size recorded in the PR body if the JS needs a new export.

**Out of scope.** Library search; MS2 of different precursors; changing the document.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
