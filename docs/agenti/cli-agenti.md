# mzlab-cli for AI agents

How an agent (Claude Code, Cowork or any other) should drive `mzlab-cli` from a shell. Recipes and a plan example: `for-agents.md`; every command: `cli.md`; plans and `prov/1`: `rust-flow.md`. This page is the contract an agent can rely on.

## Cite mzLab (rule)
- mzLab must be cited when it is used. The text is written ONCE, in `crates/mzlab-cli/src/citation.rs` (name, version of the build, `https://github.com/Freeesta/mzlab`, `DOI: to be added`). There is no DOI yet and none is invented: when it exists, set `DOI` in that file and in `CITATION.cff` (the comment there says where); nothing else changes.
- Where the agent sees it: the footer of `mzlab-cli --help` and the output of `mzlab-cli --version`; a `"citation"` field in `mzlab-cli flow steps --json` (every step object) and in `mzlab-cli llm tools` (top level of the catalog); a top-level `"citation"` key in the `prov.json` that `flow run` writes (so a run folder carries it; `report.json` and the `mzrun/2` of `flow record` do not, they are shared with the page and fixed by their schemas).
- Data commands print NO banner: stdout stays pipe- and CSV-safe, stderr stays free for progress and errors, and `--json` output is never altered. The agent should add the citation to its own report or answer (`CITATION.cff` has the formatted reference).
- The page and the `.wasm` write their records and catalogs without the key: the citation is a CLI-side field (a `prov.json` from the desktop app has it, because the app calls the same function).

## Which build is this
- `mzlab-cli --version` prints `mzlab-cli X.Y.Z (commit abc1234)` (the git short SHA the binary was built from: `MZLAB_COMMIT`, else git at build time; `unknown` when the build knew neither) and the citation line; `mzlab-cli --version --json` is `version/1` (`name`, `version`, `commit` full, `commit_short`, `citation`; `commit` is `null` when unknown). There is no build date: two builds of one commit answer alike, so no output depends on the clock. Compare `commit` with the repository before trusting a command list from memory.

## Renamed commands
- A command name of an earlier build (`channels`, `items`, `manual-table`, ...) is exit 2 with `error/1` `err.cli.command.renamed` (`params.old`, `params.now`) and the new path in the message and in `hint`; `commands --json OLD` (exit 5) puts it in `hint` too. A command that was removed answers `err.cli.command.removed`. The table is `RENAMED` in `crates/mzlab-cli/src/renamed.rs`:
<!--renames-->
| old name | now |
|---|---|
| `background` | `trace background` |
| `channels` | `file channels` |
| `convert` | `file convert` |
| `dda-window` | `spectrum dda-window` |
| `elements` | `ion elements` |
| `folder` | `file list` |
| `insilico` | `db insilico pack` |
| `ion-mz` | `ion mz` |
| `ion-pattern` | `ion pattern` |
| `items` | `file items` |
| `manual-table` | `peak table` |
| `match-mz` | `ion match` |
| `mol-anal` | `mol anal` |
| `mol-check` | `mol check` |
| `mol-cip` | `mol cip` |
| `mol-cut` | `mol cut` |
| `mol-export` | `mol export` |
| `mol-hilite` | `mol hilite` |
| `mol-label` | `mol label` |
| `mol-link` | `mol link` |
| `mol-mech` | `mol mech` |
| `mol-search` | `mol search` |
| `mol-sel` | `mol sel` |
| `msn-graph` | `msn graph` |
| `msn-lib` | `msn lib` |
| `msn-rules` | `msn rules` |
| `normative` | `db normative` |
| `pair` | `file pair` |
| `peak-area` | `peak area` |
| `peak-auto` | `peak auto` |
| `peak-auto-lr` | `peak auto-lr` |
| `peak-edges` | `peak edges` |
| `peak-split` | `peak split` |
| `peak-window` | `peak window` |
| `ref-list` | `db ref-list` |
| `samples` | `sample` |
| `steps` | `flow steps` |
| `validate` | `file validate` |
| `wiff` | `file inventory` |
| `explain-missing` | removed (no replacement) |
<!--/renames-->

## Look at a file first
- `file info FILE...` answers in one call what a file holds (container, samples, `items/1`, `channels/1`, proposed kind). Then `trace chrom FILE --id ID --format csv | peak auto-lr -`, `trace xic`, `spectrum get|list`: JSON starts with `"schema"`, CSV is `rt_min,intensity`; unknown channel id or scan: exit 5. Wiff inventory: `file inventory`.

## Linux binaries (for agents in Linux VMs)
- Every release carries static musl builds of `mzlab-cli` (profile `cli`; no libc or distro dependency, runs on any distro): `mzLab-X.Y.Z-cli-Linux-x86_64` and `mzLab-X.Y.Z-cli-Linux-aarch64` (plain executables), listed in `SHA256SUMS` of the release. The `release` workflow also keeps them, each with a `.sha256` file, as the artifacts `mzlab-cli-linux-x86_64` and `mzlab-cli-linux-aarch64`.
- Fetch and verify (replace `X.Y.Z`, use `aarch64` on ARM):
  `v=X.Y.Z; a=x86_64; u=https://github.com/Freeesta/mzlab/releases/download/v$v; curl -fsSLO $u/mzLab-$v-cli-Linux-$a -O $u/SHA256SUMS && sha256sum --ignore-missing -c SHA256SUMS && chmod +x mzLab-$v-cli-Linux-$a && ./mzLab-$v-cli-Linux-$a --version`
- Each build is smoke-tested on a runner of its own architecture before it is attached: `--version` (version and citation line) and `flow steps --json`.

## Streams and exit codes
- stdout is the data (CSV, JSON, a file list); stderr is progress (one line per job; `--progress jsonl` makes it one JSON line each), warnings and errors. Read stdout for results, stderr for diagnosis.
- Under `--json`, and for every command whose stdout is JSON, each non-zero exit writes exactly one `error/1` line on stdout (`code`, `error_key`, `params`, `message` without `error: `, `hint`, `command`, `problems` for an invalid plan) and the `error:`/`hint:` lines on stderr; the keys are stable (`docs/schemi/error-1.json`).
- `--json` is accepted by every command: a no-op where stdout is JSON already; a command with no JSON form fails with `err.cli.json.unsupported` (exit 2). `--help`/`-h` anywhere in the arguments of a command prints its help on stdout, exit 0. Every stdout document ends with `\n`.
- Exit codes (`crates/mzlab-cli/src/exit.rs`, one table for every command; the table is generated by `python3 tools/cli_doc.py`):
<!--exits-->
| code | meaning |
|---|---|
| 0 | ok |
| 1 | data error, partial outputs written |
| 2 | usage |
| 3 | plan, schema or integrity |
| 4 | input missing or unreadable |
| 5 | selector not found (sample, channel, node, step) |
| 6 | output write or external source failure |
<!--/exits-->
  A bug never ends as signal 134 (the release profile aborts on a panic): `main` answers first with exit 1, `err.run.internal` (the panic message and position in `params.detail`) and, under `--json`, one `error/1`; the test `no_command_panics_on_an_empty_or_garbage_input` runs every command on empty and garbage files. Bad or empty input is still exit 2 or 4 with its own key.
  Which codes a command can end with: `commands --json PATH` (`exit_codes`; the derived rule is in the listing's `exit_codes_note`). Every non-zero code is one `error/1` on stdout under `--json`.

## Learn the CLI in one call
- `mzlab-cli commands [--json] [PATH...]`: the `commands/1` catalog: per command its `synopsis` (the usage line of `--help`, from which `args` and `flags` are read, so help and catalog cannot differ), flags with type, unit, default and enum (an argument, a flag or a command leaves out `unit`, `default`, `enum` when null and `repeat`, `required`, `stdin` when false), `stdout` (`format` json|csv|text, `schema` when there is one), `exit_codes` (meanings once at the top) and examples. `PATH...` narrows it (`commands --json flow`); unknown path: exit 5.
  - The whole listing (no PATH) is compact, about 32 KB against the 40,000-byte cap: a flag used by two or more commands has its `help` once in the top-level `flag_help` (so a new command with `--out` or `--sample` costs nothing for them), `exit_codes` is left out when it is the one derived from the usage line (`derived_exits` in `catalog/doc.rs`), and `examples` when it is only the usage line without its `[optional]` parts.
  - The derived exit codes: 0, 1, 2 always; +3 when the path starts with `flow ` or is a verdict command (`VERDICT` in `crates/mzlab-cli/src/contract.rs`); +4 when it has a path or rest argument or a path flag other than `--out`; +5 when it has `--sample` or `--id` or its path starts with `sample `, `dad ` or `msn graph`; +6 when it has `--out`. The derived example is `mzlab-cli ` + the synopsis without its `[optional]` groups (a word with a bracket inside stays) and without `...` tokens. Both rules are also in the `exit_codes_note` / `examples_note` of the listing.
  - `commands --json PATH...` gives each command in full. To add a command: write its row and `FLAG_HELP` text as before; do not add members to the compact listing without moving something out (the test `the_whole_listing_is_compact_and_the_path_listing_is_full` holds the 33 KB line).
- `mzlab-cli schema [--list | NAME]`: the JSON Schemas of `docs/schemi/` embedded in the binary (`error/1`, `mzflow/2`, `prov/1`, `samples/1`, `design/1`, ...); `--list` is `schemas/1`. `mzflow/2` is the plan the engine reads: `program` and input fingerprints are optional. Unknown name: exit 5.
- `flow steps --compact` (`steps-compact/1`, under 20 KB: ids, versions, titles, ports, parameter and algorithm names, `executable_native`) and `llm tools --compact` (`tools-compact/1`) are the small forms; `llm tools` names the catalog in `cli_catalog`.

## Sequence
0. A plan with `table.stats@1` or `table.drift@1` needs `samples` and `design` inputs: `sample table FILE... > samples.json`, `sample design propose --samples samples.json --out design.json`, `sample design validate design.json --samples samples.json` (exit 3 names each bad key); `design-cli.md`.
1. `mzlab-cli flow steps --json [--id ID[@V]]`: the `step/1` descriptors of this very binary (`params` is a JSON Schema; `executable_native` says whether `flow run` runs it). Plain `flow steps` gives one line per step.
2. Write an `mzflow/2` plan with the differences from the defaults only.
3. `mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE] --json`: validates without the input files (`flowcheck/1`; exit 3 with the problems listed).
4. `mzlab-cli flow run PLAN.json --in DIR --dry-run [--json]`: checks plan, `--set` and inputs and prints what would run (`flowdry/1`), writes nothing.
5. `mzlab-cli flow run PLAN.json --in DIR --out RUN [--json]`: the run; `--json` ends with `flowrun/1`. The same `RUN` can be reused: only changed steps rerun.
6. Read `RUN/prov.json`, `RUN/report.json` (`mzlab-cli flow report RUN`) and the output files. `mzlab-cli flow record RUN --roles ROLES.json` writes the `mzrun/2` (roles from `mzlab-cli sample table FILE...`, or any `samples/1` whose empty `sha256` is taken from the run); `mzlab-cli flow verify RECORD --json` checks it (`verify/1`, `docs/schemi/verify-1.json`; exit 0 ok, 3 mismatch or refusal; `err.mzrun.commit` means the binary was built without a commit: set `MZLAB_COMMIT`).

## Commands with and without `--json`
`--json` is accepted by every command; the generated list says what it does for each (`python3 tools/cli_doc.py`, from the tables the contract reads):
<!--json-->
- JSON already (`--json` only makes a failure an `error/1`):
`flow report|record`, `file info|index|inventory|verify|items|channels`, `sample vars|group|table|list|select`, `sample design propose|validate|canonical`, `trace chrom|xic|background`, `dad subtract|spectrum|trace|ms-at|delay`, `spectrum get|list|dda-window`, `peak auto|auto-lr|edges|area|window`, `msn graph|complete|compare|figure`, `msn lib add|build|search`, `msn rules`, `msn rules sum`, `ion mz|pattern|match`
`mol label|anal|check|cip|hilite|sel|link|open|cut|mech|search`, `db ref-list`, `llm tools|inspect|params|context|replay|eval`, `schema`
- text or CSV, `--json` gives the document:
`flow run|check|steps|verify|command`, `file list|pair|validate|convert`, `peak table`, `msn lib export`, `ion elements`, `mol draw`, `db normative build`, `db insilico pack`, `commands`
- no JSON form (`--json` is exit 2, `err.cli.json.unsupported`):
`features`, `peak split`, `mol export`, `db normative pack|listings|import-official`, `completions bash|zsh|fish`
<!--/json-->
- `features` has no `--json` and no `--dry-run`; its machine-readable side channel is `features-notes/1` written beside `--out` (`features-notes.json` in a folder, `NAME.notes.json` beside `NAME.csv`; one entry per file with the notices the stderr lines carry; `cli-passi.md`).
- `sample list` (`sample-list/1`: ONE document, `files` in the order given, a refused file is an inline `error/1` entry, a missing file is a whole-run `error/1` (exit 4, no listing); exit 0 ok, 1 some file or sample refused or, with `--probe`, a `.wiff.scan` shorter than its index, 2 usage, 4 missing input; stdout holds the data, `listing FILE` progress and warnings go to stderr).
- `file validate --json` (`mzml-validation/1`, ONE document for all files; a missing file is exit 4 + `error/1`), `file convert --json` (`convert/1`; `--dry-run` lists inputs, readers and sizes and writes nothing; an `.mzML` named as input is `err.file.alreadyMzml`, exit 4) and `file items --no-points` (leaves out `ms2_events`).
- `file inventory [--full] FILE...` (`wiff.list/1`; exit 0 all samples `ok`, 2 usage, 4 unreadable input, 3 a sample not `ok`) and `file verify OUT --against SRC...` (`wiff.verify/1`; exit 0 equal, 1 a difference or a sample not `ok`, 2 usage, 4 unreadable input): read-only, always JSON, data on stdout (`docs/agenti/wiff-full-spec.md`).
- A command with no JSON form fails `--json` with `err.cli.json.unsupported` (exit 2, one `error/1`), never "unknown option". `db normative build` and `db insilico pack` print nothing on stdout without `--json`.
- `mzlab-cli --help` starts with "Start here" (`flow steps --json`, `flow check PLAN.json --json`, `flow run`, `COMMAND --help`), the contract for scripts and the exit codes; `features --help` is the help of `features` alone.
- On a failure after a command printed a document (a `flowrun/1` report, a `flowcheck/1`, a `verify/1`), `error/1` carries it as `report`: stdout is one JSON document either way. Every non-zero exit is one `error/1`, except the verdicts below.
- Verdict commands, by command path in `VERDICT` (`crates/mzlab-cli/src/contract.rs`): `file validate`, `file pair`, `file inventory`, `file verify`, `sample list`, `db normative build` (a stale list), `llm eval` (a failed check) and `db ref-list` (some rows wrong: good rows in `items`, bad in `errors`). Their exit 1 or 3 is the answer the site computes byte for byte, so the document stays on stdout with its own verdict (`ok`, `valid`, `probe`, `errors`, `stale`); `sample select` is not a verdict. Exit 2, 4, 5 and 6 of these are an `error/1` with no verdict (`file validate`, missing file: 4).
  - `db normative build`: exit 1 (a stale list) keeps its `normative-build/1`; exit 3 (pin mismatch) has no document and is an `error/1`.
- `peak table`: CSV on stdout with a stable header (`docs/agenti/transitions.md`), or `--json` = `{"columns": [NAME], "rows": [{NAME: TEXT}]}` (the same cells, as text); `--columns sn,fig,quality` adds the optional figure columns; `--json` and the CSV both go to stdout (data only; errors on stderr); exit 0 done, 2 usage or data error (unreadable file, bad window or column name).
  `--traces PATH` takes the traces (chromatogram, XIC, DAD) as JSON instead of files; with a `panel` on a trace the header ends with `panel,conc,conc_unit,correction,below,area_unit`.
  `--shifts` and `--auto` answer JSON.

- `db ref-list FILE.csv` (JSON `reflist/1`: items and line-numbered row errors; exit 1 = some rows wrong, 2 usage, 4 unreadable file; `--match X` adds `reflist-match/1`): `reference-lists.md`.

- `peak auto` and `peak auto-lr`: areas in counts x s; `--area-unit s|min` (`min` = counts x min). `peak auto` under `--baseline zero` (default) leaves out `b0`/`b1` and says so in `choices`; with `line` they are the line at `t0`/`t1`; `base_*` numbers without `--baseline line` are exit 2.

## Flow and llm commands
- `flow report RUNDIR` (`run-report/1`; exit 4 no folder or `prov.json`, 6 `--out` unwritable), `flow command PLAN.json --json` (`flowcommand/1`, the script in `script`; exit 0, 2, 4) follow the contract; `flow report` ends with one `\n`.
- `llm inspect FILE|DIR...` indexes the files in parallel (a folder gives its mzML by name, files keep the order given; same bytes for any number of workers). `llm params --inspection FILE|-` reuses an inspection (no indexing, its budget and `--hide-names` are the ones it was made with), so `llm inspect A B | llm params --inspection -` equals `llm params A B`; each proposal carries its rule, signals and confidence, and `choices` lists every automatic choice with its reason.
- `llm inspect`, `llm params` and `llm context`: exit 0 ok, 2 usage, 4 missing or unreadable input. `llm replay` (`replay/1`) adds 3 (a recording of the wrong format) and 1 (no turn left). A failure is one `error/1` under `--json`.
- `llm eval` (`evalrun/1`, one JSON document, a verdict command): exit 0 all checks pass, 1 a check failed (the document stays on stdout), 2 usage, 3 a case or recording of the wrong format, 4 missing file. A recording that runs out or does not answer its case is one `error/1` with exit 1.

## Chemistry commands (`mol *`, `ion *`)
- Every answer opens with `"schema"` (`label/1`, `analysis/1`, `check/1`, `cip/1`, `highlight/1`, `selection/1`, `link/1`, `cut/1`, `mechanism/1`, `substructure/1`, `substructure_db/1`, `draw/1`, `draw-image/1`, `ion_mz/1`, `ion_pattern/1`, `mz-match/1`, `elements/1`; `mzlab-cli schema NAME`), never `"format"`. Units sit in the key: `neutral_da`, `offset_da`, `abundance_pct`, `error_mda`, `error_ppm`.
- Exit codes: 2 for a formula, adduct or SMILES that does not parse or a bad option value; 1 for a valid request the chemistry cannot satisfy (adduct removes more atoms than exist, m/z below 1, cut of an aromatic bond, bond that is not in the molfile); 4 for a missing file or a request/molfile that cannot be read. 6 for an unwritable `--out`. Failures are one `error/1`. `mol export` prints text (no schema, no `--json`); `mol search` answers `substructure/1`, or `substructure_db/1` with `--db`. `error_mda` is in mDa. Schemas: `docs/schemi/*-1.json`.
- `mol draw SPEC [--format molfile|scene|ket|cdxml|svg|png]` (`draw/1`, `draw-image/1`, or a molfile, KET, CDXML or PNG without `--json`; `--set edit=JSON|@PATH` applies an edit patch, `draw_edit/1`): 1 over 2000 atoms, an atom that is no element; 2 unparseable SMILES, bad `--set`, a name/key without `--set db=PATH`, or a failed patch operation (`err.mol.draw.edit` with its index and reason); 4 missing file, patch file or db, a molfile or request that cannot be read; 5 name or key not in the db (the hint lists the nearest names); 6 unwritable `--out` (`mol-draw.md`).

## Rules for the agent
- The CLI uses no network (except `normative build` and `insilico pack`, which say so) and reads/writes only the paths it is given; never delete user files.
- At unit resolution an m/z is a candidate, not an identification.
- Keep the evidence (scores, parameters, versions) in the answer; the `prov.json` has them.
- `mzlab-cli file list DIR --json` (`folder/1`): what «Open folder» lists, one row per file and per .D / .d / Waters .raw folder (`bundle`, `members`); exit 4 if DIR is missing.
- Policy for new functions (`CLAUDE.md`): every function is also reachable from the CLI and made for agents: `--json` with stable keys, distinct exit codes, data on stdout and progress/errors on stderr, a check or `--dry-run` mode where it makes sense, and a row in the agent catalogs (`flow steps --json`, `llm tools`). A function with no CLI route is incomplete, like a parity gap.

## Index cache (`--cache DIR`, `MZLAB_CACHE`)
Every file command (`file info|items|channels`, `trace`, `spectrum`, `dad`, `msn graph`, `trace background`, `features`) builds a `.mzidx` per call. Opt-in, nothing is written where it was not asked: the global flag `--cache DIR` (declared once in the catalog `global_flags`, wins) or `MZLAB_CACHE` keeps each index in DIR and hands it to the next call.
- The flag goes after the command name (`--cache=DIR` and a `--cache` before the command are not accepted); `db normative build` keeps its own `--cache` (sources folder) and never sets the index cache. `file index FILE... [--sample ID]` pre-builds (`index-build/1`: `status` built|reused, `key`, `sha256`, `index_bytes`).
- Entry `SHA256-LABELHASH-f<format>-e<engine>.mzidx` (+ `.mzidx.sum`, the SHA-256 of the entry): the SHA-256 of the file as read now (dataset members listed by name), the sample, `idx_newer::VERSION_MAX`, the program version. The file is always hashed, so a file edited with the same size and time is a new entry; a cache hit still costs the hash and a copy, far less than the build.
- An entry is verified before use: its bytes against the checksum, then its header. Damaged (also with its length unchanged) or without checksum: stderr `warn.cache.stale`, rebuilt, replaced (logged choice). The entry is copied to the caller's index, never linked, so nothing a caller writes reaches the cache. Entry and checksum are renamed into place whole; `.tmp-*` files older than an hour (a crashed call) are removed at the next write; two processes on one entry both answer and leave one sound entry (`tests/idx_cache_processes.rs`).
- Folder not usable by a command that only reads, or an entry that cannot be replaced: `warn.cache.unusable`, built without keeping it; `file index` with no folder it can make or write ends with exit 6 (`err.run.output`). `--cache` without a folder: exit 2; unknown `--sample`: exit 5.
- stderr says `index reused: NAME` or `index built: NAME` per index (on an exit other than 0 the lines go straight to the terminal, so the `error/1` message stays alone); stdout bytes are the same with and without the cache (`idx_cache_tests.rs`). Native only: the site keeps its indexes in OPFS.
