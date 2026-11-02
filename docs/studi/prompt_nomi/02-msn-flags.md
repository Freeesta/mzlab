# N02: English flags of `msn-graph`, `msn-lib`, `msn-rules`

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, `docs/agenti/msn-grafo.md` §3 and `docs/studi/NOMI_TERMINALE.md` §3–4, §6. Branch `claude/msn-flag-names` from `origin/main`. English only.

**Goal.** The MSn commands take English flags; old Italian flags stay as aliases; no output byte changes.

**Do.** Apply exactly the `msn-graph`, `msn-lib` and `msn-rules` rows of study §4 (flags, the values `tree|radial` and `all`, subcommand `msn-rules sum`). One `const ALIASES: &[(&str, &str)]` per file, resolved before the `match`, so the match arms carry the new names only. Help texts show the new flags; next to a flag that has a step parameter, the parameter name in brackets (`--low-mz N (param mz_basso)`) until N04. Last help line: `old names still accepted: …`. Error messages name the new flag. `MZLAB_UPDATE_FIGURE` (alias `MZLAB_UPDATE_FIGURA`) in `msn_figura_tests.rs`.

**Do not touch.** The step descriptors and their parameter names (`flow_msngraph.rs`, `steps-cli-1.json`: N04), the `msngrafo/1` keys (`opzioni`, `strutture`, `rotture`, `regole`, `interpretazione`, `vista`), the route query keys of `msngrafo::rotta`, `lib.rs` (N01/N05), the Python and e2e tests that call old flags (`tests/test_msngrafo_wasm.py`, `tests/test_msnfigura_wasm.py`, `tests_e2e/e2e_msn_figura.py`, `tools/convalida_regole.py`): they are the old-name tests.

**Tests.** A table-driven Rust test per file: every alias pair gives the same stdout bytes and exit code on the synthetic MSn fixtures (`msn_graph_tests.rs`, `msn_lib_tests.rs`, `msn_rules/tests.rs`, `msn_figura_tests.rs`); a value-alias case (`--view radial` = `--vista radiale`). `cargo test -p mzlab-cli`, `crates/mzlab-desktop/tests/parita_msn.rs` green, `git diff --stat origin/main -- tests/golden docs/schemi tests_e2e/fixtures` empty, `python3 tools/pre_pr.py`.

**Docs.** Rewrite the flag lines of `msn-grafo.md`, `msn-figura.md`, `msn-interpreta.md`, `msn-grafo-strutture.md`, `msn-manuale.md`, `msnlib.md` and the `msn-graph`/`msn-lib` lines of `cli.md`.

**PR.** Title «msn CLI: English flags with aliases». Body starts with `VISIBLE: no`, has the old → new table, says outputs are unchanged and the step parameters wait for N04.
