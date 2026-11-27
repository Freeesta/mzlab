Read docs/coda/BRIEF.md.

# PANEL-DRAG-SCROLL: scroll the page while a panel is dragged

**Goal.** Dragging a panel by its handle (`arrange.js`, `arrDragGrip`) near the top or bottom edge of the window scrolls the page, so a panel can be dropped past the visible part of a long stack; the live preview (sliding panels, dashed drop box) stays correct while the page moves.

**Why.** The live preview of PR "Panel drag: live reorder preview" moves the held panel with the pointer delta of `clientX/clientY` and finds the drop place from the pointer in host coordinates. A stack taller than the window cannot be reached today without dropping first, scrolling and dragging again.

**Read first.** `grep -n 'drag\|panel handle' docs/agenti/INDICE.md`, then `docs/agenti/panels-side-by-side.md`; `mzlab/web/arrange.js` (`arrDragGrip`, `arrSlide`, `arrLiveEnd`), `mzlab/web/frame.js` (`FRAME.next`, `FRAME.behavior`).

**Done means.** An edge zone of about 48 px scrolls at a speed that grows toward the edge, with `FRAME` and no timers; the held panel and the drop place keep following the pointer while the page scrolls and after the pointer stops; reduced motion scrolls in jumps of one step per frame without smooth behaviour; Esc and drop stop it. A new step in `tests_e2e/e2e_affianca_vivo.py` pins it at 390 px with a stack taller than the window; the keyboard path is unchanged (the page follows the focused panel). Keep the guided tour valid.

**Out of scope.** The file-list reorder and the side strips (`barra.js`); the layout maths of `layout.js`.

**Follow-ups.** Closing step of the brief.
