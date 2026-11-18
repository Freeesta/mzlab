# Sample metadata grid (Flow, `flow/lazy/flow-meta-grid.js`)

English only. A virtual-row ARIA grid (`role=grid`, `aria-rowcount`, `aria-colindex`; 28 px rows, a window of about 13 rows plus 4 of margin, the active row always exists) on top of a `campioni/3` document (`samples.md`). All logic is Rust, reached with `CDOC.post("<op>")` (`api/campioni/<op>`): the file lays out the cells and moves the focus.

## Document and saving
- `F.metaGrid(S, input)` (`F.samplesEditor` calls it): the document lives in `S.samples[input.name]`; a `/2` is migrated with `migra`. With no document and with files open in Data, it opens by itself: `F.samplesFromData` → `migra` → `dal_file` with polarity, ranges, scans and instrument read from `E.files`.
- Every edit shows at once; 120 ms after `F.samplesSet` it computes the canonical SHA-256 (`CDOC.canonical`, in `input.sha256`) and `valida` redraws the warnings. The saved document therefore has the same SHA as `CDOC.canonical`.
- `F.metaImport(S, input, testo)`: `csv_leggi` (`,` `;` tab; `F.META_HEADERS` renames "Time (min)", "Series"...) and merge by file+sample or SHA-256: known rows are overwritten keeping the file data, new ones are added, free columns are merged.

## Columns
- Fixed and editable: `file`, `campione`, `campione_id`, `tipo`, `matrice`, `serie`, `gruppo`, `tempo`, `unita_tempo`, `replica`, `lotto`, `concentrazione`, `unita`, `note`; then the free columns of `colonne` (text, number, category), then read-only polarity, re-injection, acquired, instrument, m/z and RT range, scans.
- "Columns...": add, rename (the `extra` follow), change type and allowed values, required, delete. A value that does not respect the type is not rejected: `valida` flags it (`typeMismatch`).

## Keyboard and mouse
- Arrows, Home/End (Ctrl = first/last row), PageUp/PageDown move; Shift extends the range; Enter, F2 or a character opens the cell (a list takes the first entry that starts with the letter); Enter confirms and goes down, Tab confirms and goes right, Esc cancels and returns to the cell; Ctrl+D fills down (row above, or the first of the range); Del clears; Ctrl+C copies as TSV; Ctrl+V pastes a TSV block from Excel starting at the active cell (read-only cells, cells outside the grid or invalid ones are counted and reported). Tab from outside enters the active cell; no trap.
- Sort with the header (`aria-sort`), filter with column + text (removable chips), group by matrix, group or condition (border and alternating background on the groups). Targets of at least 28 px, colors only from CSS variables.

## Warnings and Rust
- `valida` → warnings per cell (`colonna`, `file` = row index): `✖`/`⚠` mark plus border, `aria-invalid` on errors, a row with a missing pair (`pairMissing`) with a dashed border; the active cell writes its warning in a `role=status` and a second `role=status` summarizes in words ("6 times guessed from file names: check them.", "N other problems to fix.", "N warnings.", "No problems found.", plus the series warnings).
- Times read from names (`timeUnconfirmed`) have the "Confirm guesses" button (`mg.confirm`, visible only if there are any): it sets `tempo_confermato: true` on all those rows at once and `valida` rechecks; the severity in Rust does not change.
- "Fill from name pattern..." (`modello`, default `{campione_id}`): preview of the first 8 rows with the captured parts (`posizioni`) and the names with no match; "Apply" writes. "Read file data" (`dal_file`) and "Generate metadata" from Data; "Export CSV" (`csv_scrivi`, separator of choice) and "Download JSON".

## Tests
`tests_e2e/e2e_flow_meta_grid.py` (240 rows, pastes 240×6 in under 300 ms, SHA, keyboard, sort, filter, free column, pattern, CSV, axe), `e2e_flow_import`, types `tools/tipi/pagina.json`. The site with the engine: `python3 tools/prepara_sito.py` (it does not watch `flow/`: delete `site/static/wasm/mzlab_wasm_bg.wasm` to prepare it again).
