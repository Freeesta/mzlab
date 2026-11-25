Read docs/coda/BRIEF.md.

# TP-LINK-NOTES: `tp.link` pairing notes in a result document

**Goal.** The pairing choices of `tp.link` (samples left out, roles proposed from names, each with its evidence) are in a document the run keeps, as they are in the `pairing` key of the `tp.dig_lr` result and of `rete/1`.

**Why.** `tp.link` writes a binary `.mzfeat` whose bytes must not change, so its notes are only stderr lines; a run read later (`flow verify`, the page) cannot show them.

**Read first.** `crates/mzlab-cli/src/flow_link.rs`, `crates/mzlab-cli/src/flow_pairnotes.rs`, `crates/mzlab-core/src/samples/roles_pair.rs`, `docs/agenti/mzfeat.md`, `docs/agenti/rust-flow.md`. Start with `grep -n 'tp.link' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** `TpRoles.notes` holds the notes; the dig and network results carry `pairing`. Decide first where the notes can live without changing existing bytes: a sidecar next to the output, a log entry of `flowrun/1`, or an opt-in block in the table. Existing `.mzfeat` bytes must not change.

**Done means.** 1. The chosen carrier, with a schema or key list updated by its tool. 2. The same bytes from the site, the app and the CLI, compared by a test (the wasm leg FAILS in CI when the tools are missing). 3. `flow run --json` or `flow verify` shows where the notes are.

**Out of scope.** The notes of `tp.dig_lr` and `tp.network` (done).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
