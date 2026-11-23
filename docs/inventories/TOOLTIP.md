# Tooltip inventory

Every tooltip site of the program (`mzlab/web`, Tabui `TP_Mine`, Flow `flow/lazy`), classified. The per-zone tables (`TOOLTIP-panels.md`, `TOOLTIP-spectra.md`, `TOOLTIP-lists-1.md`, `TOOLTIP-lists-2.md`, `TOOLTIP-drawing.md`, `TOOLTIP-tabui.md`) are generated: `python3 tools/inventory_tooltip.py --md docs/inventories` (icons with no tooltip: `--gaps`). Counts are not kept here: `python3 tools/inventory_tooltip.py` prints one row per site and the number of sites on stderr; `python3 tools/list_tooltip.py` prints the totals of the full text list (`--md` and `--csv` print the list itself to stdout, nothing is written). This page: mechanism, rule, chart hover, decisions.

## How they work
- **Control tooltips**: the `title` attribute (or `.title =`, `data-i18n-title`). The program replaces it with its discreet tooltip (`eventi.js`, block «discreet tooltip»): on the first pass `title` becomes `data-tip`, the `#qtip` box appears after 1.25 s (1 s in `.fl` rows) of a still pointer or keyboard focus, disappears when pointer or focus leave, and Esc dismisses only it (WCAG 1.4.13; guard `e2e_tip_tastiera`); it does not appear inside an open `dialog`. `data-tiph` = tooltip with HTML. On touch: a long press of 0.6 s.
- **Chart hover** (canvas): `p._a.hov` returns the HTML of the `p.tip` box (`grafici.js` `showHover`); peak and annotation labels have their own `tip` (`_a.lbls`, `eventi.js`).
- **SVG charts**: `<title>` elements (`pca.js`, `kmd-vista.js`, `pfas.js`, `profilo.js`, `qaqc.js`).
- **Texts**: keys in `mzlab/web/lang/en/` (the only catalog; the interface is English only). Sites without a key are built at run time, or are Tabui/Flow texts written directly in English.

## Classes
- **A useful**: adds what is not visible (unit, shortcut, effect, explanation of an icon). Almost every row of the tables.
- **B redundant**: repeats the label or the value already written next to it.
- **C to redo**: incomplete or inconsistent (a format unlike the others, text that names internal files).
- **D to remove**: noise (for example just "Right-click: actions").
- **-**: the line contains `.title` but it is a panel or page property, not a tooltip.

## Rule (the maintainer)
1. At most one short sentence: what the control does or the datum that is not visible; never a copy of the label, no tutorial, no explanation of the obvious.
2. Dynamic values (time range, id) are not pasted into the explanation: they have their own column or disappear. Shortcuts stay, in parentheses.
3. Icon-only buttons ALWAYS have a tooltip with the name and, if there is one, the shortcut.
4. Every chart shows time, m/z and intensity in the same format, produced by a single function.
5. Hints about gestures («Right-click: actions») belong in the help, not in the tooltip.

Text rule (guard `tests/test_tooltip_corti.py`, every `.tip`, `.title` or `Title` key and every literal `title="..."`): one short sentence, at most 90 characters, saying only what the control does (shortcut in parentheses); never promises about privacy or method, advertising, a tutorial, the repeated label, the «candidate, not identification» warning or a pasted dynamic value. A warning only when the control destroys (deletes, overwrites); facts go in the help or in visible notes. Exceptions in `tests/tooltip_ammessi.txt`, each with its reason.

Other guards:
- `tests/test_tooltip_inventario.py`: no tooltip equal to the text of its own element, none made only of «Right-click: actions».
- `tests/test_elenco_tooltip.py`: the full text list (`tools/list_tooltip.py`) covers every source and its CSV matches its Markdown.
- `tests_e2e/e2e_tooltip_dinamici.py` (built site, synthetic files of every type: Full Scan, MS2, MRM, IDA, high-resolution DDA, `.wiff` Q1/PI/EPI and SIM): collects every element with `title`, `data-tip` or `data-tiph` in every experiment tab, then hovers the group headings, every HTML tooltip and the longest texts. Fails if a text exceeds 60 characters or has more than one sentence, or if the drawn box is wider than 28ch or taller than 3 lines (`#qtip` has `max-width:28ch`).
- Settings panel: every visible control has its own tooltip, at most 60 characters, different from the label, with one LR and one HR file open; guard `tests_e2e/e2e_tip_impostazioni.py` (also mouse, long press on touch without toggling the switch, focus ring with Tab).
- The headings of the side list groups (`files.grp.tip.*`) say only the scan type; the Q1 -> q2 -> Q3 scheme is in the type help, not in the tooltip.

## Chart hover (to keep consistent)
| where | what it shows | class | note |
|---|---|---|---|
| `grafici.js` chromatogram, XIC, MRM | **RT min**, then per trace: name and intensity (+ "raw"); up to 8 lines | A/C | no unit (cps) and no m/z of the trace |
| `picchi.js` spectrum (`tip.peak`) | **m/z**, "intensity" and value, absolute cps if shown in %, Δm/z from the reference, file | C | format differs from the chromatogram |
| `grafici.js` 2D map (`map.hov`) | **RT · m/z**, "intensity" or "difference", m/z bin | A | the most complete format |
| `grafici.js` PDA map | RT min, nm, mAU written in the code, not from `lang/` | C | units outside the catalogs |
| `uv-spettro.js` UV spectrum | nm and mAU written in the code | C | as above |
| `area-etichette.js` area label | no hover text (a click copies the area) | - | the hover box with area, intensity and baseline lines was removed |
| `picchi.js` m/z label (`tip.peakLabel`) | m/z and «Right-click: actions» | D | the m/z is already on the label |
| `etichette-picchi.js` RT label (`etpk.tip.rt`) | «Peak at RT 14.34 min» | B | repeats the label |
| `spettro.js` ruler Δ labels | adduct, Δ, «Right-click: ...» | C | a single gesture hint |
| `grafici.js` annotation (`draw.ann.tip`) | the note text + double-click and right-click gestures | C | the text repeats; the gestures remain |
| `dda.js` DDA markers | title, MS2 count, precursor | A | |
| `mappa.js` side-spectrum labels | m/z, S/N | A | |
| SVG: `pca.js`, `kmd-vista.js`, `pfas.js`, `profilo.js`, `qaqc.js` | sample, m/z, RT, value | A | own formats, each with its unit |

## Open proposals
- A single chart-hover function (time, m/z, intensity with units, e.g. «RT 14.34 min · m/z 301.1 · intensity 2.3e5 cps») and units from `lang/` for the PDA map and the UV spectrum.
- Peak labels, spectrum hover and «Right-click: actions» (`tip.peakLabel`, `etpk.tip.rt`, `tip.peak`): keep only area and intensity on the peak label.
- Tooltips that repeat gestures on every element («Click: show or hide» in legends, «Click: show only this file»): move to the help.
- The tooltips above 200 characters: shorten them or move them to the help; `draw.ann.tip` and `spec.dm.tip.*` (a tooltip that is only a gesture).
- Icon-only buttons with an `aria-label` but no tooltip: `python3 tools/inventory_tooltip.py --gaps` lists them; each needs a tooltip with name and shortcut.
- Tabui and Flow follow the same rule (their texts are English only, like the rest).
