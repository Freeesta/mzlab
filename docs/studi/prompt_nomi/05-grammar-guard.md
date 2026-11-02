# N05: naming grammar in `cli.md` and a guard test

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, `docs/agenti/cli.md` and `docs/studi/NOMI_TERMINALE.md` §3. Branch `claude/name-grammar` from `origin/main` after N01, N02, N03 (and N04 if it ran) are merged.

**Do.**
1. `docs/agenti/cli.md`: a section «Names» with the 7 rules of study §3 (one line each, ≤ 600 chars) and the shared abbreviation list.
2. `lib.rs` HELP: the `msn-rules` line says `sum` (N02 made it work).
3. `crates/mzlab-cli/src/name_rule_tests.rs`: (a) every command, subcommand and flag in the `--help` text of every command matches `^--?[a-z][a-z0-9]*(-[a-z0-9]+)*$` and the length targets; (b) every step id of `flow::steps()` matches `^[a-z]+\.[a-z0-9]+(_[a-z0-9]+)*$`, ≤ 20 chars; every parameter (algorithms too) snake_case ≤ 20 chars; (c) no name contains a word of a short deny list of Italian roots met in §2 (`basso, prove, toll, perdita, regole, rotture, legami, vista, figura, canal, campion, somma, tutti`). Names that break a rule today sit in one `const KNOWN: &[&str]` with the study line that keeps them (e.g. `explain-missing`, `putative_library_level3`, the `spectra.msn_graph` params if D1 is no): the list only shrinks, like `tests/test_regola_rust.py`.
4. One line in the INDICE/`cli.md` check-list «Modulo nuovo»: (7) names follow «Names», `name_rule_tests.rs` green.

**Tests.** `cargo test -p mzlab-cli name_rule`, a negative case per rule (a fake flag `--troppo-lungo-davvero` fails), `python3 tools/pre_pr.py`.

**PR.** Title «CLI: naming grammar and guard test». Body starts with `VISIBLE: no`.
