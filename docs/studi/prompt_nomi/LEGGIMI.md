# Build prompts: short terminal names

Study: [`../NOMI_TERMINALE.md`](../NOMI_TERMINALE.md) (inventory, grammar §3, rename table §4, decisions §5). The chat title starts with «mzLab».

| Package | What | Zone (files) | After / collides with | Prompt |
|---|---|---|---|---|
| N01 | `channels`, `samples`, `peak-edges`, `flow run --events`, `ROLES.json` | `crates/mzlab-cli/src/{lib.rs,canali_cmd.rs,canali_tests.rs,campioni_cmd.rs,campioni_cmd_tests.rs,peaks_edges_cmd.rs,peaks_edges_cmd_tests.rs,flow_run.rs,flow_live_tests.rs,flow_mzrun.rs,tests.rs}`, `crates/mzlab-core/src/flow/line.rs`, its lines of `docs/agenti/{cli.md,campioni.md,canali-dati.md,dati-integrazione.md}` | none | [01](01-commands.md) |
| N02 | English flags of `msn-graph`, `msn-lib`, `msn-rules` (`sum`) | `crates/mzlab-cli/src/{msn_graph.rs,msn_uscite.rs,msn_lib.rs,msn_rules.rs,msn_rules/,msn_graph_compare.rs,msn_*_tests.rs,msn_figura_tests.rs}`, `docs/agenti/msn-*.md`, `msnlib.md`, its lines of `cli.md` | none | [02](02-msn-flags.md) |
| N03 | one verb for the golden variables, `MZLAB_APP_DIR`, `verifica.py --mostra-ok` | the test files that read the variables of §4 (not `msn_figura_tests.rs`: N02 owns it), `crates/mzlab-desktop/{examples/serve.rs,tests/e2e_shim*.py}`, `tools/verifica.py` (argparse only), the docs lines naming them | none | [03](03-variables.md) |
| N04 | `spectra.msn_graph@2` English params + `msngrafo/2` | GATED on D1 (study §5): runs only after the COO records Federico's yes | after N02; owns `steps-cli-1.json` | [04](04-msn-graph-params.md) |
| N05 | grammar in `cli.md` + guard test | `docs/agenti/cli.md` (new section), `crates/mzlab-cli/src/name_rule_tests.rs`, the `msn-rules` line of HELP in `lib.rs` | after N01, N02, N03 (and N04 if it runs) | [05](05-grammar-guard.md) |

Waves: **1** = N01 + N02 + N03 in parallel (disjoint files; `cli.md` only on their own lines). **2** = N04 if D1 is yes. **3** = N05.
No wave-1 package touches `docs/schemi/steps-*.json`, `F.liveJobs` or the build scripts; N04 is the only owner of `steps-cli-1.json`.

Rules for all:
- AGENTS.md applies (branch `claude/<topic>` from `origin/main`, PR ready for review, never merged from the chat, `python3 tools/pre_pr.py` before pushing, one message ≤ 5 lines to the COO with the 40-char SHA at green CI).
- Every old name stays accepted on input, is never printed except one line `old names still accepted: …` at the end of the command's `--help`, and the alias table sits next to the parser (one `const` slice of `(old, new)` pairs per file).
- No output byte changes for the same input: an alias test per renamed name runs old and new spelling and asserts identical stdout and exit code; `git diff --stat origin/main -- tests/golden docs/schemi` is empty (N04 excepted, by design).
- Parity: the CLI is native only; anything the site and the app share (`mzlab_core::flow::line`) changes in Rust once, so both see the same names.
- Docs: rewrite the line that describes the name (no «now», no «was»); generated files only with their tool.
