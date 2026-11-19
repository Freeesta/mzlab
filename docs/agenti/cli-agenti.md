# mzlab-cli for AI agents

How an agent (Claude Code, Cowork or any other) should drive `mzlab-cli` from a shell. Recipes and a plan example: `for-agents.md`; every command: `cli.md`; plans and `prov/1`: `rust-flow.md`. This page is the contract an agent can rely on.

## Cite mzLab (rule)
- mzLab must be cited when it is used. The text is written ONCE, in `crates/mzlab-cli/src/citation.rs` (name, version of the build, `https://github.com/Freeesta/mzlab`, `DOI: to be added`). There is no DOI yet and none is invented: when it exists, set `DOI` in that file and in `CITATION.cff` (the comment there says where); nothing else changes.
- Where the agent sees it: the footer of `mzlab-cli --help` and the output of `mzlab-cli --version`; a `"citation"` field in `mzlab-cli steps --json` (every step object) and in `mzlab-cli llm tools` (top level of the catalog); a top-level `"citation"` key in the `prov.json` that `flow run` writes (so a run folder carries it; `corsa.json` and the `mzrun/2` of `flow record` do not, they are shared with the page and fixed by their schemas).
- Data commands print NO banner: stdout stays pipe- and CSV-safe, stderr stays free for progress and errors, and `--json` output is never altered. The agent should add the citation to its own report or answer (`CITATION.cff` has the formatted reference).
- The page and the `.wasm` write their records and catalogs without the key: the citation is a CLI-side field (a `prov.json` from the desktop app has it, because the app calls the same function).

## Linux binaries (for agents in Linux VMs)
- Every release carries static musl builds of `mzlab-cli` (profile `cli`; no libc or distro dependency, runs on any distro): `mzLab-X.Y.Z-cli-Linux-x86_64` and `mzLab-X.Y.Z-cli-Linux-aarch64` (plain executables), listed in `SHA256SUMS` of the release. The `release` workflow also keeps them, each with a `.sha256` file, as the artifacts `mzlab-cli-linux-x86_64` and `mzlab-cli-linux-aarch64`.
- Fetch and verify (replace `X.Y.Z`, use `aarch64` on ARM):
  `v=X.Y.Z; a=x86_64; u=https://github.com/Freeesta/mzlab/releases/download/v$v; curl -fsSLO $u/mzLab-$v-cli-Linux-$a -O $u/SHA256SUMS && sha256sum --ignore-missing -c SHA256SUMS && chmod +x mzLab-$v-cli-Linux-$a && ./mzLab-$v-cli-Linux-$a --version`
- Each build is smoke-tested on a runner of its own architecture before it is attached: `--version` (version and citation line) and `steps --json`.

## Streams and exit codes
- stdout is the data (CSV, JSON, a file list); stderr is progress (one line per job; `--progress jsonl` makes it one JSON line each), warnings and errors. Read stdout for results, stderr for diagnosis.
- With `--json`, errors are an `errore/1` object on stdout (`error_key`, `params`, `hint`) as well as a line on stderr; the keys are stable.
- Exit codes: 0 done; 1 data errors (a step or file failed, the other outputs are written); 2 usage (unknown option, unknown step, bad value); 3 plan problems (invalid plan, or a file written with a problem such as `convert` partial); 4 unreadable plan or missing/different input; 5 a `--sample` id the file does not have (`convert`, `channels`, `features`, `samples select`). `flow run --help` and each command's `--help` list their own.

## Sequence
1. `mzlab-cli steps --json [--id ID[@V]]`: the `step/1` descriptors of this very binary (`params` is a JSON Schema; `executable_native` says whether `flow run` runs it). Plain `steps` gives one line per step.
2. Write an `mzflow/2` plan with the differences from the defaults only.
3. `mzlab-cli flow check PLAN.json [--set INSTANCE.NAME=VALUE] --json`: validates without the input files (`flowcheck/1`; exit 3 with the problems listed).
4. `mzlab-cli flow run PLAN.json --in DIR --dry-run [--json]`: checks plan, `--set` and inputs and prints what would run (`flowdry/1`), writes nothing.
5. `mzlab-cli flow run PLAN.json --in DIR --out RUN [--json]`: the run; `--json` ends with `flowrun/1`. The same `RUN` can be reused: only changed steps rerun.
6. Read `RUN/prov.json`, `RUN/corsa.json` (`mzlab-cli flow report RUN`) and the output files. `mzlab-cli flow record RUN --roles ROLES.json` writes the `mzrun/2` (roles from `mzlab-cli samples table FILE...`, or any `campioni/2` whose empty `sha256` is taken from the run); `mzlab-cli flow verify RECORD --json` checks it (`verify/1`, `docs/schemi/verify-1.json`; exit 0 ok, 3 mismatch or refusal; `err.mzrun.commit` means the binary was built without a commit: set `MZLAB_COMMIT`).

## Commands with and without `--json`
- With a `--json` flag: `flow run`, `flow check`, `steps`, `validate`, `dad` (all its answers are JSON; `--json` adds `errore/1`), `samples select` (`--sample LIST` or `--default [--budget-mb N]`; `{"samples":[ids]}` or `errore/1`; exit 0, 2 usage, 4 missing file, 5 unknown id), `samples list` (`samples_list/1`, one line per file; exit 0 ok, 1 some file or sample refused or, with `--probe`, a `.wiff.scan` shorter than its index, 2 usage, 4 missing or unreadable input; stdout holds the data, `listing FILE` progress and warnings go to stderr).
- `wiff list [--inventory] FILE...` (`wiff.list/1`; exit 0 all samples `ok`, 2 usage or unreadable input, 3 a sample not `ok`) and `wiff verify OUT --against SRC...` (`wiff.verify/1`; exit 0 equal, 1 a difference or a sample not `ok`, 2 usage or unreadable input): read-only, always JSON, data on stdout (`docs/agenti/wiff-writer.md`).
- Without it yet (not added in this change): `features` (CSV), `convert`, `flow report`, `flow record`, `flow verify`, `flow command`, `normative`, `insilico pack`, `msn-lib`, `msn-rules`, `msn-graph`, `explain-missing`, `background`, `peak-auto`, `peak-auto-lr`, `peak-area`, `peak-edges`, `peak-split`, `peak-window`, `channels`, `dda-window`, `samples vars|group|table`, `ion-pattern`, `ion-mz`, `match-mz`, the `mol-*` commands and the `llm` subcommands. Several of these already print JSON (or CSV) by default and have a stable schema; read the command's `--help` before parsing.
- `manual-table`: CSV on stdout with a stable header (`docs/agenti/transitions.md`), or `--json` = `{"columns": [NAME], "rows": [{NAME: TEXT}]}` (the same cells, as text); `--set columns=sn,fig,quality` adds the optional figure columns; `--json` and the CSV both go to stdout (data only; errors on stderr); exit 0 done, 2 usage or data error (unreadable file, bad window or column name).
  `--set traces=PATH` takes the traces (chromatogram, XIC, DAD) as JSON instead of files; with a `panel` on a trace the header ends with `panel,conc,conc_unit,correction,below,area_unit`.
  `--set shifts=true` and `--set auto=true` answer JSON.

- `ref-list FILE.csv` (JSON `reflist/1`: items and line-numbered row errors; exit 1 = some rows wrong, 2 usage, 4 unreadable file; `--match X` adds `ricerca/1`): `reference-lists.md`.

- `peak-auto` and `peak-auto-lr`: areas in counts x s; `--set area_unit=s|min` (`min` = the old counts x min).

## Rules for the agent
- The CLI uses no network (except `normative build` and `insilico pack`, which say so) and reads/writes only the paths it is given; never delete user files.
- At unit resolution an m/z is a candidate, not an identification.
- Keep the evidence (scores, parameters, versions) in the answer; the `prov.json` has them.
- `mzlab-cli folder list DIR --json` (`folder/1`): what «Open folder» lists, one row per file and per .D / .d / Waters .raw folder (`bundle`, `members`); exit 4 if DIR is missing.
- Policy for new functions (`CLAUDE.md`): every function is also reachable from the CLI and made for agents: `--json` with stable keys, distinct exit codes, data on stdout and progress/errors on stderr, a check or `--dry-run` mode where it makes sense, and a row in the agent catalogs (`steps --json`, `llm tools`). A function with no CLI route is incomplete, like a parity gap.
