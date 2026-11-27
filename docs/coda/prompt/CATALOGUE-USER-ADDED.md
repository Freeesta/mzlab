Read docs/coda/BRIEF.md.

# CATALOGUE-USER-ADDED: compounds the user adds to the built-in list

**Goal.** `mzlab-cli mol find QUERY --user FILE` searches the built-in catalogue and the molecules the user saved (`molecules/1` `user` level, `levels.rs`), and the search box of the pages shows them together with the origin of each hit.

**Why.** The built-in list is environmental only; a lab adds its own compounds and must find them the same way, in the site, the app and the CLI.

**Read first.** `grep -n 'molecules' docs/agenti/INDICE.md`, then `docs/agenti/molecules.md` (levels, `load`, user storage), `crates/mzlab-cli/src/mol_find_cmd.rs`, `crates/mzlab-desktop/tests/parity_molecules.rs`.

**Already on main (checked 27 Nov, re-check before coding).** `mol find QUERY [--db FILE]` over the built-in catalogue or a CSV; the engine merges levels (catalogue < imported < user); the page keeps user molecules in IndexedDB, the app in `molecules.json`.

**Done means.** `--user FILE` reads a `molecules/1` document or a CSV (`from_csv`); every hit says its level; exit codes as `mol find`; the parity test covers a user molecule in the core, the app, the CLI and the `.wasm`; synthetic molecules only.

**Out of scope.** Editing the built-in list.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
