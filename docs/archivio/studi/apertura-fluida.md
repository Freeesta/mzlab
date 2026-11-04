# Opening without jumps (Load data → settled work area)

Measured with `tests_e2e/e2e_apertura_fluida.py` (headless Chromium, 1500×2200, three synthetic Full Scan files, `layout-shift` observer + boxes of the main regions at every frame).
Cumulative layout shift (CLS) of the opening: **0.0169 → 0.0007** (normal and lab builds). The remaining 0.0007 is the header (`#stbar`) gaining the panel toggle in the very frame in which the view switches from the table to the work area.

## Sequence and what moved (before → after)

| # | Event (order of the code) | Before | After |
|---|---|---|---|
| 1 | `#opbtn` click → `openingStart` → `applyView`: `#v-data` shown, skeleton `#opening` | Skeleton full width (1460 px), no side panel | Same frame: side panel `#dfiles` already open with its final width; skeleton starts where the panels start |
| 2 | Session arrives (`bootSession`, `S.sess`): `barra.js` `inData()` true → `applyLayout` adds `sb-open` | Side panel appeared (slide-in), `#tools` and skeleton jumped 286 px to the right, `#v-data` grew 234 → 2152 px (CLS 0.0167) | `sb-open` is set in event 1 (`inDataOrOpening`); nothing changes here |
| 3 | `namesWidth` after the first file is listed | Panel width 300 → 280 px | Width computed in event 1 from the ticked file names |
| 4 | `toolbar()` / `renderNav()`: `#g-file`, `#g-add`, `#g-tools`, `#np-method` shown, `#fsel` filled | Toolbar items popped in and `#fsel` grew while the other files arrived (122 → 130 px, buttons shifted) | Toolbar is `visibility:hidden` while opening, reveal fades it in; `#fsel` width counts the files still pending |
| 5 | `#dpanels` shown, panels built, canvases drawn | `#dpanels` was `display:none` while the panels were built, so they were measured at width 0 and refitted by the ResizeObserver after the first paint | `#dpanels` keeps its place (`visibility:hidden`, min-height 520) from event 1: panels and canvases are created at the final size and never resized |
| 6 | Skeleton → real content | Hard swap (164 px skeleton, 662 px panels) | Skeleton overlays the top of `#dpanels` (absolute, same top); the content fades in: opacity only, `--t-med` (180 ms, `--ease-out`), none under reduced motion |

Nothing else in the sequence changes geometry: `restoreUi`, `uiSave`, the i18n text swaps (`openingRender`) and the fonts do not resize any region. Late `renderNav` / `toolbar()` calls change the toolbar only when a later file brings a new experiment type (a button for that type appears), which is not part of the first opening.

## Rules for new code in the opening

- Nothing visible may depend on `S.sess` for its position or size: reserve the final geometry in event 1.
- Class and view changes of the opening go through `applyView` in one synchronous pass; the animation touches opacity only.
