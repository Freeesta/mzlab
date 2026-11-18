# Usability study: mzLab seen by a first-time user

Report only: nothing in the program is changed by this file. Persona: a PhD student in analytical chemistry, not a programmer, who opens the site to look at one LC-MS run.

## Method and coverage
- Site built with `tools/prepara_sito.py`, served locally, driven with Playwright Chromium on a real first visit (empty storage, tour invitation on). Data: `tools/dati_sintetici.py` (`B_FullMass-t15`, HR and MRM files) and the bundled full-scan example set.
- Static checks: `tools/controlla_i18n.py` (green), the two catalogs compared key by key (5539 keys each, no untranslated sentence), `tools/list_tooltip.py` for tooltip lengths.
- **Covered**: first visit in an Italian and an English browser; opening one file and the example set; chromatogram and spectrum; "Extract ions (XIC)" with right and wrong input; one-click integration and its table; download menus; reference lists; search palette;
  settings; reset dialog and "File" reset; truncated, fake, empty and wrong-type files; Tab order on the landing and with a file open; palette → XIC by keyboard; 390 px phone (gate and "Open anyway") with `prefers-reduced-motion`.
- **Not covered (unfinished)**: identification in high resolution (formula panel, spectral libraries), the study report (Ctrl+E) and the content of the Excel/CSV files, Drawing, Tabui and Flow, contrast measurements, reduced motion at desktop width, closing the tab in the middle of a load, integration with the keyboard only, the "i" window, a full English pass beyond landing, XIC dialog, reset and settings.
  A finding marked *(catalog)* or *(attribute)* was read in the source and not seen on screen.

## What works well
- A dropped file opens straight into a TIC and the spectrum of its highest peak: no wizard, no "Load" step, about one second for a 2.7 MB mzML.
- Loading feedback: one row per file with phase and percentage, "N of M ready", a provisional TIC and a "Cancel" button.
- The example sets are one click away from the landing and say their size before the download.
- Integration: one click on an XIC peak draws the bounds, writes the area on the chart and offers a table with Excel and CSV; the bounds can be dragged or moved with the arrow keys.
- Integrating a TIC is answered with the reason and the next step ("Extract an XIC now?").
- Keyboard: every visible control of the landing and of the two default panels takes focus in a sensible order with a visible ring and a name; Ctrl+K → `xic` → value → Enter extracts an ion without the mouse; Shift+F10 opens the peak menu; `?` lists the shortcuts.
- The reset dialog opens with the focus on "Cancel" and closes with Esc.
- File errors for a truncated mzML say what happened and what to do ("copy the whole file again").
- No horizontal page scroll at 390 px on the gate or the landing; no running animation with `prefers-reduced-motion`; no console error in any walkthrough.
- The two catalogs are aligned; the decimal comma is accepted in the XIC field (`364,4` becomes `364.4`).

## Findings
Ranked by how many first-time users meet them. Severity: **blocker** (the goal fails or work is lost), **confusing** (the user stops or reads a wrong result), **polish**.

1. **blocker · Reset dialog** (`mzlab/web/reset.js:124`, `lang/*/altre-press-xic.js:189-192`; header button "New session"). The dialog is the question "Start a new session?" and three buttons, "File", "Full reset", "Cancel": no sentence, no tooltip, no `aria-describedby`.
   Nothing says that "File" removes every loaded file, XIC and integration, or that "Full reset" also removes preferences, saved lists and libraries; there is no undo and the page reloads to the empty landing without a word. *Fix: one line under each button listing what goes and what stays, and a verb on the buttons ("Remove files", "Delete everything").*
2. **confusing · XIC window misses the peak the user clicked** (`mzlab/web/pannelli.js:909`, `docs/agenti/xic-scarto.md`). In low resolution the window is [n−0.2; n+0.3] around the nearest integer. On the synthetic file the spectrum labels the parent 364.4; typing 364.4, or arming "XIC" and clicking that peak, extracts "m/z 363.8–364.3":
   the trace tops at 1.7e4 while the peak is 1.2e7, and the right-click menu still says "Extract the XIC of m/z 364.4". The bundled example (label 364.2) falls inside the window, so the trap depends on the instrument offset. *Fix: when the typed or clicked m/z is outside the window it produces, widen the window to hold it or say so next to the title.*
3. **confusing · "Extract" sometimes does nothing** (`#xic-go`, `openXic` in `pannelli.js`). Pressing "Extract" (or Enter) right after typing a range (`364.0-364.8`) or a formula leaves the dialog open with an empty `#xic-err`; a second press works. A fast user sees a dead button. *Fix: wait for the pending evaluation of the row, or show "Calculating…" and extract when it answers.*
4. **confusing · The guides describe another program** (`README.md:12`, `docs/utenti/en/students.md:8,11,19`, `docs/utenti/it/studenti.md:11`). They say "press Load data" (files open at once, no such button), "Theory and Practice" sections and "on a smartphone only Theory opens" (the header has Data, Drawing, Tabui, Flow;
   the phone shows a gate with "Open anyway"). The README lists only Data and Drawing. *Fix: rewrite the three "loading" paragraphs from the screen and name the four tabs.*
5. **confusing · English browser, Italian page** (`docs/agenti/lingue.md` lines 4 and 9 disagree; `tools/build_site.py:32` calls English "the default language"). A browser set to en-US gets the Italian interface; the switch is inside the gear ("Language"), nothing on the landing points to it. *Fix: follow `navigator.language` on the first visit, or put IT/EN on the landing; make the docs say one thing.*
6. **confusing · Wrong or damaged file** (`diag.cause.unknown`, `lang/*/altre-a11y-fair.js:59`; `err.file.notMzml`). A text file renamed `.mzML` and a 0-byte file both give "… does not look like an mzML file" followed by "Cause not recognised: open "Details" and copy the diagnosis" and raw JSON (`{"error_key":"err.file.notMzml",…}`);
   the 0-byte case does not use the clearer `err.file.empty` text, and the truncated file shows the parser's English "syntax error: tag not closed" inside the Italian sentence. The file list stays "Your files will land here!". *Fix: drop "Cause not recognised" when the engine already named the cause, and keep a row for the refused file with its reason.*
7. **confusing · Tabs "Tabui" and "Flow"** (`mzlab/web/index.html:12`). The names say nothing to a newcomer; their tooltip and accessible name are English in the Italian page ("Automatic search for transformation products", "Build and export a chain of steps"), and Tabui's accessible name is "Loading…" until its module arrives.
   *Fix: a noun the user knows next to the name ("Tabui · transformation products") and a stable `aria-label`.*
8. **confusing · File picker hides supported formats** *(attribute)* (`mzlab/web/index.html:61`, `accept=".mzml,.mzxml,.raw,.wiff,.scan,.gz,.json"`). The README and `err.file.notAccepted` list `.cdf` and the Shimadzu `.qgd`/`.lcd` as readable, but "browse" greys them out; only drag and drop reaches them. *Fix: one list of extensions for the picker, the landing text and the error.*
9. **confusing · XIC dialog field** (`xic.placeholder`, `lang/*/altre-press-xic.js:72`; `xic.adduct.title` line 85). The 125-character placeholder is cut at "range (250.8-" in a 220 px field, so formula, contaminant name and shift are never read. The adduct menu sits beside a field that usually holds an m/z and does not say it applies to formulas only;
   its tooltip reads "Gives the m/z of the ion…". Free text (`abc`, `caffeine`) gets "A row is not valid" with no hint that only names of the reference lists are known. *Fix: short placeholder plus a visible line of examples; grey the adduct when the row is a number.*
10. **confusing · Search palette does not know "export" or "library"** (`cmd.report.kw`, `lang/en/comandi.js:245`). In the English page `export` and `library` return "Nothing found"; `report` finds "Study report · Ctrl+E" (the keywords hold the Italian "esporta" only), and libraries exist only with high-resolution files, which nothing says. *Fix: add the English keywords and a result that explains where libraries live.*
11. **confusing · 390 px after "Open anyway"** (`.pnl.spec` toolbar). The spectrum panel's toolbar runs to x = 522 in a 390 px viewport: "XIC" is cut and "⋯", full screen and "×" cannot be reached; the top row cuts "+ Chromatogram"; file names shrink to "B_F…" and "B…"; the tour invitation covers the lower half of the spectrum.
    *Fix: give the spectrum toolbar the same "⋯" collapse the chromatogram has at this width.*
12. **confusing · Disabled buttons without a reason** (`#np-chrom`, `#np-spec`, `#np-mrm` before a file). Three of the five "+" buttons have an empty `title` when disabled; the other two keep a tooltip that describes the tool, not why it is off. *Fix: one tooltip for the disabled state ("Open a file first").*
13. **polish · Five example files, one shown.** After "Full Scan · 5 files" only `FullScan_t0` is ticked and drawn; the role of the checkboxes is left to the tour. *Fix: tick all, or a one-line note in the list.*
14. **polish · "XIC" in the spectrum arms a mode silently.** The first click only turns the button blue; nothing says "now click a peak". *Fix: a status line while armed.*
15. **polish · Plurals in the chart descriptions** (`a11y.line`, `lang/*/altre-a11y-fair.js:413`). Screen readers hear "1 traces", "1 integrated peaks" (also "1 tracce" in Italian). *Fix: ICU plural, as other keys do.*
16. **polish · Three number styles on one chart**: area "A = 9.992E+07", axis "1.4e+7", hover "7.9e+6"; the table header says "Area (counts·s)" but the chart label has no unit. *Fix: one formatter and the unit on the label.*
17. **polish · An XIC outside the scan range** (`9999` on a 120–480 run) opens an empty panel with y up to 0 and no note. *Fix: say that the window is outside the acquired range.*
18. **polish · TIC integration is refused** (`int.guard`, `lang/*/altre-feat-lst.js:102`). The explanation is good, but a refused result sits oddly with "the user decides"; OK only leads to the XIC dialog. *Fix: a third choice "Integrate the TIC anyway".*
19. **polish · Tooltip length** (`tools/list_tooltip.py`). Of 549 tooltips written in the pages, 85 are longer than 60 characters and 8 longer than the 90 of `.claude/rules/frontend.md` (longest: `ctl.map.norm.need`, `lib.what.group`, `ibk.chip.title`, `lock.on.title`, `ctl.zreset.title`);
    54 of the 56 Flow step descriptions pass 90. The committed `docs/studi/TOOLTIP_ELENCO.*` differ from what the tool writes. *Fix: shorten the eight, regenerate the inventory.*
20. **polish · Italian word in an English text** *(catalog)* (`load.table.remove`, `lang/en/load.js:76`: "Remove (moves to the _cestino folder)"). *Fix: "trash folder".*
21. **polish · Small things on the first screen.** The file box in the bar cuts "B_FullMass-…" with free room beside it; a queued file reads "Queued 40%"; the phone gate says "You are on a smartphone" in the formal plural while the rest of the page uses the informal singular;
    the landing has no line saying what the program does or that the files stay on the computer. *Fix: one pass on these texts.*

## Top 10 to fix first
1. Say in the reset dialog what each button deletes (1).
2. Make the default XIC window contain the m/z the user typed or clicked, or warn (2).
3. "Extract" must never be a silent no-op (3).
4. Bring README and the student guides back to the screen: no "Load data", no Theory/Practice, four tabs (4).
5. Language on first visit and one statement of the rule in the docs (5).
6. Clean the message for a fake or empty file and keep the refused file in the list (6).
7. Same extension list in the picker, the landing and the error text (8).
8. Readable examples in the XIC dialog and a hint for free text (9).
9. Spectrum toolbar at 390 px (11).
10. Name and explain Tabui and Flow, in the page language (7).

Totals: 1 blocker, 11 confusing, 9 polish.
