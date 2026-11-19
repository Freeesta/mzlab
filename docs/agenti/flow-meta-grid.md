# Sample metadata grid (Flow, `flow/lazy/flow-meta-grid.js`)

English only. A virtual-row ARIA grid (`role=grid`, `aria-rowcount`, `aria-colindex`; 28 px rows, a window of about 13 rows plus 4 of margin, the active row always exists) on top of a `samples/1` document (`samples.md`). All logic is Rust, reached with `CDOC.post("<op>")` (`api/samples/<op>`): the file lays out the cells and moves the focus.

## Document and saving
- `F.metaGrid(S, input)` (`F.samplesEditor` calls it): the document lives in `S.samples[input.name]`; any other document is not a `samples/1` and the grid starts empty. With no document and with files open in Data, it opens by itself: `F.samplesFromData` → `propose` → `from_file` with polarity, ranges, scans and instrument read from `E.files`.
- Every edit shows at once; 120 ms after `F.samplesSet` it computes the canonical SHA-256 (`CDOC.canonical`, in `input.sha256`) and `validate` redraws the warnings. The saved document therefore has the same SHA as `CDOC.canonical`.
- `F.metaImport(S, input, text)`: `csv_read` (`,` `;` tab; `F.META_HEADERS` renames "Time (min)", "Series"...) and merge by file+sample or SHA-256: known rows are overwritten keeping the file data, new ones are added, free columns are merged.

## Columns
- Fixed and editable: `file`, `sub_sample`, `sample_id`, `kind`, `matrix`, `series`, `group`, `time`, `time_unit`, `replicate`, `batch`, `concentration`, `concentration_unit`, `note`; then the free columns of `columns` (text, number, category), then read-only polarity, re-injection, acquired, instrument, m/z and RT range, scans.
- "Columns...": add, rename (the `extra` follow), change type and allowed values, required, delete. A value that does not respect the type is not rejected: `validate` flags it (`typeMismatch`).

## Keyboard and mouse
- Arrows, Home/End (Ctrl = first/last row), PageUp/PageDown move; Shift extends the range; Enter, F2 or a character opens the cell (a list takes the first entry that starts with the letter); Enter confirms and goes down, Tab confirms and goes right, Esc cancels and returns to the cell; Ctrl+D fills down (row above, or the first of the range); Del clears; Ctrl+C copies as TSV; Ctrl+V pastes a TSV block from Excel starting at the active cell (read-only cells, cells outside the grid or invalid ones are counted and reported). Tab from outside enters the active cell; no trap.
- Sort with the header (`aria-sort`), filter with column + text (removable chips), group by matrix, group or condition (border and alternating background on the groups). Targets of at least 28 px, colors only from CSS variables.

## Warnings and Rust
- `validate` → warnings per cell (`column`, `file` = row index): `✖`/`⚠` mark plus border, `aria-invalid` on errors, a row with a missing pair (`pairMissing`) with a dashed border; the active cell writes its warning in a `role=status` and a second `role=status` summarizes in words ("6 times guessed from file names: check them.", "N other problems to fix.", "N warnings.", "No problems found.", plus the series warnings).
- Times read from names (`timeUnconfirmed`) have the "Confirm guesses" button (`mg.confirm`, visible only if there are any): it sets `time_confirmed: true` on all those rows at once and `validate` rechecks; the severity in Rust does not change.
- "Fill from name pattern..." (`modello`, default `{sample_id}`): preview of the first 8 rows with the captured parts (`positions`) and the names with no match; "Apply" writes. "Read file data" (`from_file`) and "Generate metadata" from Data; "Export CSV" (`csv_write`, separator of choice) and "Download JSON".

## Tests
`tests_e2e/e2e_flow_meta_grid.py` (240 rows, pastes 240×6 in under 300 ms, SHA, keyboard, sort, filter, free column, pattern, CSV, axe), `e2e_flow_import`, types `tools/tipi/pagina.json`. The site with the engine: `python3 tools/prepara_sito.py` (it does not watch `flow/`: delete `site/static/wasm/mzlab_wasm_bg.wasm` to prepare it again).
