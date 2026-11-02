# Short terminal names: inventory, grammar, rename table

Rule (Federico, 29/10, REGOLE.md): what is typed or read in the terminal uses short names that stay self-explanatory. This study lists the names, proposes a grammar and the renames with a clear gain, and splits them by risk. Build prompts: [`prompt_nomi/`](prompt_nomi/LEGGIMI.md). Inventory taken from `origin/main` at `bcc5210`.

## 1. Two kinds of name

- **Input-only** (typed, never written into an output file): CLI commands, subcommands, flags, flag values, `--set` names of `peaks-edges`, environment variables, `tools/` options. A rename with the old name kept as an alias changes no output byte. Safe.
- **Persisted** (written into outputs or users' files): Flow step ids, step parameters (`plan.json` canonical form, `params_sha256`, `prov/1`, `corsa/1`, cache keys `flow::cache_key`, `mzrun/1` `resolved`/`seeds`), port names, schema ids (`corsa/1`, `canali/1`, …), output file names (`corsa.json`, `features.csv`, `<instance>.<ext>`), the `opzioni` keys of `msngrafo/1` (`crates/mzlab-core/src/msngrafo/json.rs:32`). The engine has no step-version migration today (no step at `version: 2`, no `migrat*` code in `mzlab_core::flow`). An alias accepted on input would still print the old name (in `plan.json`, `--dry-run`, `steps --json`, «Run on CLI»), so the rule «old name never printed» cannot hold without a version bump. **Stop point**: none of these is renamed here; §5 asks the COO.

## 2. Inventory (score: fine / long / unclear / inconsistent)

### 2.1 Commands (`crates/mzlab-cli/src/lib.rs:419`, `PLAIN` at :377)
| Name | Len | Score | Note |
|---|---|---|---|
| `features`, `flow`, `steps`, `llm`, `convert`, `validate`, `insilico`, `msn-graph`, `msn-lib`, `msn-rules`, `ion-pattern` | 3–11 | fine | |
| `flow run\|check\|report\|record\|verify\|command`, `llm tools\|replay\|context\|eval`, `msn-lib add\|build\|export\|search`, `msn-graph figure\|complete\|compare`, `normative build`, `insilico pack` | ≤8 | fine | |
| `canali` | 6 | inconsistent | Italian; every other command and message is English (cli.md «Messaggi») |
| `campioni variabili\|nome-gruppo` | 8/9/11 | inconsistent | Italian; `nome-gruppo` is noun-noun reversed |
| `peaks-edges` | 11 | inconsistent | plural first noun; `ion-pattern`, `msn-graph` are singular |
| `msn-rules somma` | 5 | inconsistent | Italian subcommand |
| `explain-missing` | 15 | long | kept: `missing` alone loses the meaning |
| `normative` | 9 | fine | an English adjective; the module and keys `err.normative.*` use it |

### 2.2 Flags (per file, non-test sources)
| Command | Flags | Score |
|---|---|---|
| `features` | `--out --preset --params` | fine |
| `flow run` | `--in --out --set --workers --budget-mb --dry-run --fresh --json --progress text\|jsonl --jsonl` | `--jsonl` unclear: next to `--json` and `--progress jsonl` it is the third JSON switch, and it means «`flowlive/1` events on stdout» (`flow_run.rs:40`) |
| `flow check` / `steps` / `validate` | `--set --json` / `--json --id` / `--json` | fine |
| `flow record` | `--roles CAMPIONI.json --out` | placeholder `CAMPIONI.json` Italian (help text only) |
| `convert` | `--out --compression --zlib --numpress --peaks --workers --report --fresh --set` | fine |
| `canali` | `--canale --rt --n --out` | `--canale` Italian |
| `campioni` | `--names --pos --sep --has --substring` | fine |
| `peaks-edges` | `--at --ref --mrm --set NAME=VALUE` with `win, areaNoise, edgeFrac, snMin, noise, minHalf, multi, tail, constrain, nr0, nr1` | `--set` names camelCase: every other `--set` name is snake_case |
| `llm` | `--out --conv --turn --strict --budget --hide-names --replay --provider --key-file` | fine |
| `msn-graph` (`msn_graph.rs:100-190`) | `--formula --smiles --molfile --ppm-formula --h-max --cutoff-q --export --out` fine; Italian: `--mz-basso --ppm-basso --prove-min --attivazione --isotopi --scelta --verita --riassunto --pulito --nodo --strutture --interpreta --ppm-perdita --regole --toll-perdita-mda --figura --vista --rotture --max-legami --ppm-struttura`; values `--vista albero\|radiale`, `--strutture tutti` | inconsistent (Italian), `--toll-perdita-mda` long (17) |
| `msn-lib` | `--id --dir --out --ppm --tol --top --delta` fine; `--annota --formato --pulito --tutti --min-prove` | inconsistent; the step `msn.library_export` already calls them `annotate`, `format`, `clean` |
| `msn-rules` | `--decoy --smiles --out` fine; `--attivazione --regole --lista` | inconsistent |
| `normative` | `--cache --out --only --update-pins --no-resolve --allow-large --sources --overrides` | fine |
| `insilico pack` | `--doi --version --piece-raw BYTES` | fine |
| `explain-missing` | `--ppm --rt-tol --out` | fine |

### 2.3 Flow steps and parameters (persisted; `docs/schemi/steps-*-1.json`, 53 steps)
- Step ids: `family.noun` or `family.noun_noun`, 7–20 chars (`structure.reactivity` 20, `export.annotations` 18, `msn.library_export` 18, `features.ms2_link` 17). Clear except `tp.dig_lr` (unclear: «dig»). Kept (§5).
- Parameters, same concept spelled several ways (inconsistent): m/z window in ppm `ppm`, `tol_ppm`, `search_ppm`, `match_ppm`, `select_ppm`, `ppm_max`, `mz_tolerance_ppm`; in Da `tol_da`, `mz_tol_da`, `mz_tolerance_da`, `mz_tol`; RT `rt_tol`, `rt_tol_min`, `rt_tolerance_min`, `rt_window_min`, `tol_min`; bound position `min_points`/`max_gap` against `std_levels_min`/`qc_every_max`; `min_prove` (`spectra.match` tree) against `prove_min` (`spectra.msn_graph`).
- Italian parameters (inconsistent): all of `spectra.msn_graph` except `formula, molecule, adduct, smiles, molfile, ppm_formula, cutoff_q, h_max`: `mz_basso, ppm_basso, prove_min, attivazione, isotopi, strutture_nodo, ppm_struttura, interpreta, ppm_perdita, regole, toll_perdita_mda, scelte, rotture, max_legami`; `prove_min` of `spectra.match`.
- Long: `meta.compare` `mz_tolerance_ppm` 16, `mz_tolerance_da` 15, `rt_tolerance_min` 16; `annot.level` `putative_library_level3` 23 (clear, kept).
- Seeds (`mzrun/1` `seeds`: integer params named `seed` or `*_seed`): `features.align.rt_seed`, `rt.predict` forest `seed`, `structure.reactivity.seed`. Fine; the `*_seed` suffix is part of the grammar.
- Ports and data types (`in`/`out`, types `table`, `report`, `mzidx`, …): short and clear. Fine.

### 2.4 Records and output names (persisted)
`plan.json`, `prov.json` (`prov/1`), `corsa.json` (`corsa/1`, Italian), `cache/index.json` (`mzflow-cache/1`), `<instance>.<ext>`, `<instance>-<k>.<ext>`, `features.csv`, `CARTELLA/.convert`, `NAME.mzML`, `NAME-ID.mzML`; schema ids printed by commands: `canali/1`, `campioni/1`, `nomivar/1`, `strumenti/1`, `contesto/1`, `msngrafo/1`, `msnlib/1`, `msnconfronto/1`, `errore/1`, `flowdry/1`, `flowrun/1`, `flowcheck/1`, `flowlive/1`, `mzrun/1`. `mzrun/1` fields `role` (values = `tipo` of `campioni/2`), `tipo_bianco`. Italian ones are inconsistent but persisted: kept (§5).

### 2.5 Environment variables (read with `env::var`/`environ`/`option_env!`)
| Name | Score | Note |
|---|---|---|
| `MZLAB_UPDATE_STEPS`, `_LLM`, `_FLOW_CASES`, `_SCHED_CASES`, `_GOLDEN` | fine | verb `UPDATE` |
| `MZLAB_WRITE_GOLDEN` (`mzlab-feat/tests/network_golden.rs`), `MZLAB_GOLDEN_WRITE` (`famalign.rs`), `MZLAB_REGEN_GOLDEN` (`tests_e2e/golden_ref.py`, `tests/test_golden_sintetici.py`), `MZLAB_WRITE_FIXTURE` (`rete.rs`), `MZLAB_GUARDIE_CREA` (`tests/test_guardie_memoria.py`) | inconsistent | five spellings of «rewrite the expected file» |
| `MZLAB_UPDATE_FIGURA` (`msn_figura_tests.rs`) | inconsistent | Italian noun in an English family |
| `MZLAB_DATA` (`mzlab-desktop/examples/serve.rs`) | unclear | one letter from `MZLAB_DATI` (the private data repo) and a different meaning (the app folder) |
| `MZLAB_DATI` | fine | in AGENTS.md and 120 files; the private repo folder is `dati/` |
| `MZLAB_COMMIT`, `MZLAB_RUSTC`, `MZLAB_TARGET`, `MZLAB_CARGO_LOCK_SHA256` (build), `MZLAB_PHASES`, `MZLAB_CACHE`, `MZLAB_GOLDEN_STRICT`, `MZLAB_CHROMIUM`, `MZLAB_IN` («Run on CLI» shell variable) | fine | |
| about 60 other test-only variables (`MZLAB_PIPE_SITE`, `MZLAB_ORO_LIVELLI`, `MZLAB_ENORMI_GB`, …) | not scored | each is set by one test or tool; not typed by hand in normal work |

### 2.6 `tools/` options
Italian and documented in AGENTS.md (`pre_pr.py --veloce --completo --senza-e2e`, `verifica.py --cambiati --solo --rapida --fumo --parte --paralleli --ultimi-falliti --setup --browser`, `golden.py --crea --controlla`, `controlla.py --aggiorna`, `ci_metrics.py --da --giorni …`, `fondi_main.py --veloce --senza-controlli`, `converti.py --uscita --prova`). Short and clear for the team. One unclear: `verifica.py --tutto` means «also print the OK lines» (`verifica.py:427`), while `pre_pr.py --completo` means «run everything»: a reader takes `--tutto` for the second.

## 3. Grammar
1. **Language**: English for every name the engine and the CLI read or print; `tools/` options stay Italian (team tools, §5 D4).
2. **Order**: commands are `noun` or `noun-noun` with the object first (`msn-graph`, `ion-pattern`), subcommands are verbs (`run`, `check`, `build`, `sum`); flags are a noun or an adjective (`--out`, `--clean`), a verb only for an action (`--annotate`).
3. **Singular** in commands and step nouns (`peak-edges`, not `peaks-edges`), plural only for a list value (`--names`, `lists`).
4. **Case**: commands and flags kebab-case; step ids `family.noun[_noun]`; parameters snake_case; environment variables `MZLAB_UPPER_SNAKE`.
5. **One spelling per concept** (shared abbreviation list): `ppm` (m/z window in ppm, always the suffix: `tol_ppm`, `ppm` when the step has one window), `da` (in Da: `tol_da`), `rt` (retention time), `tol` (tolerance), `min`/`max` as prefix of a count or threshold (`min_points`, `max_gap`) and as suffix only for a range bound pair (`mz_min`/`mz_max`), `ms2`, `mz`, `seed`/`*_seed` for random seeds, `UPDATE` for «rewrite the expected file» in variables. No other abbreviation (no `toll`, `prec`, `att`).
6. **Length targets**: command ≤ 12, subcommand ≤ 8, flag ≤ 14 without the dashes, parameter ≤ 20, step id ≤ 20, environment variable ≤ 24. Longer only when a shorter name would need a reader to guess.
7. **Aliases**: an old name stays accepted on input forever, is never printed except in one «old names still accepted» line of `--help`, and the alias table lives next to the parser.

## 4. Rename table (input-only; aliases keep every old name)
| Kind | Old | New | Gain |
|---|---|---|---|
| command | `canali` | `channels` | English (+2 letters, accepted for consistency) |
| flag | `canali --canale` | `channels --channel` | English |
| command | `campioni` | `samples` | English, −1 |
| subcommand | `campioni variabili` | `samples vars` | English, −5 |
| subcommand | `campioni nome-gruppo` | `samples group` | English, −6 |
| command | `peaks-edges` | `peak-edges` | singular, −1 |
| `--set` names | `peak-edges` `areaNoise edgeFrac snMin minHalf` | `area_noise edge_frac sn_min min_half` | snake_case like every `--set` (the others are unchanged) |
| subcommand | `msn-rules somma` | `msn-rules sum` | English, −2 |
| flag | `flow run --jsonl` | `--events` | says what comes out (`flowlive/1` events), no clash with `--json`; also accepted by `mzlab_core::flow::line` |
| help placeholder | `--roles CAMPIONI.json` | `--roles ROLES.json` | names what the file is |
| flag | `msn-graph --mz-basso` / `--ppm-basso` | `--low-mz` / `--wide-ppm` | English; the second is the wider window used below `--low-mz` |
| flag | `--prove-min` | `--min-evidence` | English, grammar 5 |
| flag | `--attivazione` | `--activation` | English |
| flag | `--isotopi` | `--isotopes` | English |
| flag | `--scelta MZ=ION` | `--pick MZ=ION` | English, −2 |
| flag | `--verita` / `--riassunto` | `--truth` / `--summary` | English |
| flag | `--pulito` (graph and lib) | `--clean` | English, = step param `clean` |
| flag | `--nodo` / `--strutture` | `--node` / `--structures` | English |
| flag | `--interpreta` | `--interpret` | English |
| flag | `--ppm-perdita` | `--loss-ppm` | English |
| flag | `--regole` (graph and rules) | `--rules` | English |
| flag | `--toll-perdita-mda` | `--loss-tol-mda` | English, −5 |
| flag | `--figura` / `--vista` | `--figure` / `--view` | English, = step params `format`/`view` of `msn.figure` |
| flag value | `--vista albero\|radiale`, `--strutture tutti` | `tree\|radial`, `all` | English; the value written into the output stays the old one |
| flag | `--rotture` / `--max-legami` / `--ppm-struttura` | `--breaks` / `--max-bonds` / `--struct-ppm` | English; `max-bonds` = `spectra.insilico.max_bonds` |
| flag | `msn-lib --annota --formato --tutti --min-prove` | `--annotate --format --all --min-evidence` | English, = step params of `msn.library_export` |
| flag | `msn-rules --attivazione --lista` | `--activation --list` | English |
| env | `MZLAB_WRITE_GOLDEN`, `MZLAB_GOLDEN_WRITE`, `MZLAB_REGEN_GOLDEN` | `MZLAB_UPDATE_GOLDEN` | one verb |
| env | `MZLAB_WRITE_FIXTURE` | `MZLAB_UPDATE_FIXTURE` | one verb |
| env | `MZLAB_GUARDIE_CREA` | `MZLAB_UPDATE_GUARDS` | one verb, English |
| env | `MZLAB_UPDATE_FIGURA` | `MZLAB_UPDATE_FIGURE` | English |
| env | `MZLAB_DATA` (desktop `serve`) | `MZLAB_APP_DIR` | no near-clash with `MZLAB_DATI` |
| tools | `verifica.py --tutto` | `--mostra-ok` | says what it does |

Kept on purpose: `explain-missing`, `normative`, `--budget-mb`, `--dry-run`, `--hide-names`, `MZLAB_DATI`, every step id, every step parameter, every schema id and output file name (§1, §5).

## 5. Decisions for Federico (yes/no, with recommendation)
- **D1** Rename persisted step parameters (`spectra.msn_graph` Italian params, `meta.compare` tolerances, the ppm/Da/RT spellings)? Needs a new mechanism: step `@2` with a Rust migration `@1 → @2` that `flow::check`, `mzrun` presets (`preset.param.migrated`) and saved plans go through, plus `msngrafo/2` for the `opzioni` keys. **Recommend: yes for `spectra.msn_graph` only** (14 Italian names in one step, prompt N04, gated); no for the rest now, grammar applies to new parameters.
- **D2** Rename user-facing step ids (`tp.dig_lr`, the 17–20-char ids)? They are in templates, saved plans, `mzrun/1`, cache keys and the page. **Recommend: no.**
- **D3** Rename Italian schema ids and output files (`corsa.json`/`corsa/1`, `canali/1`, `campioni/1`, `errore/1`, `strumenti/1`, `contesto/1`, `mzrun/1` `tipo_bianco`)? Each needs a format version and readers of both. **Recommend: no**, only when a format changes for another reason.
- **D4** Move `tools/` options to English? **Recommend: no** (Italian team tools, documented in AGENTS.md; only `--tutto` changes).
- **D5** `canali` → `channels` is two letters longer: accept consistency over length? **Recommend: yes.**

## 6. Risks
- `mzlab_core::flow::line` parses a pasted `flow run` line (`line.rs:241-249`) for the site and the app: a new `flow run` flag must be added there too, or «Run on CLI» rejects it (parity by construction, one Rust source).
- Python and e2e tests (`tests/test_msngrafo_wasm.py`, `tests/test_msnfigura_wasm.py`, `tests_e2e/e2e_msn_figura.py`, `tools/convalida_regole.py`) call the old flags: they stay as they are and double as alias tests.
- The `msn-graph` flags become English while their step parameters stay Italian until D1: `--set g.mz_basso=` and `--low-mz` name the same value. The `--help` of `msn-graph` shows the step parameter next to each flag until N04.
- `--help` texts change (tested in `src/tests.rs`, `*_tests.rs`); no data output changes. Every PR checks `git diff --stat origin/main -- tests/golden docs/schemi` is empty.
- The output keys that the `msn-graph` flags add (`strutture`, `rotture`, `regole`, `interpretazione`, `vista`) are `msngrafo/1` fields and stay as they are (D3).
- Not verified: `insilico pack --piece-raw` (a byte size, kept); the ~60 test-only variables of §2.5 one by one; recipes outside the repository (none are known to use the renamed flags).
