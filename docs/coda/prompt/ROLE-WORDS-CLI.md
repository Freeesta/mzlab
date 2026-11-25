Read docs/coda/BRIEF.md.

# ROLE-WORDS-CLI: Role words on the command line and the `model` site op

**Goal.** A user-defined word list for the roles: `mzlab-cli sample table --role dark=dc,dk --role std=ref` makes the listed whole tokens of a file name give that role, and the same rule is a site op of the samples route (`model`, the name-pattern op that fills `samples/1` from a regex or a template).

**Why.** The roles-in-TP PR fixed the built-in words (compound dark words) and the roles from a file (`sample import`), but a lab with its own naming still has to edit the table by hand. The name rule was only half met there.

**Read first.** `crates/mzlab-core/src/samples/names.rs` (`DARK_WORDS`, the word tables), `positions.rs` (`role_word`, the evidence of a word), `crates/mzlab-core/src/samples/roles_pair.rs`, `docs/agenti/samples.md` (section `model`), `crates/mzlab-cli/src/samples_table.rs`. Start with `grep -n 'sample file roles' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** The `model` op exists in the engine (`samples(op="model")`) and in the app; the site has it only where `rotte` links it, and `rotte` and `hr` have almost no room. `sample table` takes no word list.

**Done means.** 1. One Rust function takes the extra words (role, whole tokens, case-insensitive) and the proposal logs each use with the word as evidence, exactly as the built-in words do. 2. CLI flag `--role ROLE=W1,W2` (repeatable) on `sample table` and `sample pair`, `--json` keys stable, a bad role is exit 2, the flags in the agent catalogs. 3. The site gets the same rule through the `model` op in a lazy module, never by growing `rotte`; a parity test compares site, app and CLI byte for byte, and its wasm leg FAILS in CI when the tools are missing. 4. A test that the entry point is findable (`find_tests.rs`).

**Out of scope.** Changing the built-in words; short ambiguous names (row ROLE-SHORT-NAMES).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
