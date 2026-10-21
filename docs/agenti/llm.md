# LLM: catalogo degli strumenti (`crates/mzlab-llm`)
Studio e piano a PR: `docs/studi/llm/README.md`. Il crate `mzlab-llm` (membro del workspace, dipende solo da `mzlab-core`, nessuna rete, nessuna chiave) è usato da `mzlab-cli llm`; nessun `.wasm` lo include.

## Catalogo `strumenti/1`
- `mzlab_llm::tools::catalog(&[Step], &[UiTool], &dyn Fn(&str)->bool) -> Vec<Tool>` e `catalog_json(..) -> String`: 15 meta-strumenti fissi (`flow_list_steps`, `flow_describe_step`, `flow_get_plan`, `flow_check`, `flow_propose_plan`, `flow_add_step`, `flow_set_param`, `flow_remove_step`, `flow_run`, `flow_status`, `flow_read_result`, `data_summary`, `chart_make`, `code_export`, `ui_show`), ognuno con `class` (`read|propose|edit|run|ui`) e `input_schema` JSON Schema 2020-12.
- Gli argomenti sono generati dal registro `flow::steps()`: `flow_add_step.step` e `flow_describe_step.step` = `enum` di tutti gli `id@version`; `flow_set_param` = `oneOf` con un ramo per passo (`step` const, `name` = `enum` dei nomi di `--set`: parametri comuni, parametri di ogni algoritmo, `algorithm`, `preset`; `value` = schema del parametro, `anyOf` se lo stesso nome ha schemi diversi tra algoritmi); `flow_list_steps` porta `x-executable-native` (id con esecutore nativo, dalla funzione passata).
- `registry_sha256` = SHA-256 di `registry_json(steps)`. `UI_TOOLS` è l'elenco chiuso di `ui_show` (`show_xic` → `spec.xic`, `show_spectrum_at_rt` → `chrom.specAtRt`, `show_row` → `table.showRow`, `select_node` → `flow.selectNode`; gli ultimi due id si registrano nelle PR della pagina, elenco `NOT_YET` in `tests/test_strumenti.py`, che si accorcia soltanto).
- `mzlab-cli llm tools [--out FILE]` (`crates/mzlab-cli/src/llm.rs`) stampa il catalogo; `docs/schemi/strumenti-1.json` è la sua esportazione (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools` la riscrive).

## Prove
- `cargo test -p mzlab-cli --lib llm_tools` (`llm_tools_tests.rs`): ogni passo nell'enum, nomi di `flow_set_param` = nomi di `--set` (stesso giro di `flow_rule_tests.rs`), default dentro i limiti, file = esportazione, byte stabili.
- `tests/test_strumenti.py`: JSON Schema valido, id di `ui_show` registrati in `mzlab/web`, nessuna parola italiana.
