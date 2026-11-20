# Draw: unsaved changes (`disegno-pulito.js`)
The core is in CLAUDE.md; the rest of Draw is in `drawing.md` (its «New» bullet: «New» asks through `confirmDiscard`).

- Unsaved state (`disegno-pulito.js`, tests `e2e_disegno_pulito`, `tests/test_disegno_pulito.py`): the canvas is dirty when it is not empty and `canon(getKet())` (sorted keys) differs from the last
  one exported (.ket/.mol/.sdf/PNG/SVG/JPEG through `doExport`), loaded (`TPDraw.load`) or restored from the notebook; undo back to it is clean.
  `Pulito.check()` runs after each change (`changed()`).
  ONE question, `confirmDiscard(why)` (`why` = `new` | `load` | `close`): clean or empty = no dialog; dirty = `choiceBox` of `memoria-avviso.js` «Your drawing is not saved» (alertdialog, Cancel has
  the focus, Esc and a click outside cancel, Tab stays inside) with «Save …» (format list .ket default, .mol, .sdf, PNG, SVG: the panel's own `doExport`, the dialog closes only after it returned true,
  else the `#ex-warn` text shows in the box), «… without saving» and «Cancel»; a saved non-empty canvas asked by «New» keeps the plain «Clear the drawing?».
  Guarded:
  «New» (`disegno-nuovo.js`) and every `TPDraw.load` (tray, fragmentation graph, Tabui, desktop file open; it returns false when the user kept the drawing).
  Not guarded:
  Ketcher's own toolbar Open/Save menus, and drops (they add, never replace).
  The Draw view is a view of the page (no close action: switching views keeps the canvas and it is autosaved in the notebook), so there is nothing to ask there.
- Leaving the page:
  `beforeunload` is registered only while the canvas is dirty and removed when clean; the browser shows its own generic prompt (a page can neither word it nor start a download from it).
  Desktop app: the same dialog on the window's close (`onCloseRequested` in `mount`, permission `core:window:allow-destroy`; not tried in a real window in the cloud).

