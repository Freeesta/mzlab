# mzlab-cli Audit Report (Read-Only)

**Target:** `Freeesta/mzlab` (`crates/mzlab-cli` at `origin/main` commit `9ab928140`)  
**Scope:** Help texts, user-visible strings, command structure, AI-agent ergonomics (`--json`, exit codes, streams, determinism, schema discovery), execution speed & typical workflows, and documentation parity (`docs/agenti/cli.md`, `cli-agenti.md`, `for-agents.md`).  
**Verification basis:** Source inspection in `crates/mzlab-cli/src/`, `crates/mzlab-core/src/`, `mzlab/web/lang/en/errori-*.js`, `docs/schemi/`, and live runs of the debug build (`cargo build -p mzlab-cli`) against `mzlab/web/esempi/`.

---

## Executive Summary

`mzlab-cli` already has strong scientific building blocks—byte-for-byte parity with the web/WASM engine, deterministic `flow run` caching (`prov/1`), and `steps --json` JSON Schemas—but it grew bottom-up by wrapping internal web/WASM routes (`api/canali`, `api/campioni/*`, `api/manint`, `mol-*`) one-to-one rather than as a coherent CLI.

As a result, both humans and AI agents hit sharp friction on almost every non-trivial task:
1. **Italian leaks throughout JSON schemas, field names, and option values**, breaking `CLAUDE.md`'s English-only rule and forcing agents to guess mixed bilingual keys.
2. **Broken discovery and `--help` handling**: 18+ subcommands/commands exit `2`, treat `--help` as a file/sample name, or print the wrong help block.
3. **Missing CLI primitives break closed-loop workflows**: no command extracts an XIC or MRM/TIC trace from a file for the 5 `peak-*` trace commands; no command builds/validates `disegno/1` for `table.stats` / `table.drift`; and `flow record` fails out of the box on any standard `cargo build`/`install` binary and on `samples table` output.
4. **Inconsistent `--json`, error objects, exit codes, and stream discipline**: only 11 command paths accept `--json`, `Outcome::fail_error` collapses file/data/read failures into exit `2`, and error text mixes browser UI hints (`reload the page`) with unrendered `err.cli.*` keys.
5. **Repeated uncached indexing and bloated outputs**: standalone inspection commands rebuild and discard `.mzidx` on every call (~560 ms each on a 7.3 MB mzML), `llm inspect` + `llm params` double-index `DIR` sequentially (~26 s on `mzlab/web/esempi/`), and `steps --json` / `llm tools` emit 140–160 KB without compact summaries.

---

## 1. Top 10 Problems (Ranked by Impact)

### 1. Pervasive Italian JSON keys, schema identifiers, and option values
Violates `CLAUDE.md` ("English only, no Italian anywhere in the repo... saved formats are all English") and forces AI agents to handle mixed English/Italian vocabularies.

- **Evidence:**
  - `printf "HRMS_t000.mzML\nHRMS_t005.mzML\n" | mzlab-cli samples vars` (`crates/mzlab-cli/src/samples_cmd.rs:57-103`):
    `{"comune":"HRMS*","files":[{"name":"HRMS_t000.mzML","tokens":[{"classe":"word","fine":4,"inizio":0,"testo":"HRMS"},...]}],"schema":"nomivar/1","variabili":[{"copertura":1.0000,"id":"time","punteggio":1.0000,"senza":[],"tipo":"ordered","valori":[{"chiave":"0","etichetta":"t000","file":[0]},...]}]}`
  - `printf "HRMS_t000.mzML\nHRMS_t005.mzML\n" | mzlab-cli samples table` (`crates/mzlab-cli/src/samples_cmd.rs:106-126`, `crates/mzlab-core/src/samples/json.rs:16-59`):
    `{"campioni":[{"composto":"","concentrazione":null,"file":"HRMS_t000.mzML","istd":false,"note":"","ordine_iniezione":1,"replica":1,"serie":"","sha256":"","tempo":0,"tempo_confermato":false,"tempo_min":0,"tipo":"t0","tipo_bianco":null,"unita":null,"unita_tempo":"min"},...],"schema":"campioni/2"}`
  - `mzlab-cli samples group HRMS_t000.mzML HRMS_t005.mzML` (`crates/mzlab-cli/src/samples_cmd.rs:376-379`):
    `{"nome":"HRMS"}`
  - `mzlab-cli channels mzlab/web/esempi/HRMS_t000.mzML` (`crates/mzlab-core/src/channels/output.rs:36-76`):
    `{"v":"canali/1","tipo":{"proposto":"lc-ms2-mirato","motivo":"few_precursors"},"canali":[{"id":"tic","fam":"tic","gen":null,"liv":0,"pol":0,"sigla":null,"et":"TIC","omo":"tic","n":749,"rt0":4.9985,"rt1":6.398,"spettri":false},...]}`
  - `mzlab-cli msn-graph mzlab/web/esempi/HRMS_t000.mzML` (`crates/mzlab-cli/src/msn_graph.rs:20-67,274-362`):
    `{"formato":"msngrafo/2","motore":"1.0.0","sorgente":{"nome":"HRMS_t000.mzML","sha256_idx":"..."},"opzioni":{...},"polarita":1,"radici":[...],"ione":null,...}`
    `msn-graph --help` also documents Italian output keys (`strutture`, `strutture_tutti`, `interpretazione`, `regole`, `regola`, `rotture`, `vista`, `convalidata`) and `--node tutto`.
  - Additional Italian schemas/keys/placeholders: `corsa/1` & `corsa.json` (`flow_report.rs:35`), `errore/1` (`flow_json.rs:81`), `ispezione/1` & `proposta-param/1` & `"profile":"ricerca"` (`llm_facts.rs:94,129`), `contesto/1` (`llm_ctx.rs:60`), `disegno/1` (`condizioni`, `confronti`, `fattori`, `lotti`, `riferimento`, `crates/mzlab-core/src/drawing/json.rs:24-75`), and `tipo_bianco` inside `mzrun/1` (`crates/mzlab-core/src/mzrun/build.rs:105`).
  - Further Italian keys in MSn/conversion tools: `msnconfronto/1` (`nodi_isolati`, `archi`, `formule`, `entropia_ms2`, `scarto_ppm`, `convalidata`, `msn_graph_compare.rs:190-286`), `msn-rules sum` (`strato`, `registri`, `copertura`, `soglie`, `saltati`, `esito:"non_misurata"|"priorita_3"`, `msn_rules/sum.rs:97-196`), `msn-rules --decoy SEME` (`msn_rules.rs:20`), `conv/1` (`parziale`, `perdita`, `dichiarate`, `lette`, `unita:"cicli"`, `convert_report.rs:134-178`), and `metodo/1` (`avvisi`, `chiave`, `dove`, `convert_report.rs:113-118`).

### 2. No CLI command extracts an XIC, TIC/BPC, SRM trace, or MS spectrum—leaving all 5 `peak-*` trace commands stranded
`peak-auto`, `peak-auto-lr`, `peak-edges`, `peak-area`, and `peak-window` all require `TRACE.csv|-` (`two columns, RT (min) and intensity`), yet `mzlab-cli` cannot extract MS chromatograms or XICs from an LC-MS file.

- **Evidence:**
  - `mzlab-cli channels mzlab/web/esempi/MRM_std_0.6ppm.mzML` lists `"id":"tic"`, `"id":"srm:364.1>194.1@20"`, `"id":"srm:364.1>152.1@20"`, `"id":"uv:twc"`.
  - Running `mzlab-cli channels mzlab/web/esempi/MRM_std_0.6ppm.mzML --channel "srm:364.1>194.1@20"` fails (`exit 2`, `crates/mzlab-cli/src/channels_cmd.rs:143-149`):
    `error: Channel srm:364.1>194.1@20 does not exist in this file. (err.canali.unknown)`
    because `--channel` only calls `mzlab_core::channels_uv::trace` (UV/analog on mzML only) and refuses all MS channels and all non-mzML files (`channels_cmd.rs:134-136`).
  - `mzlab_core::routes::Routes` already implements `/api/xic`, `/api/chrom`, `/api/spectrum`, `/api/scanlist`, and `/api/scaninfo` (`crates/mzlab-core/src/routes.rs:99-119`), but none are exposed in `crates/mzlab-cli`.

### 3. Documented `flow record` workflow fails out of the box on both `samples table` and any `cargo build` / `cargo install` binary
Following `docs/agenti/cli-agenti.md:28` (`flow record RUN --roles ROLES.json`) fails in two ways on real runs:

- **Evidence:**
  - `printf "HRMS_t000.mzML\nHRMS_t005.mzML\n" | mzlab-cli samples table > roles.json` sets `"sha256":""` for every row (`crates/mzlab-cli/src/samples_cmd.rs:121`).
  - `mzlab-cli flow record /tmp/mzlab-audit-test/run1 --roles roles.json --out mzrun.json` fails (`exit 2`, `crates/mzlab-core/src/mzrun/build.rs:92-95`):
    `error: The run record could not be built: role (HRMS_t000.mzML). (err.mzrun.build)`
  - After manually injecting file SHA-256s into `roles_sha.json`, `mzlab-cli flow record` fails again (`exit 2`, `crates/mzlab-core/src/mzrun/build.rs:43-47`):
    `error: The run record could not be built: engine (commit). (err.mzrun.build)`
    because `crates/mzlab-cli` has no `build.rs` setting `MZLAB_COMMIT` (`option_env!("MZLAB_COMMIT")` in `mzlab_core::flow::build_agent`). Only `tools/prepara_sito.py` sets it; `cargo build -p mzlab-cli` and `cargo install --path crates/mzlab-cli --profile cli` (`docs/agenti/for-agents.md:6`) omit `"commit"` from `prov.json` (which `flow_mzrun_tests.rs:16-23` works around by string-replacing `"commit"` into `prov.json`).

### 4. `--help` and `-h` fail, misparse, or return JSON data across 18+ commands and subcommands
- **Evidence (all reproduced against the built binary):**
  - `mzlab-cli samples group --help` -> `exit 0`, stdout `{"nome":"help"}` (`samples_cmd.rs:376-379`).
  - `mzlab-cli flow --help` / `mzlab-cli llm --help` -> `exit 2`, stderr prints `error: usage: ...` and `(mzlab-cli flow --help --help)` / `(mzlab-cli llm --help --help)` (`cmd/flow.rs:23-25`, `llm.rs:46`, `error_text.rs:164-170`).
  - `mzlab-cli flow command --help` and `mzlab-cli msn-rules sum --help` -> `exit 2`, `error: --help: No such file or directory (os error 2)` (`cmd/flow.rs:8`, `msn_rules/sum.rs:228`).
  - `mzlab-cli flow report --help`, `flow record --help`, `flow verify --help`, `llm context --help`, `llm inspect --help`, `llm params --help`, `llm eval --help`, `samples vars --help`, `samples table --help`, `insilico pack --help`, `msn-graph figure --help`, `msn-graph complete --help`, `msn-graph compare --help` -> all `exit 2` with `error: unknown option --help` or `unknown argument --help`.
  - `mzlab-cli normative listings /tmp/does_not_exist --help` -> ignores `--help` because `normative/mod.rs:189` strips all `--*` flags via `.filter(|a| !a.starts_with("--"))`; conversely `mzlab-cli normative bogus` exits `0` and prints `HELP` (`normative/mod.rs:111,219`).
  - Top-level `mzlab-cli --help` (`cmd/mod.rs:81-90`) opens with `mzlab-cli features: the feature table...` and `TAIL` options for `features` only; `mzlab-cli features --help` prints the full 35-command global table (`cmd/features.rs:94`).

### 5. Missing CLI routes and undiscoverable schemas for Flow inputs (`disegno/1`, `campioni/2`), plus mistranslated error `error.badDisegno`
- **Evidence:**
  - Steps `table.stats@1` and `table.drift@1` require `"design":{"type":"design"}` (`disegno/1`), and 7 steps require `"samples":{"type":"samples"}` (`campioni/2`).
  - `mzlab_core::drawing::route` (`proponi`, `valida`, `canonico`, `digest` in `crates/mzlab-core/src/drawing/mod.rs:248-263`) has no CLI command (a CLI parity gap per `CLAUDE.md:20-21`), and `steps --json` gives no schema or format description for `"type":"design"` or `"type":"samples"`.
  - Passing an invalid or English-keyed `design.json` to `mzlab_core::drawing::json::read` (`crates/mzlab-core/src/drawing/json.rs:82-84`) emits `error.badDisegno`, which `mzlab/web/lang/en/errori-a-m.js:73` mistranslates as:
    `"The drawing file is not valid or was made by another program."` (confusing experimental design with 2D molecule drawing).
  - Additionally, `flow run ... --json` prints unrendered `"error_key":"error.badMzfeat"` in stderr progress (`[1/1] table.stats (s) ... failed: error.badMzfeat`, `flow_run.rs:278`) and appends a trailing space on `"error: failed s; not run \n"` (`flow_out.rs:110-114`).

### 6. Contradictory error messages (`err.file.notAccepted` on `.mzML`), browser-only text, and missing CLI catalog keys
- **Evidence:**
  - `mzlab-cli samples list mzlab/web/esempi/HRMS_t000.mzML --json` (`samples_cmd.rs:171`) and `mzlab-cli convert mzlab/web/esempi/HRMS_t000.mzML --out /tmp/conv` both output:
    `error: File type not accepted: HRMS_t000.mzML. Accepted: .mzML, .mzXML, .cdf, .wiff together with its .wiff.scan, .raw, .d folders and .dam methods. Anything else: convert it to mzML with MSConvert first. (err.file.notAccepted)` (`mzlab/web/lang/en/errori-file.js:46`). Telling the user `.mzML` is accepted while rejecting `.mzML` is contradictory and confusing.
  - `error_text.rs:5-11` renders web UI instructions directly in the terminal: e.g. `error.badIndex` ("usually because the browser ran short of space..."), `err.flow.registry` ("reload the page"), `error.badRequest` ("reload the page and try again"), `err.flow.tablematch.files` ("open the files of the table again").
  - Conversely, `err.cli.usage`, `err.cli.file`, `err.cli.input.missing`, `err.cli.input.twice`, `err.cli.out`, `err.flow.step.native`, and `err.normative.*` are absent from `mzlab/web/lang/en/errori-*.js`, so `error_text::human_error` falls back to raw `key k=v` (`error_text.rs:131-133`), and `convert --report` logs raw `"cause":"err.file.notAccepted name=HRMS_t000.mzML"` (`convert.rs:327`).

### 7. Fragmented `--json` coverage, inconsistent schema keys (`schema`/`format`/`formato`/`v`/none), and missing `errore/1` errors
- **Evidence:**
  - Only **11** command paths accept `--json`: `flow run`, `flow check`, `steps`, `validate`, `dad`, `samples list`, `samples select`, `folder list`, `elements`, `pair`, and `manual-table` (which rejects `--json` when combined with `--set auto=true` or `--set shifts=true`: `manual_table_cmd.rs:102-104`).
  - Even among `--json` commands, error output is inconsistent:
    - `validate /tmp/does_not_exist_12345.mzML --json` -> `exit 2`, prints `[]` on stdout (`validate.rs:83-118`).
    - `folder list /tmp/does_not_exist_12345 --json` -> `exit 4`, empty stdout (`folder_cmd.rs:242-248`).
    - `pair mzlab/web/esempi/HRMS_t000.mzML --json` -> `exit 2`, empty stdout (`pair_cmd.rs:45`).
    - `dad delay /tmp/does_not_exist_12345.mzML --json` -> `exit 1`, embeds stderr's `"error: ... (err.flow.io)\n"` inside `errore/1.message` (`dad_cmd.rs:374-381`).
  - **40+ commands** reject `--json` with `exit 2` (`error: unknown option --json`), including commands whose default stdout IS JSON (`channels`, `items`, `dda-window`, `background`, `ion-pattern`, `ion-mz`, `match-mz`, `mol-*`, `peak-*`, `samples vars|table`, `msn-graph`, `msn-lib search`, `llm *`). Worse, `samples group --json HRMS_t000.mzML` silently treats `--json` as a filename (`{"nome":""}`).
  - JSON root schema tags vary five ways: `"schema"` (`flow*`, `step/1`, `tools/1`, `folder/1`, `elements/1`, `wiffpair/1`, `samples_list/1`, `campioni/2`, `nomivar/1`, `corsa/1`), `"format"` (`mzflow/2`, `etich/1`, `anal/1`, `check/1`, `cip/1`, `hilite/1`, `sel/1`, `link/1`, `cut/1`, `mecc/1`, `sub/1`, `subdb/1`, `insilico/1`), `"formato"` (`msngrafo/2`, `msnconfronto/1`, `msnlib/1`, `msncerca/1`), `"v"` (`canali/1`), or none (`items`, `validate`, `dda-window`, `dad`, `ion-pattern`, `ion-mz`, `match-mz`, `background`, `peak-*`, `samples group`).
  - Multi-result JSON shape also conflicts: `validate --json` emits a JSON array `[{...}]`, whereas `samples list --json` and `channels --sample 1,2` emit NDJSON (one object per line).

### 8. Broken exit-code discipline and stdout/stderr stream violations
- **Evidence:**
  - `Outcome::fail_error` (`crates/mzlab-cli/src/error_text.rs:112-118`) hardcodes `code: 2` (usage error) for all engine errors. Missing files, unreadable mzMLs, and missing channels in `items`, `dda-window`, `background`, `msn-lib`, `msn-graph compare`, and `mol-*` exit `2` instead of `4` or `1`.
  - `channels` delegates all errors to `sample_select::fail` (`channels_cmd.rs:203`, `sample_select.rs:33-38`), returning `5` for `err.sample.unknown` and `2` for missing files, missing channels (`err.canali.unknown`), or missing DAD matrices (`err.pda.none`).
  - `dad delay /tmp/does_not_exist_12345.mzML --json` exits `1` instead of `4` (and `dad` also returns `1` for unknown `--sample` ID instead of `5`: `dad_cmd.rs:366-372`), violating `docs/agenti/cli-agenti.md:20`.
  - Stream violations:
    - `flow verify RECORD` writes mismatch errors (`self_sha256\n`, exit `3`) to **stdout** and nothing to stderr (`flow_mzrun.rs:73-77`).
    - `validate FILE.mzML` (without `--json`) writes findings (`error|warning ...`) to **stdout** and per-file summaries to **stderr** (`validate.rs:116-120`).
    - `insilico pack CSV OUT` writes human prose (`"... candidates, ... row(s) without SMILES..."`) to **stdout** (`insilico_pack.rs:326-330`).
    - `peak-split` without `split=` prints JSON without a trailing newline (`{"splits":[14.3357,14.3927]}`, verified via `wc -l` = `0`, `peak_split_cmd.rs:154,171`).

### 9. Performance & agent token bottlenecks: ephemeral re-indexing, sequential double-indexing in `llm`, and 140–160 KB uncompactable outputs
- **Evidence (measured with debug `mzlab-cli` on `mzlab/web/esempi/`):**
  - Startup is fast (`5.9 ms` `--version`, `6.2 ms` `--help`, `5.8 ms` `folder list`), but every standalone file command (`items`, `channels`, `dda-window`, `background`, `dad ms-at`/`delay`, `features`, `msn-graph`) builds a fresh `.mzidx` in `/tmp/mzlab-cli-<pid>-*` (~560 ms on 7.3 MB `HRMS_t000.mzML`) and immediately deletes it (`std::fs::remove_dir_all(&tmp)`). Running `items` -> `channels` -> `dda-window` -> `background` rebuilds the same index 4 times (~2.3 s).
  - `llm inspect mzlab/web/esempi` runs sequentially on 1 thread over all `.mzML` files in `DIR` (`13,034.1 ms` for 15 files) and cannot take `FILE...`. `llm params mzlab/web/esempi` then re-runs the entire `inspection()` from scratch (`12,975.9 ms`; **26.0 s combined**) instead of accepting `ispezione/1` JSON via file or stdin (`llm_facts.rs:32-61,192-214`).
  - `steps --json` dumps `159,579 B` and `llm tools` dumps `139,383 B` with no `--summary`/`--compact` flag to list step IDs/ports without full parameter prose; `items HRMS_t000.mzML` dumps `10,849 B` because it includes the raw `dda_pts` `[rt, mz]` array of every MS2 scan.

### 10. Contradictions between `--help` / `docs/agenti/` and actual binary behavior
- **Evidence:**
  - `manual-table --help` (`manual_table_cmd.rs:44-46`) lists 13 columns (`channel, sample, time_min, label, area, start_min, end_min, apex_rt_min, height, snr, baseline, overlap, pv`), whereas real CSV output has **14** columns with `apex_rt_min` 5th and `origin` 14th (`channel,sample,time_min,label,apex_rt_min,area,start_min,end_min,height,snr,baseline,overlap,pv,origin`).
  - `peak-split --help` (`peak_split_cmd.rs:24`) states `--set label=NAME    the name of the window (the last column of the table)`, which is false (`label` is column 4; `origin` is column 14).
  - `manual-table --help` (`manual_table_cmd.rs:34`) documents `--set auto=true` output keys as `{"windows", "log", "groups", "unmatched"}`, omitting `"tol"` (real output has `['tol', 'windows', 'log', 'groups', 'unmatched']`).
  - `peak-auto --help` (`peaks_auto_cmd.rs:19,22`) says `baseline = zero (default)`, yet even with `baseline=zero` it populates `"b0"` and `"b1"` from `auto_integrate`'s edge baseline rather than `0`, and `peak-auto-lr --help` (`peaks_auto_lr_cmd.rs:20`) omits `"baseline"` from `"params"` (real output: `{"params":{"half":18,...,"split":"drop","baseline":"engine","noise":...}}`).
  - `docs/agenti/cli.md:18` claims old names `variabili`, `nome-gruppo`, and `--canale` are "still accepted as input", but live runs prove `samples variabili` and `channels ... --canale` both fail with `exit 2`!
  - `docs/agenti/cli.md:32` and `match_mz_cmd.rs:2` claim `match-mz` outputs `ricerca/1`, which is absent from the actual JSON (`{"x":...,"hr":...,"hits":...}`).
  - `docs/agenti/cli-agenti.md:31-32` omits `pair`, `folder list`, and `elements` from the `--json` list (contradicting line 43 and `--help`).
  - `docs/agenti/for-agents.md:21` uses `"step":"features.detect@1"` in its example plan while `mzlab-cli steps` prints `features.detect@2`.
  - `flow command` (`crates/mzlab-core/src/flow/command.rs:34`) and `docs/agenti/cli.md:76` emit `cargo install --path crates/mzlab-cli`, omitting `--profile cli` recommended in `cli.md:79` and `for-agents.md:6`.
  - `docs/agenti/cli-agenti.md:12-15` documents prebuilt Linux release binaries (`mzLab-X.Y.Z-cli-Linux-x86_64`), contradicting `docs/agenti/cli.md:79` ("Out of scope...: precompiled binaries, releases") and `docs/agenti/for-agents.md:6` ("Today only from source").

---

## 2. Full Findings Table

| Area | Command | Problem | Evidence | Proposed Fix | Effort | Breaking |
|---|---|---|---|---|---|---|
| Strings | `samples vars/table/group`, `channels`, `msn-graph*`, `flow report`, `llm *` | Italian JSON keys, schema IDs (`nomivar/1`, `campioni/2`, `canali/1`, `msngrafo/2`, `msnconfronto/1`, `corsa/1`, `errore/1`, `ispezione/1`, `proposta-param/1`, `contesto/1`, `disegno/1`) | Live CLI outputs; `samples/json.rs:17-58`; `channels/output.rs:36`; `msn_graph.rs:274-362`; `msn_graph_compare.rs:190-286`; `msn_rules/sum.rs:110-196`; `drawing/json.rs:24-75` | Standardize all schemas, JSON keys, and option values to English; keep parser migration if needed for saved `.mzflow` plans | L | yes |
| Strings | `msn-graph`, `msn-rules` | Italian CLI values/placeholders (`--node tutto`, `--decoy SEME`) | `msn_graph.rs:57`; `msn_rules.rs:20` | Replace `tutto` -> `all`, `SEME` -> `SEED` | S | yes |
| Strings | `flow run` (`table.stats`) | Mistranslated `error.badDisegno`: says "drawing file" instead of experimental design | `mzlab/web/lang/en/errori-a-m.js:73`; `drawing/json.rs:82-84` | Rename key to `error.badDesign` and reword to "The experimental design file is not valid..." | S | yes |
| Strings | `samples list`, `convert` | Contradictory `err.file.notAccepted`: lists `.mzML` as accepted while rejecting `.mzML` | `mzlab-cli samples list mzlab/web/esempi/HRMS_t000.mzML --json`; `errori-file.js:46` | Distinguish vendor-only commands (`samples list`, `convert`) with a clear `err.file.notVendor` / `err.file.alreadyMzml` error | S | no |
| Strings | `error_text.rs` | Browser-only error wording ("reload the page", "open the files... again") shown in CLI | `errori-a-m.js:8-25,80`; `errori-flow.js:73,90`; `error_text.rs:37-54` | Add `.native` / CLI overrides in `error_text.rs` for any web-only phrasing | S | no |
| Strings | `flow run`, `flow check`, `convert`, `normative` | `err.cli.*`, `err.flow.step.native`, `err.normative.*` missing from `errori-*.js`; raw keys printed | `error_text.rs:131-133`; `convert.rs:327`; `flow_run.rs:278` (`failed: error.badMzfeat`) | Add all `err.cli.*`, `err.normative.*`, and `err.flow.step.native` templates to `errori-flow.js` and render errors in `progress_line` | S | no |
| Strings | `flow run` | Trailing space on `error: failed s; not run \n` when `skipped` is empty | `flow_out.rs:110-114` | Omit `; not run ...` clause when `run.skipped.is_empty()` | S | no |
| Strings | `manual-table`, `peak-split` | `--help` column order/count wrong (omits `origin`, misplaces `apex_rt_min`); omits `"tol"` in `auto=true` | `manual_table_cmd.rs:34,44-46`; `peak_split_cmd.rs:24`; live CSV header check | Fix `--help` column list, `label` description, and `auto=true` keys | S | no |
| Strings | `peak-auto`, `peak-auto-lr` | `peak-auto` prints edge `b0`/`b1` when `baseline=zero`; `peak-auto-lr --help` omits `"baseline"` in `params` | `peaks_auto_cmd.rs:19,96-109`; `peaks_auto_lr_cmd.rs:20` | Document `b0`/`b1` accurately (or set to `0` under `zero`) and list `baseline` in `peak-auto-lr --help` | S | no |
| Strings | `cmd/mod.rs`, `citation.rs` | Top-level `--help` header describes only `features`; citation says `DOI: to be added` | `cmd/mod.rs:81-90`; `citation.rs:15-17` | Give `mzlab-cli --help` a true top-level summary + "Start here" section; separate `features --help` | S | no |
| Structure | `cmd/features.rs` | `mzlab-cli features --help` prints the entire 35-command global help instead of `features`-only help | `cmd/features.rs:94` calls `super::help()` | Add dedicated `features::HELP` | S | no |
| Structure | `flow`, `llm`, `samples`, `msn-graph`, `msn-rules`, `normative`, `insilico` | 18+ commands/subcommands fail (`exit 2`), return JSON (`{"nome":"help"}`), or treat `--help` as a path | Verified on binary: `flow report/record/verify/command`, `llm *`, `samples vars/table/group`, `msn-graph figure/complete/compare`, `msn-rules sum`, `insilico pack`, `normative pack/listings/import-official` | Route `--help`/`-h` before positional/file parsing in every command and subcommand (`exit 0`, stdout) | M | no |
| Structure | `normative` | `normative bogus` exits `0` with help; `normative pack/listings --bogus` silently ignores `--*` flags | `normative/mod.rs:111,189,219` | Reject unknown subcommands (`exit 2`) and unknown `--*` options (`exit 2`) | S | yes |
| Structure | `explain-missing`, `flow command`, `samples group` | Positional parsers accept unknown `--flags` as filenames or sample names (`--json` -> `{"nome":""}`) | `flow_explain.rs:42`; `cmd/flow.rs:8`; `samples_cmd.rs:377`; `samples group --json HRMS_t000.mzML` | Reject any argument starting with `-` (except bare `-` for stdin) unless declared as a flag | S | yes |
| Structure | `channels` / missing commands | No CLI command can extract XIC, TIC/BPC, SRM trace, or MS1/MS2 spectrum from a file | `channels_cmd.rs:143-149` fails on `srm:*`; `routes.rs:99-119` routes unexposed | Add `trace xic`, `trace chrom`, and `spec at` commands (or extend `channels` + `spectrum`) | M | no |
| Structure | missing `design` command | `mzlab_core::drawing::route` (`disegno/1`) has no CLI command despite being required by `table.stats`/`drift` | `crates/mzlab-core/src/drawing/mod.rs:248-263`; `docs/agenti/cli.md` | Add `sample design propose|validate` (or `design`) CLI command | M | no |
| Structure | Top-level command list | Flat sprawl: 11 `mol-*` commands, 6 `peak-*` commands, inconsistent verbs/nouns (`steps` vs `flow`, `explain-missing`) | `cmd/mol.rs`, `cmd/traces.rs`, `cmd/spectra.rs` | Consolidate into `noun verb` subcommand groups (`mol`, `peak`, `trace`, `spec`, `sample`, `file`, `flow`, `ion`, `db`, `llm`) | L | yes |
| Structure | Flag style | Mix of `--flag val`, `--set k=v`, and `--set` accepting file paths (`--set windows=PATH`) vs `--windows PATH` | `manual_table_cmd.rs:19`; `peak_split_cmd.rs:15`; `match_mz_cmd.rs:10`; `msn_graph.rs:22-62` | Reserve `--set` for `step.param=value` overrides in `flow`/`convert`; use typed `--flags` on direct commands | M | yes |
| Structure | Discovery | No shell completion (`completions`), no `schema` command for `mzflow/2` / `campioni/2` / `design/1`, no "Start here" path | `cmd/mod.rs:109-120`; `steps_cmd.rs` | Add `mzlab-cli completions <bash|zsh|fish>` and `mzlab-cli flow schema [NAME] --json` | M | no |
| Agent | All non-`--json` commands (40+) | Reject `--json` (`exit 2`), lack `errore/1` on stderr/stdout failure, and lack uniform `--dry-run` | `channels`, `items`, `dda-window`, `background`, `ion-*`, `match-mz`, `mol-*`, `peak-*`, `features`, `convert`, `flow report/record/verify` | Support `--json` universally (emitting `error/1` on error) and add `--dry-run` to `convert`, `features`, `msn-lib add/build` | L | no |
| Agent | `validate`, `folder list`, `pair`, `manual-table`, `dad` | Partial/buggy `--json`: `validate` prints `[]` on missing file; `folder`/`pair` print empty stdout on error; `dad` embeds `"error: ...\n"` in `message`; `manual-table --json` forbids `auto`/`shifts` | Live checks; `validate.rs:83-118`; `folder_cmd.rs:242`; `pair_cmd.rs:45`; `dad_cmd.rs:374`; `manual_table_cmd.rs:102` | Emit standardized `error/1` on every `--json` failure and allow `--json` on all modes | M | yes |
| Agent | JSON root keys | Five incompatible schema key conventions (`"schema"`, `"format"`, `"formato"`, `"v"`, none) and NDJSON vs array conflicts | `channels_cmd.rs`, `items_cmd.rs`, `mol_label_cmd.rs`, `msn_graph.rs`, `validate.rs`, `samples_cmd.rs` | Use `"schema": "<name>/<ver>"` on every JSON output and consistent JSON arrays (with `--ndjson` opt-in if streaming) | L | yes |
| Agent | `Outcome::fail_error`, `channels`, `dad` | Exit codes collapse to `2` (`fail_error`, `channels`) or `1` (`dad` missing file / unknown `--sample` ID) | `error_text.rs:112-118`; `channels_cmd.rs:203`; `dad_cmd.rs:366-372` | Map `mzlab_core::Error` keys to unified exit codes `1..6` across all commands | M | yes |
| Agent | `flow verify`, `validate`, `insilico pack`, `peak-split` | Stream & formatting violations: errors on stdout (`flow verify`, `validate`), prose on stdout (`insilico`), missing `\n` (`peak-split`) | `flow_mzrun.rs:73-77`; `validate.rs:116-120`; `insilico_pack.rs:326-330`; `peak_split_cmd.rs:154` | Put data only on stdout, diagnostics/summaries on stderr, and ensure trailing `\n` on all stdout JSON | S | yes |
| Agent | `flow record`, `samples table` | `flow record` always fails on `cargo build`/`install` (`engine (commit)`) and `samples table` output (`sha256:""`) | Live run on `run1`; `crates/mzlab-core/src/mzrun/build.rs:43-47,92-95`; `samples_cmd.rs:121` | Add `crates/mzlab-cli/build.rs` to capture `git rev-parse HEAD`; let `samples table` accept real file paths and hash them | S | no |
| Speed | `items`, `channels`, `dda-window`, `background`, `dad`, `msn-graph` | Rebuilds and deletes `.mzidx` in `/tmp` on every single call (~560 ms/call on `HRMS_t000.mzML`) | Measured `558–623 ms` per call; `items_cmd.rs:83-87`, `channels_cmd.rs:180-201`, etc. | Add `--cache-dir DIR` (or shared temp/XDG cache keyed by file SHA-256/mtime+size) and `file index` | M | no |
| Speed | `llm inspect`, `llm params` | Sequential 1-thread indexing of all mzMLs in `DIR` (13.0 s on `esempi`); `llm params` repeats it (26.0 s total) | Measured `13,034 ms` + `12,976 ms` on `mzlab/web/esempi/` | Parallelize indexing with `par.rs`, accept `FILE...`, and let `llm params` read `inspect` JSON from file or `-` | M | no |
| Speed | `steps --json`, `llm tools`, `items` | Unpaginated 160 KB `steps --json`, 139 KB `llm tools`, and 11 KB `items` (`dda_pts` array) bloat agent context | Measured `159,579 B`, `139,383 B`, `10,849 B` | Add `--compact` / `--summary` to `steps --json` & `llm tools`, and `--no-points` / summary mode to `items` | S | no |
| Docs | `cli.md:18` | Claims `variabili`, `nome-gruppo`, `--canale` are "still accepted"; binary rejects `variabili` and `--canale` (`exit 2`) | `mzlab-cli samples variabili` (`exit 2`), `channels ... --canale` (`exit 2`) | Remove stale aliases from docs (or wire aliases if still wanted—docs say no backward compat needed) | S | no |
| Docs | `cli.md:32`, `cli-agenti.md:31`, `for-agents.md:21`, `cli.md:76` | `match-mz` lacks `ricerca/1`; `cli-agenti.md` omits `pair`/`folder list`/`elements`; `for-agents.md` uses `features.detect@1`; `flow command` omits `--profile cli`; `cli-agenti.md` vs `cli.md` on Linux release binaries | `match_mz_cmd.rs:2`; `cli-agenti.md:12-15,31-32`; `for-agents.md:21`; `flow/command.rs:34`; `cli.md:79` | Sync `docs/agenti/cli*.md`, `for-agents.md`, and `flow/command.rs` with binary truth | S | no |
| Docs | `INDICE.md:183`, `cli.md:8` | `explain-missing` relies on Tabui (`tp.link`), marked in `INDICE.md:183` as hidden/to be deleted | `docs/agenti/INDICE.md:183` | Clarify status or generalize `explain-missing` beyond Tabui `tp/1` | S | no |

---

## 3. Speed Benchmarks & Five Typical Agent Workflows

### Measured Timings (Debug Binary on `mzlab/web/esempi/`)

| Command | Measured Time | Exit | Stdout | Stderr |
|---|---:|---:|---:|---:|
| `mzlab-cli --version` | 5.9 ms | 0 | 115 B | 0 B |
| `mzlab-cli --help` | 6.2 ms | 0 | 6,434 B | 0 B |
| `mzlab-cli folder list mzlab/web/esempi --json` | 5.8 ms | 0 | 2,228 B | 0 B |
| `mzlab-cli steps --json` | 10.7 ms | 0 | 159,579 B | 0 B |
| `mzlab-cli llm tools` | 12.6 ms | 0 | 139,383 B | 0 B |
| `mzlab-cli dda-window mzlab/web/esempi/HRMS_t000.mzML --from 5.0 --to 6.0` | 558.8 ms | 0 | 9,113 B | 0 B |
| `mzlab-cli channels mzlab/web/esempi/HRMS_t000.mzML` | 562.5 ms | 0 | 10,960 B | 0 B |
| `mzlab-cli items mzlab/web/esempi/HRMS_t000.mzML` | 565.5 ms | 0 | 10,849 B | 0 B |
| `mzlab-cli channels mzlab/web/esempi/MRM_std_0.6ppm.mzML` | 617.7 ms | 0 | 749 B | 0 B |
| `mzlab-cli background mzlab/web/esempi/FullScan_t0.mzML` | 623.5 ms | 0 | 34,479 B | 0 B |
| `mzlab-cli validate mzlab/web/esempi/HRMS_t000.mzML --json` | 785.0 ms | 0 | 103 B | 82 B |
| `mzlab-cli manual-table MRM_std_0.6ppm.mzML MRM_std_2.4ppm.mzML --set auto=true` | 1,053.1 ms | 0 | 1,025 B | 0 B |
| `mzlab-cli features HRMS_t000.mzML HRMS_t005.mzML` | 1,544.9 ms | 0 | 233,430 B | 170 B |
| `mzlab-cli msn-graph mzlab/web/esempi/HRMS_t000.mzML` | 1,919.1 ms | 0 | 853,832 B | 0 B |
| `mzlab-cli llm inspect mzlab/web/esempi` | 13,034.1 ms | 0 | 9,106 B | 0 B |
| `mzlab-cli llm params mzlab/web/esempi` | 12,975.9 ms | 0 | 2,795 B | 0 B |

### Five Typical Agent Workflows (Call Count & Failure Points)

1. **Inspect an unknown folder, propose parameters, run an untargeted HR pipeline, and export a filtered CSV**
   - **Calls needed (minimum 7):**
     1. `folder list DIR --json` (discover files)
     2. `llm inspect DIR` (~13.0 s; cannot pass `FILE...`, so it indexes unrelated LR/MRM files in `DIR` too)
     3. `llm params DIR` (~13.0 s; **re-indexes every file in `DIR` from scratch**)
     4. `steps --json` (~160 KB token dump to verify step ports/versions)
     5. `printf ... | samples table` (build `campioni/2` metadata for `table.filter@2` / `export.table_csv@1`)
     6. `flow check plan.json --json`
     7. `flow run plan.json --in DIR --out RUN --json`
   - **Where it fails / stalls:** `llm inspect` mixes incompatible series in a folder; `llm params` wastes 13 s re-indexing; `samples table` uses Italian keys (`campioni/2`, `ordine_iniezione`, `tipo`) and outputs empty `"sha256":""`.

2. **Reproducible run with cryptographic `mzrun/1` provenance (`flow run` -> `flow record` -> `flow verify`)**
   - **Calls needed (5):** `samples table` -> `flow check` -> `flow run` -> `flow record` -> `flow verify`.
   - **Where it fails:** Fails **twice** at step 4 (`flow record`). First with `err.mzrun.build: role (HRMS_t000.mzML)` because `samples table` outputs empty `"sha256":""`; after fixing that, it fails with `err.mzrun.build: engine (commit)` because `cargo build` / `cargo install` sets no `MZLAB_COMMIT`.

3. **Extract an XIC or MRM transition trace from an `.mzML` and integrate peaks (`peak-auto` / `peak-auto-lr` / `peak-area`)**
   - **Calls needed (intended 3):** `channels FILE` -> extract trace CSV -> `peak-auto` / `peak-auto-lr` / `peak-area`.
   - **Where it fails:** **Impossible via `mzlab-cli` alone.** `channels --channel "srm:364.1>194.1@20"` fails with `err.canali.unknown` (`exit 2`), and no `xic` or `chrom` CLI command exists. Agents must either write an external XML parser or fall back to `manual-table --set auto=true` (which does not return raw trace points).

4. **Differential statistics (`table.stats@1`) or batch drift correction (`table.drift@1`)**
   - **Calls needed (5+):** `features`/Flow base -> `samples table` -> create `design.json` -> `flow check` -> `flow run`.
   - **Where it fails:** **No CLI command creates or validates `disegno/1`**, and `steps --json` does not describe its schema (`condizioni`, `confronti`, `fattori`, `lotti`, `riferimento`, `"schema":"disegno/1"`). An agent writing English keys gets `error.badDisegno` rendered as `"The drawing file is not valid or was made by another program."`.

5. **Multi-sample vendor folder triage, selection, conversion, and mzML validation**
   - **Calls needed (6):** `folder list DIR --json` -> `pair FILE... --json` -> `samples list FILE... --json` -> `samples select FILE --sample LIST --json` -> `convert INPUT... --out OUT --report r.json` -> `validate OUT/*.mzML --json`.
   - **Where it fails:** Calling `samples list` or `convert` on an `.mzML` yields contradictory `err.file.notAccepted` ("Accepted: .mzML..."). `samples list --json` prints NDJSON (1 JSON object/line) while `validate --json` prints a JSON array; `convert` has no `--json` or `--dry-run` mode and writes machine output only via `--report FILE.json`.

---

## 4. Proposed New Command Tree & Naming Scheme

Because `mzlab` is unpublished and no backward compatibility is required, adopt a strict `noun verb` (resource-action) command tree in **kebab-case**, with universal `--json`, `--help`/`-h`, `--out PATH`, and `--cache-dir DIR` on file-reading commands. Keep `features` as the one-command shortcut for the pipeline.

### Command Tree Overview
- `mzlab-cli features` — one-shot feature table CSV/JSON
- `mzlab-cli flow <run|check|steps|report|record|verify|command|schema>` — workflow engine & schemas
- `mzlab-cli file <folder|items|channels|samples|pair|validate|convert|index>` — file & dataset operations
- `mzlab-cli trace <xic|chrom|background|dad>` — chromatogram/trace extraction & DAD math
- `mzlab-cli spec <at|dda-window|msn-graph|msn-lib|msn-rules>` — spectra & MSn fragmentation
- `mzlab-cli peak <auto|auto-lr|edges|area|window|split|table>` — peak integration & manual tables
- `mzlab-cli sample <vars|group|table|design|list|select>` — sample naming, metadata & design (`design/1`)
- `mzlab-cli ion <mz|pattern|match|elements>` — exact masses, isotopes, adducts & periodic table
- `mzlab-cli mol <label|anal|check|cip|hilite|sel|link|export|cut|mech|search>` — 2D structures
- `mzlab-cli db <normative|insilico>` — reference databases & candidate packs
- `mzlab-cli llm <tools|inspect|params|context|replay|eval>` — assistant tools & evals
- `mzlab-cli completions <bash|zsh|fish>` — shell completions

### Before / After Mapping Table

| Current Command | Proposed Command | Notes / Changes |
|---|---|---|
| `features` | `features` | Add `--json`, `--dry-run`, dedicated `--help`, `--cache-dir` |
| `flow run\|check\|report\|record\|verify\|command` | `flow run\|check\|report\|record\|verify\|command` | Fix `--help` on all subcommands; add `--json` to `report`/`record`/`verify` |
| `steps` | `flow steps` | Move under `flow`; add `--compact` (IDs, versions, titles, `executable_native`) |
| *(missing)* | `flow schema [NAME] --json` | **New:** expose `mzflow/2`, `samples/2`, `design/1`, `error/1`, etc. |
| `folder list` | `file folder DIR` | Add `error/1` on exit `4` |
| `items` | `file items FILE...` | Add `--json`, `--no-points` (omit `dda_pts`), `--cache-dir`, fix exit codes |
| `channels` | `file channels FILE` | List channels only (rename schema to `channels/1`, English keys) |
| `pair` | `file pair FILE...` | Emit `error/1` on `--json` failure |
| `validate` | `file validate FILE.mzML...` | Summaries to stderr; structured findings to stdout; fix missing-file `--json` |
| `convert` | `file convert INPUT... --out DIR` | Add `--json` and `--dry-run`; clear error when input is already `.mzML` |
| *(missing)* | `file index FILE... --out DIR` | **New:** pre-build shared `.mzidx` cache for fast repeated queries |
| *(missing)* | `trace xic FILE --mz M --tol T` | **New:** expose `/api/xic` as JSON or 2-col CSV (`--format csv`) for `peak-*` |
| `channels --channel ...` | `trace chrom FILE --id ID` | **New/split:** expose `/api/chrom` (`tic`, `bpc`, `srm:*`, `uv:*`, `an:*`) + `--format csv` |
| `background` | `trace background FILE` | Add `--json`, typed flags alongside `--set`, `--cache-dir` |
| `dad subtract\|spectrum\|trace\|ms-at\|delay` | `trace dad <subcmd>` | Fix exit codes (`4` missing file, `5` missing sample); clean `error/1.message` |
| *(missing)* | `spec at FILE (--rt MIN \| --sid ID)` | **New:** expose `/api/spectrum` + `/api/scaninfo` |
| `dda-window` | `spec dda-window FILE` | Move under `spec`; add `--json`, `--cache-dir` |
| `msn-graph [complete\|compare\|figure]` | `spec msn-graph [build\|complete\|compare\|figure]` | Explicit `build` verb (or default); English JSON (`msn-graph/2`), `--node all` |
| `msn-lib add\|build\|export\|search` | `spec msn-lib <subcmd>` | Add `--json` on all subcommands; fix exit codes |
| `msn-rules [sum]` | `spec msn-rules [eval\|sum]` | Fix `sum --help`; replace `--decoy SEME` -> `--decoy SEED`; English JSON |
| `peak-auto` | `peak auto TRACE.csv\|-` | Add `--json`; clarify `b0`/`b1` under `baseline=zero` |
| `peak-auto-lr` | `peak auto-lr TRACE.csv\|-` | Add `--json`; document `params.baseline` in `--help` |
| `peak-edges` (old `peaks-edges`) | `peak edges TRACE.csv\|-` | Add `--json`; fix `--help` |
| `peak-area` | `peak area TRACE.csv\|-` | Add `--json` |
| `peak-window` | `peak window TRACE.csv\|-` | Add `--json` |
| `peak-split` | `peak split FILE...` | Add trailing `\n`; `--json` works in both split and valley modes |
| `manual-table` | `peak table FILE...` | Fix `--help` 14 columns; allow `--json` with `--auto`/`--shifts` |
| `samples vars\|group\|table\|list\|select` | `sample vars\|group\|table\|list\|select` | `sample table FILE...` hashes real files; fix `--help`/`--json`; English `samples/2` |
| *(missing)* | `sample design <propose\|validate>` | **New:** expose `mzlab_core::drawing::route` (`design/1`) |
| `ion-mz`, `ion-pattern`, `match-mz`, `elements` | `ion mz\|pattern\|match\|elements` | Uniform `--json`, exit codes, `schema` tags (`match-mz/1`, etc.) |
| `mol-{label,anal,check,cip,hilite,sel,link,export,cut,mech,search}` | `mol <label\|anal\|check\|cip\|hilite\|sel\|link\|export\|cut\|mech\|search>` | Consolidate 11 top-level commands into 1 group; fix exit codes |
| `normative`, `insilico` | `db normative\|insilico` | Fix flag parsing in `normative` and `--help`/`--json`/stdout in `insilico` |
| `llm tools\|inspect\|params\|context\|replay\|eval` | `llm <subcmd>` | Fix `--help`; accept `FILE...` and piped `inspect` JSON in `params`; parallel index |
| `explain-missing` | `explain-missing` *(or remove with Tabui)* | Fix unknown-flag handling; align with Tabui removal plan (`INDICE.md:183`) |
| `help-md` | *(internal / hidden)* | Keep hidden or `dev help-md` |

---

## 5. Proposed Exit-Code Table & JSON Error Object Shape

### Standardized Exit-Code Table

| Code | Meaning | When Used (Every Command) |
|---:|---|---|
| `0` | **Success** | Command completed with no data, plan, or validation errors (warnings allowed on stderr). |
| `1` | **Data / domain error** | Valid CLI invocation & readable input, but domain check failed: `validate` found mzML errors; `flow run` / `convert` had failed steps/files (partial outputs written); `samples list` refused a sample or truncated `.wiff.scan`; `pair` missing companion; `dad` missing DAD matrix; `llm eval` failed cases. |
| `2` | **Usage / syntax error** | Unknown command/subcommand/flag, missing required argument, conflicting flags, malformed `--set` value, invalid formula/SMILES syntax in CLI args. |
| `3` | **Plan / schema / integrity failure** | `flow check` invalid plan (`problems`); `flow verify` fingerprint mismatch; `convert` partial read (`conv/1.partial`); `normative` source pin mismatch; invalid input JSON document (`mzflow/2`, `samples/2`, `design/1`). |
| `4` | **Input missing or unreadable** | Input file or folder missing, is a directory when file expected (or vice versa), unreadable permissions, corrupt container, or SHA-256 mismatch against plan. |
| `5` | **Selector not found** | Valid file opened, but requested `--sample ID`, `--channel ID`, `--node ID`, or `--id STEP` does not exist in it. |
| `6` | **Output I/O or external source failure** | Cannot create/write `--out`, `--report`, or `prov.json`; network/HTTP/download limit failure in `db normative build`. |

### Standardized JSON Error Object Shape (`error/1`)

When `--json` is active (or on JSON-by-default commands), **every non-zero exit** writes a single `error/1` JSON object (ending with `\n`) to **stdout**, while keeping human-readable `error: ...` and `hint: ...` lines on **stderr**:

```json
{
  "schema": "error/1",
  "code": 4,
  "error_key": "err.file.notFound",
  "params": {
    "path": "missing.mzML"
  },
  "message": "File not found: missing.mzML",
  "hint": "Check the file path or pass --in FOLDER.",
  "command": "file items",
  "problems": []
}
```

- `message` must never include `"error: "` prefix or trailing `"\n"` (fixes current `dad --json` bug).
- `problems` is omitted or `[]` except on plan/schema validation failures (`code: 3`), where each entry is `{"instance": "...", "port": "...", "error_key": "...", "params": {...}, "message": "..."}`.
- All success JSON payloads use `"schema": "<english-name>/<version>"` as the first key.

---

## 6. Suggested Order of Work (Independent PR-Sized Tasks)

Each task is scoped to one chat / one PR with clear files and acceptance checks.

1. **PR 1 — Fix `--help`/`-h`, root vs `features` help, and trailing newlines**
   - **Scope:** Make `--help`/`-h` return `exit 0` on stdout across all 18+ failing commands/subcommands (`flow *`, `llm *`, `samples *`, `msn-graph *`, `msn-rules *`, `normative *`, `insilico pack`); reject unknown `--flags` in `normative`, `explain-missing`, `flow command`, `samples group`; separate `mzlab-cli --help` ("Start here" + command index) from `features --help`; add trailing `\n` to `peak-split`.
   - **Files touched:** `crates/mzlab-cli/src/cmd/*.rs`, `llm*.rs`, `samples_cmd.rs`, `msn_graph.rs`, `msn_outputs.rs`, `msn_graph_compare.rs`, `msn_rules*.rs`, `normative/mod.rs`, `insilico_pack.rs`, `flow_report.rs`, `flow_mzrun.rs`, `flow_explain.rs`, `peak_split_cmd.rs`, `tests/data/help/help_snapshot.txt`.
   - **Acceptance test:** Iterate every command + subcommand path in `cmd::tests`: `--help` and `-h` exit `0` with non-empty stdout and empty stderr; unknown `--bogus` exits `2`; `peak-split` stdout ends with `\n`.

2. **PR 2 — Fix `flow record` out of the box (`MZLAB_COMMIT` + `samples table` hashing)**
   - **Scope:** Add `crates/mzlab-cli/build.rs` emitting `cargo:rustc-env=MZLAB_COMMIT=...` from `git rev-parse HEAD` (preserving explicit `MZLAB_COMMIT` override); allow `samples table [FILE...]` to accept real paths and compute SHA-256 digests (`samples_cmd.rs`) instead of hardcoding `""`.
   - **Files touched:** `crates/mzlab-cli/build.rs` (new), `crates/mzlab-cli/src/samples_cmd.rs`, `crates/mzlab-cli/src/flow_mzrun_tests.rs`.
   - **Acceptance test:** Run `flow run` -> `samples table FILE...` -> `flow record` -> `flow verify` using a fresh `cargo build -p mzlab-cli` binary without patching `prov.json` or `roles.json`; `flow verify` exits `0`.

3. **PR 3 — Unified exit codes (`0..6`), universal `--json`, `error/1`, and stream discipline**
   - **Scope:** Replace `Outcome::fail_error` hardcoded `code: 2` with structured `mzlab_core::Error` -> exit code `1..6` mapping; accept `--json` on all commands; emit `error/1` on stdout for all `--json` failures (`validate`, `folder`, `pair`, `manual-table`, `dad`, etc.); move `flow verify` errors and `validate` summaries to proper streams and `insilico pack` summary to stderr (or JSON on stdout).
   - **Files touched:** `crates/mzlab-cli/src/error_text.rs`, `flow_json.rs`, `validate.rs`, `folder_cmd.rs`, `pair_cmd.rs`, `dad_cmd.rs`, `channels_cmd.rs`, `manual_table_cmd.rs`, `flow_mzrun.rs`, `insilico_pack.rs`, and `*_cmd.rs`.
   - **Acceptance test:** Integration tests verifying exit codes `0..6` and valid `error/1` JSON on stdout (no `"error: "` prefix inside `message`) across all commands for missing files (`4`), unknown selectors (`5`), invalid flags (`2`), and domain failures (`1`/`3`).

4. **PR 4 — English-only JSON schemas, keys, and CLI values**
   - **Scope:** Replace all Italian JSON schema names, keys, and CLI option values (`nomivar/1`, `campioni/2`, `canali/1`, `msngrafo/2`, `msnconfronto/1`, `msnlib/1`, `msncerca/1`, `disegno/1`, `corsa/1`, `errore/1`, `ispezione/1`, `proposta-param/1`, `contesto/1`, `conv/1`, `metodo/1`, `--node tutto`, `--decoy SEME`, `tipo_bianco` in `mzrun/1`); standardize `"schema"` as the discriminator key across all JSON outputs; regenerate `docs/schemi/*.json`.
   - **Files touched:** `crates/mzlab-core/src/{samples,channels,drawing,msngraph,mzrun,flow,conv}/*.rs`, `crates/mzlab-llm/src/*.rs`, `crates/mzlab-cli/src/*.rs`, `docs/schemi/*.json`, parity tests (`tests/`).
   - **Acceptance test:** `cargo test --workspace` and `python3 tools/controlla_i18n.py` pass; grep for known Italian schema/key words (`campioni`, `canali`, `disegno`, `errore`, `corsa`, `ispezione`, `formato`, `tutto`, `SEME`) in CLI JSON outputs returns zero matches.

5. **PR 5 — Fix CLI error message catalog & `--help` factual mismatches**
   - **Scope:** Fix `error.badDisegno` -> `error.badDesign` wording; split `err.file.notAccepted` so vendor-only commands (`samples list`, `convert`) do not list `.mzML` as accepted while rejecting it; add `err.cli.*`, `err.normative.*`, `err.flow.step.native` to `errori-flow.js`; add `.native` CLI overrides for browser-worded keys; render human text in `flow run` stderr progress lines; fix `manual-table`/`peak-split`/`peak-auto`/`peak-auto-lr` `--help` texts.
   - **Files touched:** `mzlab/web/lang/en/errori-*.js`, `crates/mzlab-cli/src/{error_text.rs,flow_run.rs,manual_table_cmd.rs,peak_split_cmd.rs,peaks_auto_cmd.rs,peaks_auto_lr_cmd.rs}`.
   - **Acceptance test:** `samples list HRMS_t000.mzML`, bad `design.json` in `table.stats`, and missing-file `flow run` emit clear CLI-appropriate English errors; `manual-table --help` column list matches actual CSV header byte-for-byte.

6. **PR 6 — Add missing CLI primitives (`trace xic`, `trace chrom`, `spec at`, `sample design`, `flow schema`)**
   - **Scope:** Expose `Routes` `/api/xic`, `/api/chrom`, `/api/spectrum`, and `/api/scaninfo` as CLI commands (`trace xic`, `trace chrom` supporting `tic`/`bpc`/`srm:*`/`uv:*`/`an:*` and `--format csv` for `peak-*`, `spec at`); expose `drawing::route` as `sample design propose|validate`; add `flow schema [NAME] --json` and `flow steps --json --compact`.
   - **Files touched:** `crates/mzlab-cli/src/` (new command modules + `cmd/` registration), `crates/mzlab-core/src/routes.rs` (if helper needed), parity tests.
   - **Acceptance test:** End-to-end test extracts an SRM transition with `trace chrom --format csv | peak auto-lr -` (`exit 0`), extracts an XIC (`exit 0`), proposes/validates `design/1` from `sample table` and feeds `table.stats@1` (`exit 0`).

7. **PR 7 — Index caching (`--cache-dir` / `file index`), parallel `llm inspect`, and piped `llm params`**
   - **Scope:** Add `file index` and `--cache-dir DIR` (reusing `.mzidx` keyed by file SHA-256 or size+mtime) to `items`, `channels`, `dda-window`, `background`, `dad`, `msn-graph`, `trace`, and `spec`; parallelize `llm inspect` via `par.rs` and accept `FILE...`; let `llm params` accept `--inspect FILE|-` so it skips re-indexing; add `--no-points` to `items` and `--compact` to `llm tools`.
   - **Files touched:** `crates/mzlab-cli/src/{idx_open.rs,flow_pace.rs,llm_facts.rs,items_cmd.rs,channels_cmd.rs,dda_window_cmd.rs,background_cmd.rs,dad_cmd.rs,msn_graph.rs}`.
   - **Acceptance test:** Benchmark on `mzlab/web/esempi/` shows warm-cache `items`/`channels`/`dda-window` under `50 ms` and `llm inspect` + piped `llm params` under `3 s` combined.

8. **PR 8 — Command tree reorganization, shell completions & documentation sync**
   - **Scope:** Reorganize flat commands into the proposed `noun verb` tree (`flow`, `file`, `trace`, `spec`, `peak`, `sample`, `ion`, `mol`, `db`, `llm`, `features`, `completions`); implement `mzlab-cli completions <bash|zsh|fish>`; regenerate `docs/agenti/cli.md` (`tools/cli_doc.py`), update `cli-agenti.md`, `for-agents.md`, and `flow/command.rs` (`--profile cli`), and resolve `cli-agenti.md` vs `cli.md` release binary discrepancy.
   - **Files touched:** `crates/mzlab-cli/src/cmd/*.rs`, `crates/mzlab-core/src/flow/command.rs`, `docs/agenti/{cli.md,cli-agenti.md,for-agents.md}`, `crates/mzlab-cli/tests/data/help/help_snapshot.txt`.
   - **Acceptance test:** `cargo test -p mzlab-cli`, `python3 tools/controlla.py`, and `python3 tools/gate_release.py` (if applicable) pass; every workflow 1–5 runs cleanly against `mzlab/web/esempi/` using documented commands.
