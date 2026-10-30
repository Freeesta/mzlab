# Landing on the work area, option D (Federico's design)

Previous study: `docs/studi/LANDING_AREA_LAVORO.md` (option C: start card in the empty work area, staging table, «Load data» kept for the first load). Federico replaced C with his own design, called D here. This study maps D onto the code, answers the hard questions with evidence, adds how files are **added and removed** in D (Coordinator's request), and gives build prompts in `docs/studi/prompt_landing_d/`.

**Short answer: D is buildable with what the page already has, and it is simpler than C.** Almost every piece exists and is tested: the side-panel drop that opens files at once (`directDone`), the progressive opening (`E.pending`), the soft removal with undo (`fileRemoval`, `STORIA.tolto`), the LR/HR rule for additions (`#fl-note`), the empty-area hint (`syncWorkEmpty`). What D adds is one rule for the work area and three pieces of new code: progress rows in the side list *during* the upload, a write-through from the sample table to the open files, and the «nothing shown yet» state of a new session. What D removes is a whole mode (`S.adding`, `#loaded`, `#cancadd`, «Load data»). Measured: §1 (Playwright, built site with the Rust engine). Not verified: §9.

## 0. Option D in one table

| | Option C (old study) | Option D (this study) |
|---|---|---|
| First paint | header + start card in the work area, side panel closed | header + **disabled** toolbar + side panel **open** (empty, own drop zone `#fl-dz`) + drop zone in the centre of the work area |
| After a drop | staging table in the card, «Load data» confirms | files **open at once**, one by one; each appears in the side panel as it loads; no «Load data» |
| Before the first click on a file | — | the work area shows the **sample table** (type, time, concentration), filled live, editable |
| First click on a file in the side panel | — | the usual panels (TIC, MS1…); the sample table stays reachable (Statistica › Sample metadata, and the work area whenever no file is shown) |
| Adding later | «+» opens the picker, files open at once | same, plus drop anywhere on the work area |
| Removing | side list × (exists) | × in the side list and in the sample table, «Remove all», undo; the last file removed brings back the empty landing |

### The one rule for the work area

`#v-workspace` shows exactly one of three things, decided in `applyView` from state alone (no flag):

1. **panels** when at least one file is shown (`E.files.some(f => f.vis && !f.gone)`) or a panel is open;
2. **sample table** when files are loaded (open or being opened) but none is shown;
3. **drop zone** (the landing) when there is no file at all (none open, none uploading, none set aside).

This one rule covers the first visit (3), files arriving (2), the first click (1), hiding every file with the boxes of the side list (2, today the muted text `work.empty`), removing the last file (3). It replaces `#work-empty` (§1, row «all hidden»: today that state is an almost empty page with one muted line), `#start`, `S.adding` and the `S.opening` staging.

## 1. Measured with Playwright

Headless Chromium, built site with the Rust engine (`tools/prepara_sito.py`), viewport 1440×900 then 820×900, synthetic files only (`tools/dati_sintetici.py`). One run on one cloud machine (4 CPU, software GL); the script is in §1.2 so the build chats can run it again. Screenshots were not taken.

| State (today's code) | Measured |
|---|---|
| First visit, no session | `#v-data` hidden; the toolbar buttons exist and none is disabled except undo/redo (they are hidden with `#v-data`, not disabled) |
| First load of 2 LR files with «Load data» | only the first file shown (`vis` true for t0, false for t15): today's first-load rule |
| Drop of 3 LR files on the side panel (session open) | wall time 1.9 s. Side list: **no row for 544 ms** (only `#fl-note` «Adding files…» while 3 `CONVROWS` rows run in the hidden table), then the 3 rows **all at once** at 584 ms with «3 files added». The added files arrive **hidden** (`vis` false) |
| Proposal race (table `ST.files` vs open files `E.files`) | 5/5 files: same role and time on both sides. Not reproduced, because today the side drop opens after the whole batch (`UPBUSY` 0) and the times come with the file list; it becomes possible when files open one by one (§3 d) |
| HR file dropped on an LR session | not opened, unticked in the table; note «0 files added. 1 of a different resolution … stay in “Load other files”» (the note points to the loading page that D removes) |
| Every file hidden (`setVisOf(all, false)`) | 0 panels; `#work-empty` «Tick a file: the ions are waiting!» alone in an empty work area |
| Click on a file row when none is shown | that file shown, 2 panels built (TIC + spectrum): the D «first click» already works |
| «Remove all» (`removeAllFiles`, no button today) | 5 files `gone`, `S.sess` kept, side list empty, **2 empty panels stay on screen** (no file to draw, `#work-empty` silent because panels exist) |
| Repaint of the load table `renderFiles` (synthetic rows) | 50 rows 9 ms, 300 rows 34 ms, 1000 rows 150 ms |
| Repaint of Sample metadata `CAMPIONI.paint` | 50 rows 5 ms, 300 rows 22 ms, 1000 rows 96 ms |
| Side panel width | 300 px with no file (LR default; 340 in HR), 280 px with two short names (`max(280, min(300, names))`, `barra.js:19,52,64`) |
| 820 px with a session | side panel closed (`sb-open` false), work area 808 px wide, no horizontal scroll |
| Page errors during the run | none |

### 1.1 What the measurements say for D

- **Progress rows are missing in the side list** (row «Drop … on the side panel»): D's «files appear as they load» is new work (§3 e), not a move.
- **The first click works today** (rows «Every file hidden», «Click…»): D's state 2 → state 1 needs no new opening code, only the first-load visibility rule (§3 c) and the sample table in place of `#work-empty`.
- **Removing everything leaves empty panels** (row «Remove all»): D's state 3 needs `fileRemoval.take` to close the panels that have no file left (and the undo to reopen them, `riapriPannello` exists for the `files.view` undo).
- **Large folders**: 300 rows repaint in 34 ms, below a frame budget of 50 ms; 1000 rows take 150 ms: chunked rendering is worth it only above ~500 rows (§4 h).
- **Width**: the side panel moves by 20 px (LR) to 60 px (HR) when the first files arrive: D freezes it until the first click (§4 d).
- **Texts that name the loading page** must change (`files.drop.mixed` says «stay in “Load other files”»): D3/D5 grep `lang/` for «Carica altri file» / «Load other files».

### 1.2 How to run the measurement again

The probe (drop through a real `DataTransfer` on `#dfiles`, a rAF timeline of the side-list rows, the comparison of `ST.files` and `E.files`, render cost with 50/300/1000 synthetic rows) is reproduced in D2 (`tests_e2e/e2e_landing.py`, prompt `prompt_landing_d/D2.md`), which prints the same JSON keys.

## 2. What is removed, reused, moved

### 2.1 Removed

| Piece | Today | In D |
|---|---|---|
| `#start` page (section after `#v-data` in `index.html`) | the loading screen; shown when `!S.sess \|\| S.adding` (`applyView`, `shell.js:38`) | gone as a page; its contents move (§2.3). The id `#start` stays on the landing box so the 18 e2e that wait for `#start:not([hidden])` (8 calls) keep a target (§6) |
| «Load data» `#opbtn` click handler (`shell.js`, the long `$("#opbtn").onclick`) | waits for the uploads (`idle`), negates dark times, `openingStart`, `openStaged(true)`, handles `err.hr.mix` with «Replace» / «Cancel» | no button. The body becomes the per-file opening of §3 (c); the `err.hr.mix` «Replace» branch survives as the switch of world (§4 b) |
| `checkGo()` | enables «Load data»; blocks it when LR and HR are both ticked | the LR/HR rule acts at opening time (§4 b); nothing to enable |
| `S.opening` staging (`STATO.apertura`, `#opening` skeleton) for the first load | the work area drawn while the ticked files are read | not needed for drops (rows arrive one by one in the side list). `#opening` stays for the **return visit** (restore skeleton, §4 c) |
| `S.adding`, `STATO.aggiunta`, `_addY`/`_wasAdd` in `applyView`, `leaveAdding`, `#cancadd` | «Add files» swaps the work area for `#start` | gone: adding never leaves the work area (§5) |
| `#loaded` + `renderLoaded` («N files already loaded», «Clear loaded data» `#clearall`, «Back to data» `#backdata`) | the list of open files on the loading page | gone: the side list *is* that list; «Clear loaded data» → «Remove all» of the side list (undoable) and the «Files and session only» choice of «New session» (exists, `QQRESET.run("files")`) |
| `#work-empty` text (`syncWorkEmpty`) | muted «no file shown» line | replaced by state 2 of the rule (sample table) |
| `#drop-skeleton` («Restoring session…» inside `#start`) | return visit | the restore uses `#opening` inside the work area (old L4, still valid) |

### 2.2 Reused as they are

| Piece | Where | Role in D |
|---|---|---|
| `upload()` / `upload0()` (`shell.js`) | one pipeline for picker, drop, folder, examples, desktop «Open with» | unchanged; every entry point already ends here (`predefinito.js:20` for «Open with») |
| `directDone()` | side-panel drop: ticks the open files, keeps the other world aside, `openStaged(false)`, `#fl-note` | generalised to *every* drop, first one included (§3 c) |
| `exploreList` / `openRest` / `E.pending` | the first file at once, the others appended (`api/explore` `append:true`), failures stay red | unchanged; the side list already renders pending rows |
| `fileRemoval` / `removeFile` / `removeAllFiles` (`lista-file.js:44-79`) | take (soft, `f.gone`), back (undo), free (OPFS copy and indexes after 6 s, `storia.js:105`) | the only way to remove, from the side list **and** the sample table. `removeAllFiles` exists but no button calls it today (grep): D gives it one |
| `CAMPIONI.apply` / `campioniEdit` (`campioni.js`, `campioni-tabella.js`) | the only writer of role, time, conc, rep of open files, undoable (`samples.meta`) | the write-through of the sample table (§3 d) |
| `RUOLI.propose` / `setRole` / cells (`ruoli-carica.js`, `ruoli-celle.js`) | proposals from the names (engine, `campioni/2`), one blank type, dark negative | unchanged; the sample table keeps these cells |
| `hrMixPreselect`, `hrMixPick`, `#hrmix` | LR/HR choice in the table | moves with the table (§4 b) |
| `CONVROWS` (`.raw`/`.wiff` conversion rows, progress, «Download mzML») | rows in `#flist` | mirrored as progress rows in the side list (§3 e) |
| `CartPunt` (`#cartpick`, `#cartbody`, «Load all» `#cartgo`) | pointed folder, inside card «1.» | in the landing box; with files open, from the «+» menu (§5) |
| Sample metadata (Statistica, `STAT.add({id:"meta"})`) | editor of the open files | stays the post-opening editor; same data as the sample table (§3 d) |

### 2.3 Moved

| Piece | From | To |
|---|---|---|
| `#drop` (+ `#pick`, `#pickdir`, `.fmts` ⓘ), `#cartpick`, `#cartbody`, `#demobtn`, `#convhelp` | `#start` card «1.» | the landing box `#start` inside `#v-workspace`, centred (state 3) |
| `#dropdam` / `#mlist` (method `.dam`) | `#start` second card | one secondary line under the drop zone; with files open the toolbar «Method» `#np-method` exists |
| `#flist` (the table) + `#hrmix`, `#hrnote`, `#modenote`, `#unopen`, `#gomsg` | `#start` | the sample-table box inside `#v-workspace` (state 2), same ids |
| `#up` (upload errors), `#err` | `#start` | above the sample table / landing (one `role=status` region, §4 f) |

## 3. The sample table and the opening

### 3 (a) Is it the same component as Statistica › Sample metadata?

**No, not today.** Two tables on two models:

| | Load table `renderFiles` → `#flist` (`shell.js:109`) | Sample metadata `CAMPIONI.paint` (`campioni-tabella.js`) |
|---|---|---|
| Model | `ST.files` (every uploaded file, open or not): `use`, `type`, `role`, `time`, `conc`, `gC` (yellow guess), `prop`, `ed` | `E.files` filtered by `CAMPIONI.files()` (open, level 1, not gone) |
| Columns | tick, File (+HR tag, conversion label, polarity), Experiment (menu), Type, Time (unit menu), Conc. (unit menu, yellow guess for standards), Size, «Download mzML», × | #, File, Type, Time (unit), Conc + unit, Replicate; export/copy/import TSV; gradient legend |
| Writer | the table itself, then `RUOLI.commit` → `CAMPIONI.apply(…, quiet)` once, at opening | `campioniEdit` → `CAMPIONI.apply`, one undo step each |
| Extra rows | conversion rows `CONVROWS` (progress, errors) | none |

A third editor exists in Flow (`F.metaGrid`, `docs/agenti/flow-meta-griglia.md`), fed from `E.files`.

**Recommendation for D:** the sample table in the work area is the **load table** (`renderFiles`, `#flist`, same cells, proposals, yellow guess, one blank type, validation `aria-invalid`), moved into the work area, with one change: an edit on a row whose file is open also calls `campioniEdit` (undoable, the same writer Sample metadata uses). Sample metadata stays the editor after the first click. Merging the two tables into one component is possible later (both read `RUOLI.OPTS`, `tunitGet`, `parseDecimal`) but it is not needed for D and it would touch the 84 e2e that read `#flist`. Why not the CAMPIONI table in the landing: it has no conversion rows, no tick for the other world, no yellow guess, no size, and it shows only files already open, so it would be empty during the first seconds of every drop.

### 3 (b) The tick column means «in the session»

Today the tick (`use`) means «open this file at «Load data»». In D the files open at once, so the tick becomes **«in the session»**: ticked = open (in the side list), unticked = uploaded but set aside (other resolution world, or the student unticked it). Unticking an open file = `removeFile` (undo as usual; the row stays in the table, unticked). Ticking an aside file = opening it (`api/explore` `append`). This keeps `#flist input[data-k=use]`, which 70 e2e files read directly.

### 3 (c) When the session starts and what `bootSession` needs

- The session starts when the **first file of the first drop has been read** (its index exists): `exploreList` with that file. This is what a side-panel drop does today (`directDone` → `openStaged(false)`), only earlier: today `directDone` waits for the whole batch (`upload(fs).then(done)`, after `UPBUSY` reaches 0). D opens each file as soon as its upload resolves (`upload0` `one(f)` → append), so the side list grows file by file.
- `bootSession0` (`explore.js:441`) decides visibility on a first load: LR → only the first non-blank file shown (`vis=(k===first)`), HR → only the first base. **D: on a first load from the landing every file starts hidden** (`vis=false`), so the work area shows the sample table (rule state 2) and no panel is built. A first load is recognisable today (`isFirstLoad = !oldNames.length || seq || waiting`). Restore (`restoreUi`, notebook) keeps saved visibility, so a return visit still shows the panels it had.
- The **first click** on a file row in the side list (or on the file name in the sample table: same action, needed below 900 px and for the keyboard) = `soloFile(f)` (`lista-file.js:37`): that file shown, its tab set, `ensureLayout` builds the panels. From then on the side list behaves exactly as today.
- The `api/explore` payload carries `type`, `time`, `conc` from `ST.files`; edits after opening go through `CAMPIONI.apply` → `api/meta` (`set_meta`, exists in the engine and in `mzlab/explore.py`). Nothing in the engine needs to change.

### 3 (d) Proposals and write-through (a race to fix)

`RUOLI.propose` is asynchronous (engine `campioni/proponi`). `openStaged` sends the explore payload and then `RUOLI.commit(ST.files)` once. A proposal that lands **after** the commit updates `ST.files` only. In C this did not matter (the student pressed «Load data» after the proposals); in D files open as they arrive, so the commit can run before the proposals. Measured in §1 (row «Proposal race»): not reproduced today, because the side drop opens after the whole batch; D3 must add a test that delays `campioni/proponi` and checks `E.files` afterwards. Fix (D3): `RUOLI.propose` resolution and every table edit call `CAMPIONI.apply` for the rows whose file is open and not edited by the student (`ed`), quiet for proposals, undoable (`campioniEdit`) for student edits.

### 3 (e) Progress in the side list

During the upload (copy to OPFS, indexing: 0-100 % per row, `rowPc`) the side list shows nothing today: the progress is in `CONVROWS` rows of the hidden `#flist` and the side list only shows `#fl-note` «Adding…» (§1, timeline). D needs a progress row per file in `#flst` from the moment of the drop: the same `CONVROWS` state (phase, percent, error with «Details», cancel), rendered as a grey `.fl.pr` row (the class exists for `E.pending`). The rows sit in the order of arrival (`CONVROWS.rank`). A conversion (`.raw`, `.wiff`, `.d` through the converter) is one such row with its phases.

## 4. The hard questions

### (a) The teaching checkpoint

Today the student names blank, t0 and standards **before any chromatogram** because the table is a page and «Load data» is the only way on. In D the table is still the first thing the student sees after a drop, and **no chromatogram is drawn until a click** (rule state 2: every file starts hidden). What is lost is the explicit confirmation: a student can click a file without looking at the table. What keeps the step:

- The sample table fills the work area until the first click; nothing else is on screen (no panel, toolbar disabled), so the step is visible.
- Every value a student did not touch stays marked: the yellow `guess` of standards' concentrations (exists), and the same marking for proposed roles and times (new: `prop && !ed` → the `guess` class, same CSS variable).
- Roles and times stay editable after the click (Sample metadata, and the work area whenever no file is shown), undoable; everything that reads them (series `byTime`, gradient legend, blank subtraction, QC) reads `CAMPIONI` live, so a late correction redraws.
- Teoria and Pratica do not describe the loading page (old study §1.2): principle 5 is not involved.

Judgement, not measured: this is weaker than a forced confirmation but stronger than option B (in B the chromatogram came first). If Federico wants the forced step back, the cheapest form is to keep the first click disabled until every sample has a time; today's rule is the opposite («Load data» never blocked by a missing time, `ruoli-carica.js` header), so the study does not recommend it.

### (b) LR/HR mix

- The engine refuses a session with both worlds (`err.hr.mix`, `mzlab/explore.py:1050`; the Rust has the same check through parity).
- Today, first load: the table preselects the more numerous world (`hrMixPreselect`), `#hrmix` offers «Only LR (n)» / «Only HR (n)», `checkGo` keeps «Load data» disabled while both are ticked. Additions: `directDone` keeps the world of the open files, the other world stays unticked, `#fl-note` says «N added, M kept aside» (`files.drop.mixed`).
- **D:** one rule for first load and additions: **the world of the session is the world of the first file that finishes reading**; files of the other world stay in the sample table unticked, with `#hrmix` («2 high-resolution files set aside · Switch to high resolution»). The switch is today's «Replace» branch of `err.hr.mix` (`api/new` + `api/explore` of the other world, `shell.js`), with a «Removed · Undo» entry for the files it closes. For a whole folder whose minority world finishes first, the student switches with one click; preselecting by count is not possible because `f.hr` is known only after the file is read.
- HR «only the first file opened» (`bootSession0`) becomes «no file shown until the click», like LR: one rule for both worlds.

### (c) Session start, restore, desktop «Open with»

- Start: §3 (c). `bootSession` needs: the first-load visibility rule (all hidden), `syncWorkEmpty` replaced by the three-state rule, nothing else; `nbLoad`, `CAMPIONI.restore`, `restoreUi`, `toolbar()` run as today.
- Return visit: unchanged in substance from the old L4 (still valid): with the session flag (`mzlab.sessione`, `AVVIO.segnaSessione`) the page paints the work area with the side panel and `#opening` in «restoring» mode; then panels (or the sample table if the saved session had nothing shown). The landing is never painted when a session is expected.
- Desktop «Open with» (`predefinito.js` `take()` → `upload(files, "data")`): today the files land in the staging table and wait for «Load data»; in D they open at once like a drop. No code change in `predefinito.js`; the e2e with the Tauri mock (`tests/test_cartella_puntata.py` pattern) must assert they appear in the side list.

### (d) First paint and CLS

- Today `#v-data` is `hidden` in the markup; D shows it from the first paint (header, `#tools`, side panel, landing box), as the old L1 planned. The landing box is static HTML; `first-paint.js` / `app.css` decide the open side panel before scripts.
- **Side panel width.** `BARRA` computes the width as `min(defaultW, namesWidth)` (`barra.js:64`); with no file `namesWidth` returns the default, with files it returns the longest name + 110 px, clamped to 280-560 px: measured 300 px empty, 280 px with two short names, 340 px default in HR, so **the panel changes width when the first files arrive** and the work area moves by 20-60 px (§1). D: while no file has been shown yet (rule state 2/3) the width stays at the default (or the user's saved width); the names-based width is applied once, at the first click, under the panels' fade (rule of `apertura-fluida.md`).
- Toolbar: the buttons are already in the markup (`#tools`); disabling is an attribute, not a layout change (no shift). `#fsel` (file menu) is empty without files: give it the width it has with one file (min-width) so it does not grow.
- Target: CLS < 0.01 from navigation to the settled first panel, through landing → drop → table → click (D2 measures, D1 asserts).

### (e) Phone and < 900 px

- Phone (`telefono.js`, `?telefono=1`): unchanged, no Dati at all.
- Below 900 px the side panel, when open, is a **sheet that replaces the work area** (`SHEET = 900`, `barra.js:20`); measured in §1: at 820 px with a session the panel was closed and the work area took the width (the sheet itself was not opened in the run). An open side panel on the landing would therefore cover the drop zone. D below 900 px: the side panel starts **closed**; the landing (drop zone) and then the sample table fill the width; the sample table is the file list (each file name is the «open» action, §3 c); after the first click the side panel works as today (toggle, sheet).

### (f) Accessibility

- Focus order: header (tabs, tools) → toolbar (disabled buttons are skipped by Tab, which is right) → side panel (its «+» and its drop zone are not focusable; the list is empty) → landing: `#drop` first (`role=button`, `tabindex=0`, Enter/Space, `aria-label` `load.drop.aria`; today it is a plain `div`, old study §1.2), then folder, examples, method, conversion help.
- Live regions: one at a time. `#fl-note` (`role=status`) announces arrivals in the side list («3 files added»); the sample table box is not a live region (it would read every row); `#hrmix` (`role=status`) speaks once when files are set aside. `#work-empty` disappears.
- After a drop, focus stays where it was (the drop zone becomes the table's «Add files» line in the same box); after the first click focus goes to the first panel (as today).
- The sample table rows: the file name is a `button` («Open S_t15»), the × has `aria-label` «Remove S_t15» (exists as `load.table.remove` title; it needs an accessible name).
- Targets ≥ 24×24 px (40 at touch), axe on the four states (D6).

### (g) Deep links

`#/draw`, `#/theory`, `#/practice` never paint the landing: keep `html.in-draw #start{display:none}` on the landing box and the hash check in `first-paint.js` (old L1, still valid). `#/data` is the landing. Tests: `e2e_indirizzi`, `e2e_primo_disegno`.

### (h) Conversions and large folders

- Conversions: one progress row per file in the side list (§3 e) and its row in the sample table (`CONVROWS.html()` exists); the file opens when its mzML is read. Cancel from the row (`CONVROWS.onCancel` = `cancelLoads`).
- Large folders (hundreds of files): the table repaints at most once per animation frame during a loading (`RUOLI.coalesce`, `FILETAB.soon()`), so the cost that matters is one full repaint. Measured in §1 (row «render cost»). The side list builds one row per open file (`renderFileList`). Recommendation: no virtualisation in D; measured 34 ms at 300 rows and 150 ms at 1000 (§1), so D3 renders the table body in chunks of 200 rows per frame only above 500 rows, and keep the cap of the workers' pool as it is (no wasm cap rises).
- Opening a whole folder now opens every file (no tick to leave unchecked before «Load data»): the student unticks or removes afterwards. With hundreds of files the side list fills up and memory grows: the existing memory guard of the engine applies; a folder above a threshold (proposal: 50 data files) asks once «Open 312 files?» with «Open all» / «Choose…» (the second shows the table with every tick off). This is a real choice for Federico (§8).

## 5. Adding and removing data in D

### 5.1 Adding, at any time

| Entry | Before any file | With files open |
|---|---|---|
| Drop on the work area | the landing `#drop` | **anywhere on `#v-workspace`** (a full-area overlay `dz-over`, same as the side panel's) |
| Drop on the side panel | `#fl-dz` (open from the start) | `#fl-dz` (exists) |
| «+» `#addf` (side panel header) | opens the picker | a small menu «Files…» / «Folder…» / «Example files» / «Method (.dam)…» (the folder entry uses `CartPunt.choose`) |
| Folder pick, remembered folder | `#cartpick`, `#cartbody` in the landing | «+» › «Folder…»; the remembered folder's rows in the sample table box when no file is shown |
| Desktop «Open with» | opens | opens (adds) |
| Flow «Add files» (`data-flow-open`) | opens at once (exists) | same |

Every entry goes to `upload()` and then to the same per-file opening (§3 c): a progress row in the side list, then the file row; a row in the sample table (ticked if same world); the LR/HR rule of §4 b; the proposals of §3 d. Files added after the first click arrive **hidden**, as today's additions do (§1: `vis` false; `bootSession0` «later load: the panels stay»): the student ticks them in the side list.

### 5.2 Removing

| Action | Where | What happens | Undo |
|---|---|---|---|
| Remove one | × on the side-list row (exists in the row menu «Remove»; D adds a visible × on hover/focus), × in the sample table | `removeFile(f)`: `f.gone`, panels that showed it move to another file of the tab or close (`take`), `MAPPA.dropFile`; the sample-table row stays **unticked** (the file is still uploaded) until the undo window ends, then the row goes too | «Removed · Undo» (Ctrl+Z), 6 s before the OPFS copy is freed (`storia.js:105`) |
| Remove several | select rows in the side list (Ctrl/Shift click, exists for visibility) + «Remove» in the row menu; or untick several in the sample table | one `STORIA.tolto` entry for the group (the `removeAllFiles` pattern) | one undo |
| Remove all | «Remove all» in the side-panel menu (`removeAllFiles`, exists, no button today) | as above for every file; then rule state 3 (§5.3) | one undo; after 6 s final |
| Remove a file still uploading or converting | × on its progress row | `CONVROWS` cancel (exists) + the row goes | none (nothing was open) |
| Clear everything (files and session) | «New session» › «Files and session only» (`QQRESET.run("files")`, exists) | reload, empty landing | none (asks once, as today) |

What happens to things built on removed files:

- **Panels**: handled by `fileRemoval.take` (panels move to another file of the same tab; a panel left with no file closes with the files' visibility, `cleanupOrphanPanels`).
- **Sample metadata / CAMPIONI**: `CAMPIONI.files()` skips `gone`, so the tables update; the undo restores the values (`fileRemoval.back` keeps the file object).
- **Flow**: the plan reads the open files at run time (`F.runPlanFromData`); a missing file is excluded with the note «N files excluded» (`err.flow.input.missing`, `flow-grafica.md`). The sample grid of Flow (`S.samples`) keeps its document; nothing is deleted there. No change needed.
- **Notebook / session saved** (`uiSave`, `NB.session`): `gone` is saved (`explore.js:546`), so a reload does not bring removed files back.
- **Undo after the 6 s**: the OPFS copy is freed; Ctrl+Z no longer brings the file back (the entry left the stack). Same as today.

### 5.3 The last file removed

Rule state 3: no file open, none uploading, none set aside → the landing comes back in the same box (drop zone in the centre), the toolbar is disabled again, the side panel stays open (empty, `#fl-dz`), `S.sess` stays (the session keeps its notebook and settings; the next drop appends to it as a first load: all hidden, sample table). During the 6 s of «Removed · Undo» the landing is shown and Ctrl+Z brings the files and their panels back. Measured today (§1, row «Remove all»): after `removeAllFiles` two empty panels stay on screen; D5 closes them in `fileRemoval.take` when no file of their tab is left (the undo reopens them).

### 5.4 Where the old functions go

| Old | New |
|---|---|
| `#loaded` «N files already loaded» | the side list (always visible on ≥ 900 px) |
| «Clear loaded data» `#clearall` (two presses, `QQRESET.run("files")`) | «Remove all» in the side-panel menu (undoable) + «New session» › «Files and session only» (reload) |
| «Back to data» `#backdata`, «Cancel» `#cancadd`, `leaveAdding`, `_addY` | nothing: adding never leaves the work area |
| `S.adding`, `STATO.aggiunta`, `aggiunta` events | removed; `tools/controlla.py` and `STATO.verifica` lose the field |
| «Add files» `#addf` → loading page | «+» menu (§5.1) |
| `#hrmix-cancel` (→ `#cancadd`) | «Keep as is» (closes the note) |

## 6. Tests and compatibility

Counts on `main` (30/10), `grep -lF` over `tests_e2e/*.py` (470 scripts): `#pick` 194, `#drop` 109, `#opbtn` 95 (88 through `pg.click("#opbtn")`, the rest in text or evaluate), `#flist` 84 (70 read `#flist input[data-k=use]`), `stage(` 68, `#start` 18, `#addf` 5, `#loaded` 2, `S.adding` 2, `#cancadd` 1, `#clearall` 1, `#hrmix` 2, `#work-empty` 1.

**Minimal compatibility (no hidden button):**

1. Keep the ids `#pick`, `#drop`, `#flist`, `#start` (landing box), `#hrmix`; keep `#flist input[data-k=use]` (the tick means «in the session», §3 b). That alone keeps `stage()` and the 70 direct readers of `#flist`.
2. `#opbtn` is removed from the page. `tests_e2e/lib.py` already wraps `Page.click` (`_click`, for `_tempi`): the wrapper maps `"#opbtn"` (and the texts «Carica dati» / «Load data») to `lib.open_first(pg)`: wait until `UPBUSY === 0` and no progress row, then click the first file row of the side list (the student's first click). The 88 `pg.click("#opbtn")` files need no edit. `_tempi` writes the times through the sample table and `campioniEdit` (the files are already open). A hidden auto-clicked `#opbtn` was considered and rejected: it keeps a dead control in the DOM and a second code path.
3. Files to edit by hand: the 18 with `#start` (8 `wait_for_selector("#start:not([hidden])")` stay valid on the landing; the 3 `wait_for_selector("#start", state="hidden")` after opening become `lib.open_first`), the 9 that assert `#v-data` hidden (old study §5), the 7 with `#opbtn` outside `click` (evaluate, text), the add/clear group (`#addf` 5, `#loaded` 2, `S.adding` 2, `#cancadd` 1, `#clearall` 1: `e2e_hr_base`, `e2e_modo`, `e2e_ms2_ricarica`, `e2e_torna_ai_dati`, `e2e_unfile`, `e2e_elenco_file`, `e2e_progressivo`, `e2e_reset`, `tests/test_apertura_area.py`), the restore skeleton group (`e2e_avvio`, `e2e_avvio_shell`, `tests/test_avvio_fase3.py`, `tests/test_avvio_progressivo.py`), `e2e_pannelli_vuoto_csv` (`#work-empty`), and the tests that assert «only the first file opened» on a first load (`e2e_progressivo`, `e2e_hr_base`, grep `vis` after `#opbtn` in D3). **About 45 files by hand**, the other ~180 through `lib.py`.

## 7. Wireframes

Empty landing (≥ 900 px):

```
┌ mzLab · Dati Disegno Teoria ·············· ⚙ 🧮 · New session ⛶ ⓘ ┐
│┌ Files ──────── [+]┐┌ ‹ ▾ › │ +Chrom +Spec +Map +XIC +MRM │ ↶ ↷ ✕ │ Method ┐│  ← toolbar disabled
││                    ││                                                          ││
││  Drop files here   ││   ╔═══════════════════════════════════════════════╗     ││
││  to add them       ││   ║  Drop data files here or browse   (button)    ║     ││
││  (empty)           ││   ║  .mzML · .raw · .wiff (+.wiff.scan) · .D  ⓘ   ║     ││
││                    ││   ╚═══════════════════════════════════════════════╝     ││
││                    ││   📁 Open a folder   ·  or [ Example files ]             ││
││                    ││   + Acquisition method (optional) .dam                  ││
││                    ││   No mzML files? How to convert ▸                       ││
│└────────────────────┘└──────────────────────────────────────────────────────────┘│
```

Files arriving (progress rows in the side list, table filling in the work area):

```
│┌ Files ──────── [+]┐┌ toolbar still disabled ──────────────────────────────────┐│
││ BLK_01      blank  ││ Samples · 2 of 4 read          [+ Add files] [📁]        ││
││ S_t0        0 min  ││ ☑ File         Experiment  Type        Time[min▾] Conc[µg/L▾] ×││
││ ░ S_t15  ▓▓▓░ 62 % ││ ☑ BLK_01       Full Scan   Blank ▾     —          —       ×││
││ ░ S_t30  waiting   ││ ☑ S_t0         Full Scan   Sample ▾   (0)                 ×││
││ ⓘ 2 files added    ││ ░ S_t15        reading 62 %                               ×││
││                    ││ ░ S_t30        waiting                                    ×││
│└────────────────────┘└──────────────────────────────────────────────────────────┘│
   (0) = proposed from the name, yellow until the student confirms or edits
```

Sample table filled (all read; nothing shown yet):

```
││ BLK_01      blank  ││ Samples · 5 files · click a file to open it   [+ Add files]   ││
││ S_t0        0 min  ││ ☑ BLK_01     Full Scan  Blank ▾    —        —            ×    ││
││ S_t15      15 min  ││ ☑ S_t0       Full Scan  Sample ▾   0                     ×    ││
││ S_t30      30 min  ││ ☑ S_t15      Full Scan  Sample ▾   15                    ×    ││
││ STD_50   standard  ││ ☑ S_t30      Full Scan  Sample ▾   30                    ×    ││
││                    ││ ☑ STD_50     Full Scan  Standard ▾ —       [50]          ×    ││
││ [Remove all ▾]     ││ ☐ HR_x30 HR  Full Scan  Sample ▾   30                    ×    ││
││                    ││ ⓘ 1 high-resolution file set aside · [Switch to high resolution]││
```

After the first click (S_t0): the usual Dati page.

```
│┌ Files ──────── [+]┐┌ ‹ S_t0 › │ +Chrom +Spec +Map +XIC │ ↶ ↷ ✕ │ Method ┐│  ← enabled
││ ☐ BLK_01   blank   ││ ┌ TIC ─────────────────────────────────────────────────┐ ││
││ ☑ S_t0     0 min   ││ │                                                      │ ││
││ ☐ S_t15   15 min   ││ └──────────────────────────────────────────────────────┘ ││
││ ☐ S_t30   30 min   ││ ┌ MS1 ─────────────────────────────────────────────────┐ ││
││ ☐ STD_50  standard ││ │                                                      │ ││
│└────────────────────┘└──────────────────────────────────────────────────────────┘│
   Sample table: Statistica › Sample metadata, or untick every file (work area shows it again)
```

Below 900 px (side panel closed; the table is the list; a file name opens it):

```
┌ mzLab · Dati ··· ☰ ┐
│ [Files (5) ▸]       │   ← toggle of the sheet, count of files
│ Samples · 5 files   │
│ BLK_01  Blank ▾  —  │
│ S_t0    Sample ▾ 0  │
│ S_t15   Sample ▾ 15 │
│ (scrolls inside)    │
│ [+ Add files]       │
└─────────────────────┘
```

## 8. Decisions for Federico (yes / no)

1. Before files are loaded, the calculators (Calculator, Adducts, Losses, Lists) stay usable; only the buttons that need data are greyed out. **Recommended: yes** (today they work without data; a student can prepare masses before the files arrive).
2. Dropping a whole folder with more than 50 files asks once «Open all / Choose»; below 50 everything opens. **Recommended: yes** (a student who drops the whole course folder would otherwise open hundreds of files).
3. If low- and high-resolution files are dropped together, the first one read decides; the others wait in the table with a «Switch» button. **Recommended: yes** (the two cannot be open together; one click fixes the rare wrong guess).
4. Removing a file or all files asks nothing and can be undone for a few seconds; removing the last file brings back the empty page with the drop zone. **Recommended: yes** (it is how removing a file already works today).
5. On a narrow window (tablet, small laptop under 900 px) the file panel starts closed and the table itself is the list of files. **Recommended: yes** (an open panel would cover the drop zone).

## 9. Not verified

- Real browsers other than headless Chromium (Firefox, Safari, WebKit), touch devices, the sheet below 900 px with a finger.
- The desktop app (Tauri) and «Open with» from the OS: read from `predefinito.js` and `rust-desktop.md`, not run.
- The D behaviour itself: the measurements are of today's code paths that D reuses (side-panel drop, removal, hidden files, table cost); the new pieces (progress rows in the side list, write-through, three-state rule) are designs, not prototypes.
- The pointed folder with a real disk folder (`showDirectoryPicker` is absent in the cloud).
- Students' behaviour: whether the sample table without a confirmation keeps the naming step is a judgement (§4 a).
- Timings are from one cloud machine: compare only before/after on the same machine.

## 10. Build prompts

`docs/studi/prompt_landing_d/` (`LEGGIMI.md` = order and waves). Old prompts in `docs/studi/prompt_landing/`: L1, L3 superseded by D1, D3, D5; L2 reused as D2 (extended); L4 reused as D4 (restore and tour parts unchanged, the «Open with» part now asserts the side list); L5 reused as D6 (sheet rules changed by §4 e).

| Wave | Prompt | Zone (owned files) |
|---|---|---|
| 1 | D1 landing shell | `index.html` (landing box, `#v-data` visible, toolbar disabled), `applyView` (three-state rule), `app.css` landing rules, `first-paint.js`, `barra.js` width freeze and open-at-start, `lang/*/load.js` |
| 1 | D2 landing measure | new `tests_e2e/e2e_landing.py` only |
| 2 | D3 open at once + sample table | `shell.js` (upload → per-file opening, `directDone`, `#opbtn` handler removed, `openStaged`, `renderFiles` write-through), `explore.js` `bootSession0` first-load rule, `ruoli-carica.js`, `lista-file.js` (progress rows, `syncWorkEmpty` removed), `tests_e2e/lib.py`, D3 test list |
| 2 | D4 restore, desktop, tour | `avvio-progressivo.js`, `tour.js`, `predefinito.js` (read, fix only if needed), D4 test list |
| 3 | D5 add and remove | `eventi.js` `#addf` menu, `shell.js` adding leftovers (`S.adding`, `renderLoaded`, `leaveAdding`, `#cancadd`), `stato.js`, `lista-file-menu.js`, `reset.js`, work-area drop overlay, D5 test list |
| 3 | D6 small screens, axe, docs | `barra.js` sheet bits, `app.css` < 900 px, `e2e_landing_stretto.py`, axe states, `docs/agenti/` sweep |
