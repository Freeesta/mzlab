# English migration: what is still Italian

Measure it with `python3 tools/check_english.py` (counts per area; `--elenco` one line per file; `--file PATH` every hit). Counts at `main` 7f973633:

| area | files | with hits | comment lines | identifiers | keys | file names |
|---|---|---|---|---|---|---|
| `crates/` | 2829 | 950 | 5 | 2813 | 1466 | 258 |
| `mzlab/web/` | 511 | 420 | 4 | 1193 | 1697 | 354 |
| `flow/` | 82 | 29 | 0 | 50 | 48 | 1 |
| `TP_Mine/` | 52 | 35 | 1 | 40 | 40 | 15 |
| `tools/` | 410 | 346 | 4 | 1170 | 2173 | 236 |
| `tests_e2e/` | 693 | 622 | 0 | 304 | 1159 | 500 |
| `tests/` | 969 | 750 | 1 | 395 | 1394 | 647 |
| `docs/` | 348 | 346 | 0 | 0 | 53 | 346 |

## Families left, smallest risk first (each one is a PR with a guard test, no aliases, no converters)
1. Tool flags and docs prose: `tools/html_safe.py --aggiorna`, `misura`/`peggiori`, the «HTML sicuro» heading of `data.md`.
2. Panel action registry (`azioni/1`, `AZIONI`): the fields unique to the registry files are done (`group`, `views`, `visible`, `available`, `shortcut`, `doubleClick`, `download`). Still open: the contested names (`nome`, `esegui`, `vista`, `voci`, `ordine`, `icona`, `sotto`, `attiva`) and the API (`registra`, `voce`, `per`, `elenco`, `TASTI`). The calculator API is done.
3. Language area names under `mzlab/web/lang/en/` (file names such as `errori-flow.js`).
4. Schema ids and their keys: `livelli/1`, `ipotesi/1`, `riga/1`, `riga_tracce/1`, and the hypothesis source labels `famiglia` / `importata`.
5. Rust identifiers (`crates/`: `mzlab-feat`, `-ops`, `-cli`, `-core`): by crate, rename-only, parity tests unchanged.
6. File and directory names (`tests*/`, `tools/`, `mzlab/web/*.js`, `TP_Mine/`) and the format name `regole_tp` itself.
7. `cancello/2` (deferred by the coordinator, untouched).

## Kept on purpose
- External names: `lit:ARCHITETTO_TP_FOTOCATALISI 1.2` (a workbook name) and any real literature or vendor name.
- The gate `tools/gate_release.py` and the generated `docs/agenti/mappa.md` follow the families above, never by hand.
