# N01: short English command names (`channels`, `samples`, `peak-edges`, `flow run --events`)

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, `docs/agenti/cli.md` and `docs/studi/NOMI_TERMINALE.md` §3–4. Branch `claude/cli-short-names` from `origin/main`. English only in code, tests, commit and PR.

**Goal.** The commands typed most often use short English names; every old name keeps working.

**Do.**
1. `lib.rs`: `channels` (alias `canali`), `samples` (alias `campioni`), `peak-edges` (alias `peaks-edges`) in `execute`/`PLAIN`/`run_plain`; HELP lists the new names only, plus one last line `old names still accepted: canali, campioni, peaks-edges, nome-gruppo, variabili, --canale, --jsonl` (exactly the aliases this package adds).
2. `canali_cmd.rs`: `--channel` (alias `--canale`); help with the new names.
3. `campioni_cmd.rs`: subcommands `vars` (alias `variabili`) and `group` (alias `nome-gruppo`).
4. `peaks_edges_cmd.rs`: `--set` accepts `area_noise, edge_frac, sn_min, min_half` besides the camelCase names (same field); help shows snake_case.
5. `flow_run.rs`: `--events` (alias `--jsonl`), help line rewritten; `mzlab_core::flow::line` (`line.rs:249`) accepts `--events` too (the pasted line of «Run on CLI», site and app).
6. `flow_mzrun.rs` and HELP: placeholder `--roles ROLES.json`.

**Do not touch.** `msn_*` files and the `msn-rules` HELP line (N02, N05), schemas, `steps-*.json`, lang catalogs, the output keys (`canali/1`, `nomivar/1` stay).

**Tests.** For each alias: old and new spelling give the same stdout bytes and exit code (`canali_tests.rs`, `campioni_cmd_tests.rs`, `peaks_edges_cmd_tests.rs`, `flow_live_tests.rs`, `tests.rs` for dispatch and `--help`); a `line.rs` test that `flow run p.json --out o --events` parses like `--jsonl`. `cargo test -p mzlab-cli -p mzlab-core`, `flow_rule_tests.rs` green, `git diff --stat origin/main -- tests/golden docs/schemi` empty, `python3 tools/pre_pr.py`.

**Docs.** Rewrite the lines of `cli.md`, `campioni.md`, `canali-dati.md`, `dati-integrazione.md` that name these commands (new names, one mention of the alias).

**PR.** Title «CLI: short English command names with aliases». Body starts with `VISIBLE: no` (terminal only), lists old → new, says no output byte changed and that the site/app share `line.rs`.
