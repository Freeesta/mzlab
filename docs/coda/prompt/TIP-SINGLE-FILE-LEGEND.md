Read docs/coda/BRIEF.md.

# TIP-SINGLE-FILE-LEGEND: no file name where one file is loaded, beyond the hover box

**Goal.** With one file loaded, the chromatogram hover box already shows only time, colour dash and intensity (`hovName` in `mzlab/web/grafici.js`, `oneFile()`). Apply the same rule, from the same helper, to the other places that repeat the file name with a single file, once the owner has said which.

**Why.** The file name is noise when there is only one file. Found while doing the hover box: the legend chips (`legend()` and the `data-h` chips in `grafici.js`), the XIC lens (`s.file` in `xic-lente.js`) and the file label in the peak label box and integration records (`picchi.js`, `file:`). The UV spectrum box already hides it with one series.

**Read first.** `docs/agenti/data.md` (hover box line), `tests_e2e/e2e_tooltip_un_file.py`.

**Done means.** 1. Owner decision recorded for each place (legend chips keep the name as the only identification of a TIC; the report and exports keep it). 2. Each place uses `hovName` or `oneFile` (no copies). 3. An e2e step per place: one file no name, two files name. 4. Start-up scripts cap respected (`tests/test_peso_avvio.py`): put code in a lazy script if needed.

**Out of scope.** Engine, exports, reports and the file list. Sibling experiments of one raw file count as separate loaded files today: decide whether they should count as one.

**Follow-ups.** Closing step of the brief.
