# DDA HR: how a user reaches an MS2 today, what breaks, and a simpler design (stage 1)

Audit of the path «I have an Orbitrap DDA file; show me the MS2 of this ion/scan» on the built site (Playwright, Chromium), synthetic data only.
Line numbers refer to `origin/main` at 4422f76 unless a fix is named.

## 1. Data used

`tools/dati_sintetici.hr_dda` (Exploris: HCD NCE 30, ±0.75; Fusion: CID 35 ion trap, ±1.5; profile variant), plus one scratch variant
`HR_DDA-var-t30` (same scans, Orbitrap-like timing: the next MS1 comes after the MS2 of its cycle, MS1 gaps 0.008-0.023 min). 650 MS1,
142 MS2, at most 3 MS2 per cycle, 74 MS1 scans with at least one child, 0.3 min dynamic exclusion. Scripts (scratch, not in the repo):
`audit.py`, `audit2.py`, `audit3.py`, `audit4.py`; the regression test that stays is `tests_e2e/e2e_dda_ms2_vie.py`.

The owner's screen is reproduced at the first view: the trio opens on the TIC apex (`apexSpectrum`, `explore.js:494`). At the apex the main ion
is in dynamic exclusion, so that MS1 scan has **no child MS2**: `markN = 3` but `flags = 0`. Only three open grey triangles ▽ («fragmented in
another scan», `--muted` stroke) are drawn, one of them on top of the m/z label of the base peak. The MS2 panel says «Clicca una bandierina ▼
nella Full Scan…» (`dda.click`): there is no ▼ anywhere. In a real top-N file the apex scan is very often in exclusion, so this is the usual first view.

## 2. Every way to an MS2 today

| # | Path | Averaged MS1 | Zoomed MS1 | Peak without MS2 | Several MS2 / precursor | Second DDA file | Keyboard / touch |
|---|------|------|------|------|------|------|------|
| 1 | ▼ flag over a precursor (own scan), click (`dda.js:273`) | **none drawn**: `decorateFull` returns when `d0.scans !== 1` (`dda.js:214`) | flags outside the zoom vanish, no hint (`dda.js:251`) | no flag, panel hint unchanged | one flag per scan; numbers 1..n | **none drawn** until something loads the file (bug B3) | canvas only, no keyboard; hit box 18×24 px (`dda.js:260`) |
| 2 | ▽ open triangle (same ion fragmented within ±1 min), click = closest, right click = average (`dda.js:227-249`) | none (same guard) | as 1 | only if fragmented nearby | average of ≤ 5 | as 1 | canvas only; grey stroke, low contrast; overlaps the m/z label |
| 3 | Arrows on the chromatogram (`stepScan`, `pannelli.js:473`) → Full Scan scan by scan | – | – | – | – | – | **bug B1**: every step shows a Full Scan without scan id (`scroll.js:65` drops `sid`), so **no flag, no ▽, no «Apri la scansione MS²» while walking** (25/25 steps) |
| 4 | Right click on an HR peak → «Apri la scansione MS² (precursore …)» (`dda.js:444`, `ms2For`) | entry missing (needs `d0.sid`) | ok | entry missing | the closest precursor only | missing (B3) | Menu key / Shift+F10 work |
| 5 | Right click → «Segui questo ione nelle MS2» (`follow`, `dda.js:305`) | ok | ok | message `dda.none` (+ `dda.mixed` when co-isolated): clear | most intense, then ← → | ok after fix B2 | menu only |
| 6 | Triangles on the chromatogram (`grafici.js:407`, click `eventi.js:170`) | – | – | – | one per MS2, merged when < 3 px apart | – | **off by default**; the switch «mostra le MS2» (`ctl.ms2tri`) sits inside the «Intervallo» box of the chromatogram (`pannelli.js:716`); 6×7 px triangles, hit ±6 px; no keyboard. The panel hint names them anyway |
| 7 | ← → with the MS2 panel active (`DDA.step`, `dda.js:141`) | – | – | with a followed ion: its scans, or those whose window holds it | ok | ok | from an empty panel it jumps to the **first MS2 of the file** (RT 0.5), moving the chromatogram cursor away from where the user was |
| 8 | «Albero MSn» button of the HR bar → «Experiments» grid (`msn-elenco.js`) | – | – | – | grouped, sortable, keyboard grid | ok | good keyboard path, but the trio never points to it |
| 9 | «Elenco dei precursori…» (`dda.js:372`) | – | – | – | – | – | **unreachable in HR**: `listMode` needs `E.tab === "ms2"`, the HR DDA file is one row in the Full Scan tab |
| 10 | Banco / statistics row, inclusion list (`openMs2`, `openEntry`) | – | – | – | – | – | ok (not the beginner path) |

No page error or console error came up in any run (only the known Chromium note about the sandboxed iframe).

### Bugs (red test first, fixed on local branch `claude/dda-ms2-vie`, not pushed)

- **B1** walk with the arrows: the cached scan returned by `scData` (`scroll.js:65`) has no `sid`, although `api/scanbin` sends it. Effect: no DDA marker and no «open MS2» entry on any Full Scan reached with ← →. Fix: `sid: s.sid ?? null`.
- **B2 (scientific)** two DDA files open, second one shown: the Full Scan is the second file but the MS2 panel reads `api/scan?k=<first file>`; the title takes the precursor of the second file. Repro: open Exploris + Fusion, click Fusion in the list, choose an MS2 → caption «HCD · NCE 30» (Exploris) instead of «CID · NCE 35». Cause: `ctl()` sets the panel `k` to the first ticked Full Scan item (`pannelli.js:748`); `selectMs2` never re-reads it. Fix: `own(s2, f)` in `selectMs2` and `average` puts the panel on the Full Scan item of the same physical file.
- **B3** the MS2 list (`/api/dda`) of a DDA file shown after the first is never loaded, so its Full Scan has no markers until a reload or a follow. Fix: `decorateFull` preloads it once and redraws.

Test: `tests_e2e/e2e_dda_ms2_vie.py` (red on main: both steps; green with the fix). Neighbour e2e green: e2e_hr_dda, e2e_hr_dda2, e2e_hr_nearest, e2e_hr_vista, e2e_spettro_hr, e2e_hr_msnelenco, e2e_scroll, e2e_cursore, e2e8, e2e_asse_x, e2e_longtask, e2e_matrice_bug, e2e_misti; `pre_pr.py` green.

`DDA` in `dda.js` is now exactly at its line tolerance (443): PR 2-5 first move the drawing of the markers (`decorateFull`, `decorateMs2`) into a module of its own.

### Confusing texts

- `dda.click` promises «▼» and «triangolino sul cromatogramma» in views where neither is drawn (averaged MS1, apex in exclusion, triangles off).
- `dda.near.hint` «Tasto destro: media» names no touch equivalent (not checked on a real tablet).
- The MS2 panel also writes «Grafico senza dati da mostrare.» to the screen reader text while the canvas shows `dda.click`: two different messages.
- `dda.first` / `dda.last` give no position («3 of 7»).

## 3. Proposed design: one obvious action, the reason when empty

Principle: the MS1 view itself says how many MS2 it holds and offers them; the peak is the target, the flag only marks it.
Nothing here names a compound: an MS2 is shown as acquired (precursor m/z, scan, CE); identification stays with the library/formula tools, as scored candidates.

1. **«N MS²» button in the Full Scan header** (24×24 px minimum, `aria-haspopup="listbox"`). One scan: the MS2 started by it; averaged scan: the MS2 of all its scans; zero: the button reads «0 MS²» and is disabled with the reason in `title`. Click, Enter or Space opens a list: precursor m/z, RT, CE, intensity of the precursor; ↑ ↓ Enter, Esc closes. The averaged case groups by precursor (n scans each).
2. **Click on a peak** (no need to hit the flag): if the shown scan(s) fragmented it, its MS2 appears; otherwise the closest MS2 of that ion within ±1 min (today's ▽ logic) with a one-line note; otherwise the panel says why (point 4). The ▼/▽ markers stay as signs, with a 24×24 px hit box, the ▽ drawn with `--text` at reduced opacity (contrast ≥ 3:1) and moved above the label.
3. **MS2 panel header**: ‹ › «previous / next MS² of this precursor (2/5)»; Shift+‹ › all MS² in time. From an empty panel ← → start at the chromatogram cursor, not at the start of the file.
4. **Empty MS2 panel = the reason** (one of):
   - averaged MS1 with MS2 inside → «Media di {n} scansioni: {m} MS² dentro. Scegline una con «{m} MS²» qui sopra.» / «Average of {n} scans: {m} MS² inside. Pick one with «{m} MS²» above.»
   - single scan with no child, ions fragmented nearby → «Nessuna MS² partita da questa scansione. ▽ = ione frammentato in una scansione vicina: clic sul picco.» / «No MS² started from this scan. ▽ = ion fragmented in a nearby scan: click the peak.»
   - single scan, no MS2 within ±1 min → «Nessuna MS² entro ±1 min. ← → sul cromatogramma, o «Albero MSn» per tutte.» / «No MS² within ±1 min. ← → on the chromatogram, or «MSn tree» for all.»
   - precursors outside the m/z zoom → «{n} MS² fuori dallo zoom m/z. Backspace: tutto.» / «{n} MS² outside the m/z zoom. Backspace: all.»
   - file without MS2 → «Questo file non ha scansioni MS².» / «This file has no MS² scans.»
   - default (a scan with children) → «Clic su un picco con ▼, o «{n} MS²» qui sopra.» / «Click a peak with ▼, or «{n} MS²» above.»
5. **Header chip text**: «3 MS²»; tooltip «MS² da questa scansione» / «MS² from this scan» (average: «MS² da queste {n} scansioni» / «MS² from these {n} scans»).
6. **Chromatogram triangles**: a visible toggle «▲ MS²» in the chromatogram bar of a DDA trio (24 px, `aria-pressed`), or remove them from the hint; the merged triangles keep a count in their tooltip.
7. **Keyboard**: Tab reaches «N MS²»; in the spectrum canvas the existing T/D keys gain «M» = open the MS² list (announced in the canvas description). Touch: tap on a peak = point 2; the average of nearby MS² also in the «N MS²» list, so no right click is needed. Safari 16.4: no new API (popover attribute avoided; a `<div role=listbox>` like the other menus).

## 4. PR plan (small steps)

| PR | Content | Risk |
|----|---------|------|
| 1 | Bugs B1-B3 + `e2e_dda_ms2_vie` (ready locally) | low: three lines of interface code; neighbour DDA, scroll and cursor e2e re-run |
| 2 | Empty-state reasons in `DDA.waiting` (point 4) and the corrected hint texts, IT + EN keys; screen-reader text = canvas text | low: text and a pure function over data already loaded; tour check (`e2e_tour`) |
| 3 | Rust route `/api/dda_window?k&r0&r1` (MS2 of the MS1 scans in a window, grouped by precursor within the file tolerance) in `hrroutes.rs` + reference + `hr_parity`; flags and «N MS²» on averaged MS1; the header button and its list (lazy module via `carica()`) | medium: new route (site and desktop share `mzlab-core::routes`, so parity is by construction plus the byte test); `.wasm` growth recorded in `wasm-tetti.md` |
| 4 | MS2 navigation: ‹ › of the same precursor with n/N, ← → from the cursor | low-medium: `DDA.step`; existing e2e_hr_dda arrows step |
| 5 | Click on a peak → its MS2 (own scan, else nearest ±1 min); 24 px hit boxes; ▽ contrast and placement; visible chromatogram triangle toggle | medium: click routing in `eventi.js`/`dda.js` shared with ruler, annotation and marker clicks; tablet and axe checks |

Desktop app: it loads the same `mzlab/web` interface and the same `mzlab-core` routes (`routes.rs:108` `/api/dda`), so every finding above
applies to it as well; the Tauri window itself was not driven in this cloud session (no display).
