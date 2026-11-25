Read docs/coda/BRIEF.md.

# SAMPLE-PAIR-SITE: `sample pair` as a site op

**Goal.** The check `mzlab-cli sample pair` (which files of a table a samples document leaves out, which get a role from their name, with the evidence) exists on the site too, as an op that returns the same `sample-pair/1` bytes.

**Why.** The CLI route was added with the roles-in-TP PR; the site gets the notes only inside the `pairing` key of a dig or network result, after a run. A user of the page cannot preview the pairing.

**Read first.** `crates/mzlab-cli/src/samples_pair.rs`, `crates/mzlab-core/src/samples/roles_pair.rs` (`order_for_roles`, `notes_json`, `with_pairing`), `docs/schemi/sample-pair-1.json`, `docs/agenti/samples.md`, `docs/agenti/wasm-caps.md`. Start with `grep -n 'sample file roles' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** The Rust function is shared by the CLI and by `tp_dig_inputs`; only the wasm export and a page entry are missing. `rotte` is nearly full and `hr` has a headroom under 0.1 %: the export goes into a lazy module.

**Done means.** 1. A wasm export in a module with room (record module, raw bytes before and after and % in the PR body). 2. A page control or a flow check message that shows the notes before the run. 3. A parity test: the site op equals `mzlab-cli sample pair --json` byte for byte; its wasm leg FAILS in CI when the tools are missing. 4. Text through `lang/` keys.

**Out of scope.** Changing the exit codes or the schema of the CLI route.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
