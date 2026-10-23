# Azioni che cambiano stato (generato)

Generato da `python3 tools/doc_comandi.py` dalla tabella `COMANDI_TAB` di `mzlab/web/comandi.js`: non si modifica a mano. Ogni azione che cambia stato nella vista Dati sta nella tabella; `tests/test_comandi.py` lo controlla.

| Id | Nome | Modulo | Cosa sceglie l'utente | Stato che cambia | Tasto | Passi | Annullamento |
|---|---|---|---|---|---|---|---|
| `view.zoom` | zoom (`storia.zoom`) | `eventi.js` | m/z or RT interval, intensity interval | zoom, zoomY, lock | wheel, Backspace, double click, Shift+drag | a burst of wheel steps, a drag and a pan are one step | undo with a snapshot (storia.js) |
| `peaks.integration` | integrazione (`storia.int`) | `picchi.js` | limits of the integrated peak | ints, isel | I, Del | an edge drag is one step | undo with a snapshot (storia.js) |
| `panel.fields` | correzione (`storia.campi`) | `grafici.js` | blank, background, smoothing, correction, reference | bk, bg, fondo, smooth, corr, ref, bgr | - | - | undo with a snapshot (storia.js) |
| `spec.ruler` | righello (`cmd.spec.ruler`) | `spettro.js` | the reference peak and the second peak of each measure | meas, rul | R, Esc | - | command: do / undo handler (comandi.js) |
| `files.remove` | pannello (`storia.pannello`) | `lista-file.js` | the file | - | - | - | removal with its own undo / redo (STORIA.tolto) |
| `panel.close` | pannello (`storia.pannello`) | `pannelli.js` | the panel with its zoom, integrations and settings | - | - | - | removal with its own undo / redo (STORIA.tolto) |
| `map.marks.clear` | marcature (`storia.marcature`) | `mappa-tabella.js` | the marks of the map | - | - | - | removal with its own undo / redo (STORIA.tolto) |
| `layout.arrange` | Disposizione ripristinata (`aff.undone`) | `affianca.js` | position and size of the panels | - | Alt+arrows | - | arrangement of the panels (affianca.js) |
| `panel.source` | scelta del pannello (`cmd.source`) | `pannelli.js` | file, MS level, precursor, chromatogram kind, display mode, m/z limits, scale | k, kSet, level, prec, all, kind, mode, mz0, mz1, tr, mapKind, norm, scale, gamma, log | - | - | command: do / undo handler (comandi.js) |
| `spec.annotation` | nota (`cmd.annotation`) | `eventi.js` | add, edit, delete a note | anns | - | - | command: do / undo handler (comandi.js) |
| `spec.display` | opzioni di visualizzazione (`cmd.display`) | `visualizza.js` | threshold, labels, decimals, sticks, dots, markers, contaminants, gradient overlay | thr, nlab, dec, sticks, dots, mk, noCont, rel, gdelay, pdaDelay, pdaAlign, overlay | - | - | command: do / undo handler (comandi.js) |
| `spec.isotopes` | correzione (`storia.campi`) | `spettro.js` | isotope simulation and resolution | iso | - | - | NOT undoable yet |
| `spec.background` | fondo (`storia.fondo`) | `fondo-spettro.js` | background window of the spectrum | bw0, bw1, bwi | - | - | NOT undoable yet |
| `peaks.noise` | integrazione (`storia.int`) | `eventi.js` | noise stretch of an integration | noise | - | - | NOT undoable yet |
| `xic.traces` | correzione (`storia.campi`) | `pannelli.js` | add, remove, tolerance and unit of the XIC traces, split | traces | X | - | NOT undoable yet |
| `series.visibility` | correzione (`storia.campi`) | `grafici.js` | hide or show a series from the legend | hid, showHid | - | - | NOT undoable yet |
| `panel.link` | correzione (`storia.campi`) | `eventi.js` | freeze or relink the live spectrum | link, src | - | - | NOT undoable yet |
| `panel.rename` | pannello (`storia.pannello`) | `pannelli.js` | title of the panel | title | - | - | NOT undoable yet |
| `panel.open` | pannello (`storia.pannello`) | `pannelli.js` | create a panel (spectrum, XIC, extracted window) | - | - | - | NOT undoable yet |
| `map.marks` | marcature (`storia.marcature`) | `mappa.js` | add, move, delete, note, reference of a mark | - | - | a drag is one step | NOT undoable yet |
| `files.view` | correzione (`storia.campi`) | `lista-file.js` | visibility, name, colour, group, order of a file | - | - | - | NOT undoable yet |
| `samples.meta` | correzione (`storia.campi`) | `campioni-tabella.js` | type, replicate, concentration, unit, time; TSV import | - | - | - | NOT undoable yet |
| `peaks.settings` | correzione (`storia.campi`) | `picchi.js` | peak-finding parameters, options of the integration table | - | - | - | NOT undoable yet |
| `app.settings` | correzione (`storia.campi`) | `settings.js` | theme, language, font, contrast, palette, map preferences | - | - | - | NOT undoable yet |
| `flow.plan` | correzione (`storia.campi`) | `flow-shell.js` | steps and parameters of the plan | - | Ctrl+Z, Ctrl+Y | one entry per change | own stack of the Flow (S.undo) |
| `draw.editor` | correzione (`storia.campi`) | `draw.js` | the structure in the editor (Ketcher) | - | Ctrl+Z, Ctrl+Y | - | own stack of the editor (Ketcher) |
| `panel.selection` | correzione (`storia.campi`) | `eventi.js` | selected interval, selected integration, cursor | sel, isel, cur | - | - | not undone: transient selection: it moves nothing the student made |
| `view.navigate` | correzione (`storia.campi`) | `pannelli.js` | walk through the scans | si, r0, r1 | arrows | - | not undone: navigation: it chooses what to look at, the data are the same |
| `view.fullscreen` | correzione (`storia.campi`) | `pannelli.js` | full screen of a panel | - | F | - | not undone: transient view of the page; Esc leaves it |

Da portare nello stack: 13 azioni su 29.
