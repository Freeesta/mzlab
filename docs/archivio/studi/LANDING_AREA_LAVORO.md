# Landing on the work area (no separate loading screen)

Question (Federico, 30/10/2026): the site opens on the loading screen (`#start`: «1.» folder pick, drop zone, load table with type / time / concentration, «Load data»). Should the first page BE the work area, with loading (a) in the side panel (`#dfiles`) when files are already open and (b) in the empty work area (`#v-workspace` / `#dpanels`, `#work-empty`) when there are none?

**Short answer: yes, with one condition.** The gain is not a click less on the first visit (the student still drops files and confirms types and times), it is *one place* instead of two: the page never switches from a form to the work area and back. Today that switch happens on every first load, on every return visit (restore skeleton on `#start`, then `#v-data`), and every time «Add files» (`#addf`) is pressed (the whole work area is replaced by `#start` until «Load data»). The condition: the load table stays as the checkpoint before the FIRST opening of a session (it is where the student decides which run is the blank, which is t0, which are the standards), and it moves into the empty work area instead of being a page of its own. Recommended design: option C (§3); build prompts §7.

Measured / verified in this study: §1.1 (Playwright, headless Chromium, built site with the Rust engine, three synthetic Full Scan files). Not verified: §6.

## 1. Today's flow

| Situation | What the user sees today | Code |
|---|---|---|
| First visit (desktop) | Header with tabs and toolbar placeholder; `#start` page: card «1. Data files» (`#drop`, folder icon `#cartpick`, «or» + example files `#demobtn`, conversion help `#convhelp`), card method `.dam` (`#dropdam`), card with «Load data» (`#opbtn`, disabled). `#v-data` is `hidden`. | `index.html` `#start`; `applyView` in `shell.js` (`#start` shown when `!S.sess \|\| S.adding`) |
| Files picked / dropped on `#drop` | Rows appear in the load table `#flist` (order of arrival, proposals for type, time, concentration; conversion rows `CONVROWS` for `.raw` / `.wiff`); `#hrmix` if LR and HR are mixed; «Load data» enabled | `upload()`, `renderFiles`, `RUOLI.proponi`, `CONVROWS`, `checkGo()` |
| «Load data» | Same frame: `#v-data` with the opening skeleton `#opening` (side panel already at its final width, toolbar reserved), then panels fade in; CLS 0.0007 (`apertura-fluida.md`) | `openingStart`, `openStaged`, `bootSession` |
| Return visit with a saved session | `#start` drawn, the drop box replaced by «Restoring session…» (`#drop-skeleton`, same height), then the view switches to `#v-data` | `avvio-progressivo.js` `applicaIndizioSessione`, `init()` → `bootSession` |
| Drag and drop with files open | Drop on `#dfiles` (zone `#fl-dz`): files open at once, without the table (`directDone` → `openStaged`), pending rows in the list; other-resolution files stay unticked in the (hidden) load table with a note in `#fl-note` | `shell.js`, `dati-barra.md` «Rilascio sull'elenco dei file» |
| «Add files» button (`#addf`) with files open | The work area disappears: `S.adding` → `#start` again, with card «N files already loaded» (`#loaded`, «Clear loaded data»), «Cancel» (`#cancadd`) | `eventi.js` `#addf`, `renderLoaded`, `leaveAdding` |
| Folder pointed and remembered | Inside card «1.» of `#start`: grey list `#cartbody`, «Load» per row, «Load all» (`#cartgo`), «Allow access» after a reload | `cartella-puntata.js` (`CartPunt`), desktop `CartPuntDesk` |
| Phone | No Dati at all: `telefono-scheda.js` card (`#phone`, «Open anyway») | `telefono.js` |
| Installed app / offline (service worker) | Same page as the site, from cache | `sw.js` |
| 3200 LR teaching session | Table: Type (sample, blank, dark, standard, QC…; one blank type), time (minutes; dark negative), concentration only for standards (yellow `guess` until touched), «Load data» never blocked by a missing time | `ruoli-carica.js`, `ruoli-celle.js`, `dati-carico.md` |
| HR session (Orbitrap, Q-TOF, DDA) | Same table; «HR» tag on the row; only the first file is opened, the others listed; LR/HR mix note with «Only LR (n)» / «Only HR (n)» | `hrMixNote`, `bootSession0` |
| Desktop app, files opened from the OS («Open with», second launch, macOS `Opened`) | Paths queued (`take_opened`), registered like picked files and opened in the Dati view (`MZLAB_TOFILE`) | `rust-desktop.md` |
| Deep links | `#/data` (default), `#/draw` (Disegno, `html.in-draw #start{display:none}`), `#/theory`, `#/practice` (redirect to the standalone Teoria) | `hashView`, `first-paint.js` |

### 1.1 Measured with Playwright

Headless Chromium, built site with the Rust engine (`tools/prepara_sito.py`), viewport 1440×900, reduced motion, three synthetic Full Scan files (`dati_sintetici.full_scan`, t0/t15/t30). One run on one cloud machine; screenshots seen during the study, not committed.

| State | Measured |
|---|---|
| First visit | FCP 456 ms, LCP 496 ms, DOMContentLoaded 555 ms; `#start` occupies y 70-444 of a 900 px window (the rest is empty); CLS 0.0023 up to the staged table |
| Staged table | Columns: tick, File, Experiment, Role, Time (unit menu), Conc. (unit menu), Size, ×; the three rows proposed «Sample» with times 0 / 15 / 30 from the names |
| «Load data» → panels | skeleton then TIC + MS¹ panels; side list already shows the time of each file («0 min», «15 min») |
| «Add files» (`#addf`) | `#start` replaces the whole work area: card «Files already loaded (3)» with «Clear loaded data» and «Back to data», then the full card «1.» with the drop zone AND the three already-open rows, «Load data» disabled, «Cancel» |
| Return visit (reload) | 250 ms after navigation `#start` is shown with `#drop-skeleton` («Restoring session…») and `#v-data` hidden; panels after 0.55 s; CLS of the whole return 0.0049 (the switch `#start` → `#v-data`) |
| Keyboard (Tab from the top) | header tabs, gear, calculator, «New session», full screen, ⓘ, then `#cartpick`, ⓘ of the formats, `#demobtn`, `#dropdam`: **`#drop` is never reached** (no `tabindex`, no `role`) |
| 820 px (`?telefono=0`) / phone 390 px (`?telefono=1`) | 820: same card, full width; phone: the phone card, no Dati |

### 1.2 Things found on the way

- `#drop` (the main data drop zone) is a plain `div` with a click handler: no `tabindex`, no `role="button"`, no key handler (the method zone `#dropdam` has all three). Confirmed with Tab in Chromium: with the keyboard the only way to pick files is the folder icon (which, where `showDirectoryPicker` exists, points a folder instead of picking files). Whatever option is chosen, L1 fixes it.
- In «Add files» mode the table lists the files already open together with the new ones, and at 1440 px the last header («Dimensione») is clipped by the card; the files' time in the side list («0 min») is clipped by a few pixels too. Small, noted for L3 / L5.
- On the first visit the toolbar (Calculator, Adducts, Losses, Lists, Table) already works without files: the work area with no files is not «empty» for those tools, which is a point in favour of landing there.
- The Teoria and the Pratica never describe the loading screen (grep of «Carica dati», «tabella di carico», «schermata di caricamento» in `teoria/`): no Teoria text has to change, so principle 5 is not involved.
- A second place to edit roles after opening already exists: Statistica › «Sample metadata» (`campioni-tabella.js`, every edit through `CAMPIONI.apply`, undoable), visible in LR and HR.

## 2. What the load table does that must not be lost

| Function of the table | Why it matters | Where it lives in the new design (option C) |
|---|---|---|
| Choice of the files to open (tick `use`, ×) | Students drop a whole folder and open part of it | Staging table inside the empty-state card (first load); later additions: side list (pending rows already have ×) |
| Type / role (`roleCell`, one blank type, no t0 role) | Teaching: the student *decides* which run is the blank, the dark, the standards; the program only proposes from the names | First load: same cell, same proposals, same `RUOLI` model. After opening: file row menu «Type and time…» → Statistica › Sample metadata (exists) |
| Time (`timeCell`, minutes, dark negative, unit menu `#tunit`) | Kinetics and series (`byTime`, gradient legend, subtraction of t0) | Same as type |
| Concentration of standards (`conc`, unit `#cunit`, yellow `guess` until touched) | Calibration series; the yellow guess forces a look | First load: same cell. Later: Sample metadata (concentration column exists) |
| Order of arrival, `sortFiles` after a student edit | Students recognise their files in the order they copied them | Unchanged (same `renderFiles`) |
| Validation (`parseDecimal`, `aria-invalid` on bad numbers, `checkGo()`: LR+HR both ticked blocks «Load data») | Prevents a session that the engine would refuse (`err.hr.mix`) | Unchanged in the card; for additions with files open the existing rule of the side-panel drop applies (other world stays aside, `#fl-note`) |
| Conversion rows (`CONVROWS`: `.raw`, `.wiff` → mzML, progress, «Download mzML») | Long operations with their own progress and errors | Inside the card (first load). Additions with files open: pending rows of the side list (already show progress via `E.pending`) — conversions of later additions need a progress row there (L3) |
| Notes `#hrnote` (profile HR), `#modenote` (profile + centroid), `#hrmix`, `#unopen`, `#cartinfo` | Status, not errors | In the card under the table; with files open, `#fl-note` (`role=status`) |
| Method `.dam` (`#dropdam`, `#mlist`) | Optional MRM/MS2 method | Secondary line in the card; with files open: the toolbar «Method» entry (`#np-method`) already exists |
| Example files (`#demobtn`) | First contact without data | In the card (first visit only) |
| «N files already loaded» / «Clear loaded data» (`#loaded`, `#clearall`) | Reset of the file store | Moves to the «New session» dialog (`QQRESET.ask`, mode «files» does the same); `#loaded` disappears with `S.adding` |
| «Load data» as explicit confirmation | Teaching checkpoint; also the moment `S.opening` starts | Kept for the first load only |

Teaching intent: the exercise of experience 3 starts by naming the runs (blank, t0, t5…, standards). The table makes that an explicit step before any chromatogram is drawn; option B (§3) removes it. That is why option C keeps it for the first load.

## 3. Design options

All three keep the element ids of the loading pieces (`#pick`, `#drop`, `#flist`, `#opbtn`, `#hrmix`, `#cartpick`, `#demobtn`, …): 194 e2e scripts use `#pick`, 95 `#opbtn`, 108 `#drop`, 84 `#flist` (§5). Moving the markup keeps them working; renaming would not.

### Option A — relocate: `#start` becomes the empty work area

The header, tabs and toolbar are the same from the first paint; `#v-data` is always shown in the Dati view; with no session, the side panel is closed and `#dpanels` holds the start card (today's three cards in one column). «Add files» still swaps the panels for the card.

```
┌ header: logo · Dati Disegno · [toolbar, disabled] · New session · ? ┐
│                                                                      │
│        ┌──────────── 1. Data files ────────────── [folder] ┐        │
│        │   Drop data files here or browse                   │        │
│        │   .mzML · .raw · .wiff · .D                        │        │
│        │   ── or ──  [ Example files ]                      │        │
│        │   (staging table after the drop)                   │        │
│        │   + Add the acquisition method (optional) .dam     │        │
│        │                               [ Load data ]        │        │
│        └────────────────────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────────────┘
```

Gain: no page switch on first load and on return visits. «Add files» still hides the work. Effort: small-medium (one PR + test fixes).

### Option B — open on drop, edit afterwards

Like today's drop on the side panel, for every case: the drop opens the files at once with the proposed types and times; the table never appears. Types, times and concentrations are corrected afterwards (file row chips, Sample metadata).

```
┌ header ──────────────────────────────────────────────────────────────┐
│┌ Files ──────────────┐┌ work area ─────────────────────────────────┐│
││ ■ S_t0.mzML   t 0  ▾ ││  TIC / XIC panels                          ││
││ ■ S_t15.mzML  t 15 ▾ ││                                            ││
││ □ BLK.mzML  blank  ▾ ││                                            ││
││ ⟳ S_t30 (loading…)  ││                                            ││
││ [Review types/times]││                                            ││
│└─────────────────────┘└────────────────────────────────────────────┘│
```

Gain: one click and one screen less. Losses: the teaching checkpoint (students would start reading chromatograms with wrong types); yellow concentration guesses become silent; LR/HR choice becomes automatic; long conversions start without a chance to untick files; the whole folder is opened. Effort: large (roles editor in the side list, conversion progress there, `#hrmix` logic, rewrite of `e2e_ruoli_tempi`, `e2e_tempo_un_clic`, `e2e_carico_*`).

### Option C (recommended) — work area with inline staging; additions in place

First load = option A (the start card in the empty work area, staging table inside the card, «Load data» confirms; the card turns into the opening skeleton in the same box). With files open there is no staging screen any more: «Add files» (`#addf`) opens the file picker (or the folder list) and the files go the way a drop on `#dfiles` already goes: opened at once, pending rows in the side list, other-resolution files set aside with `#fl-note`. Types and times of added files: proposed as today; corrected from the file row menu «Type and time…», which opens Statistica › Sample metadata on that row.

Empty work area, first visit (≥ 900 px):

```
┌ header: logo · Dati Disegno · [toolbar: tools disabled] · New session ┐
│ (side panel closed: nothing to list)                                   │
│   ┌─ 1. Data files ─────────────────────────────── [📁 Open folder] ┐ │
│   │ ╔══════════════════════════════════════════════════════════════╗│ │
│   │ ║   Drop data files here or browse    (button, Enter/Space)    ║│ │
│   │ ║   .mzML · .raw · .wiff (with .wiff.scan) · .D (folder) ⓘ     ║│ │
│   │ ╚══════════════════════════════════════════════════════════════╝│ │
│   │  Remembered folder: Lab3 (12 files)  [Load all]                  │ │
│   │  ── or ──  [ Example files ]      No mzML files? ▸               │ │
│   └──────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

Same card after a drop (staging inside the card, the drop zone shrinks to one line):

```
│   ┌─ 1. Data files ───────────────────────────── [📁] [+ Add files] ┐ │
│   │  ☑ File              Experiment  Type       Time [min▾] Conc[µg/L▾]│
│   │  ☑ BLK_01.mzML       Full Scan   Blank ▾    —           —        │ │
│   │  ☑ S_t0.mzML         Full Scan   Sample ▾   0                    │ │
│   │  ☑ S_t15.mzML        Full Scan   Sample ▾   15                   │ │
│   │  ☑ STD_50.mzML       Full Scan   Standard ▾ —          [50]      │ │
│   │  ⓘ (hrmix / hrnote / modenote / unopen when relevant)            │ │
│   │  + Add the acquisition method (optional) .dam                    │ │
│   │                                         [Load data]              │ │
│   └──────────────────────────────────────────────────────────────────┘ │
```

«Load data»: the same box becomes `#opening` (title, phase, bar, file list, «Cancel»); the side panel opens at its final width in the same frame (rule of `apertura-fluida.md`), panels fade in.

Files open, adding more (side panel docked):

```
┌ header ──────────────────────────────────────────────────────────────┐
│┌ Files            [+] [×]┐┌ work area ────────────────────────────┐│
││ ■ BLK_01.mzML           ││  panels stay where they are            ││
││ ■ S_t0.mzML             ││                                        ││
││ ░ S_t30.mzML  loading… ×││                                        ││
││ ▓▓▓▓▓░░░ 1 of 2         ││                                        ││
││ ⓘ 1 HR file kept aside  ││                                        ││
│└─────────────────────────┘└────────────────────────────────────────┘│
   row menu (right click / ⋯): Rename · Colour · Type and time… · Remove
```

Below 900 px: with no files the side panel is closed and the card takes the full width (no change from today's `#start` width rules, `max-width:1240px`). With files, the side panel is already a full-width sheet (`sb-sheet`) that replaces the work area; «+» there opens the picker and the sheet stays.

Phone: unchanged (no Dati, `telefono-scheda.js`).

**Why C:** it removes every page switch (first load, return visit, additions) and keeps the one deliberate step that teaching needs. It reuses paths that already exist and are tested (direct drop on `#dfiles`, `E.pending`, `#fl-note`, Sample metadata), so most of the work is moving markup and deleting `S.adding`.

## 4. Risks and how each prompt handles them

| Risk | Detail | Handling |
|---|---|---|
| First paint | Today `#start` is in the HTML and painted before any script; `#v-data` is `hidden` until a session exists. The start card must still be in the first paint, now inside `#v-workspace`, with `#v-data` visible and the side panel closed | L1: `#v-data` not `hidden` in the markup for `#/data`; the card is static HTML; `first-paint.js` / `app.css` decide the closed panel before scripts. Measure first-paint and LCP before/after (L2 baseline) |
| Engine boot | Drop before the engine is ready already works (`avvio-progressivo.js` `pronto()` queues the upload) | Unchanged; L2 checks a drop at t≈0 |
| CLS | The card must not move when the session check resolves (restore) or when the toolbar fills | L1/L4: the card and the restore skeleton share one box (today's `minHeight` trick of `#drop-skeleton`); `e2e_apertura_fluida` + new `e2e_landing` assert CLS < 0.01 from navigation to settled area, for first visit, staging, opening, return visit |
| Return visit | Today: restore skeleton on `#start`, then switch to `#v-data` | L4: with the session flag (`mzlab.sessione`) the page paints `#v-data` with the side panel at its saved width and `#opening` in restore mode; no card at all |
| Focus order | Header → (side panel, only if open) → card. Today `#drop` is not focusable | L1: `#drop` becomes a real button (`role=button`, `tabindex=0`, Enter/Space), first focusable of the card; after «Load data» focus goes to `#opening` (as today); after the opening to the first panel (as today) |
| Live regions | `#work-empty` (`role=status`) and the card would both talk | L1: `#work-empty` is replaced by the card when there is no session; it stays only for «files open, all hidden» |
| Keyboard / reduced motion | Card ↔ skeleton ↔ panels | Opacity-only transitions with `--t-med`, none under `prefers-reduced-motion` (existing rule) |
| Small screens < 900 px | Sheet mode of the side panel | L5: e2e at 820 px: card full width, sheet «+» keeps the sheet; no horizontal scroll |
| Deep links | `#/draw` must never draw the card first (`html.in-draw #start{display:none}`) | L1 keeps the rule on the new element; `e2e_indirizzi`, `e2e_primo_disegno` |
| Session restore + pointed folder | `CartPunt` lives in card «1.»; «Allow access» after reload needs a click | The folder line stays in the card; with files open it is reachable from «+» (menu: Files… / Folder…) — L3 |
| Desktop «Open with» | `take_opened` registers and opens | L4 checks it goes straight to the work area (with Tauri mocked as in `tests/test_cartella_puntata.py`) |
| Tour | `tour.js` step that loads an example file and clicks `#opbtn` | L4: tour steps re-pointed if a target moves; `e2e_tour` |
| Tests tied to `#start` | §5 | Ids kept; each prompt owns its list |

**Effort** (chat-days, Opus/Sonnet with the CI): L1 1-1.5 (largest: markup move + `applyView` + ~18 test files), L2 0.5, L3 1, L4 1, L5 0.5. Total ≈ 4-4.5 chat-days in three waves.

## 5. Tests that depend on the loading screen

Counts on `main` (30/10), `grep -lF`:

- `#start` (18 e2e): `e2e15`, `e2e_agilent`, `e2e_apertura_fluida`, `e2e_avvio`, `e2e_avvio_shell`, `e2e_calc_avvio`, `e2e_calc_qui`, `e2e_campioni_salva`, `e2e_carica_subito`, `e2e_carico_icone`, `e2e_indirizzi`, `e2e_inglese_caricamento`, `e2e_inglese_dati`, `e2e_inglese_hr`, `e2e_primo_disegno`, `e2e_ruoli_tempi`, `e2e_taccuino_flush`, `e2e_titolo_scheda`.
- `#v-data` hidden asserted (9 e2e): `e2e_area_lavoro_misure`, `e2e_area_senza_sprechi`, `e2e_elenco_file`, `e2e_file_enormi`, `e2e_flow_avvio`, `e2e_memoria_misure`, `e2e_memoria_riduzioni`, `e2e_titolo_scheda`, `e2e_torna_ai_dati`.
- «Add files» / `S.adding` / `#loaded` / `#cancadd`: `e2e_hr_base`, `e2e_modo`, `e2e_ms2_ricarica`, `e2e_torna_ai_dati`, `e2e_unfile`, `e2e_elenco_file`, `e2e_progressivo`, `e2e_reset`, `tests/test_apertura_area.py`.
- Restore skeleton `#drop-skeleton`: `e2e_avvio`, `e2e_avvio_shell`, `tests/test_avvio_fase3.py`, `tests/test_avvio_progressivo.py`.
- `#work-empty`: `e2e_pannelli_vuoto_csv`. `#demobtn`: `e2e_carico_pulizia`, `e2e_esempi`, `e2e_inglese_caricamento`, `e2e_isolamento`, `e2e_lab`, `misura_macchine`.
- Through `lib.py` (`stage()`, `_sif` on `#pick`, `_click` on `#opbtn`): 194 / 95 / 84 / 108 scripts use `#pick` / `#opbtn` / `#flist` / `#drop`. They keep working only if the ids stay and `#flist` is still filled before «Load data» on the first load — option C keeps both; option B would break most of them.

## 6. Not verified

- Real browsers other than headless Chromium (Firefox, Safari, WebKit), real touch devices, the iPad sheet below 900 px with a finger.
- The desktop app window (Tauri) and «Open with» from the OS: read from `rust-desktop.md`, not run.
- The pointed folder with a real disk folder (`showDirectoryPicker` is not available in the cloud; `e2e_cartella_puntata` uses OPFS).
- Students' behaviour: whether keeping the table only for the first load is enough as a teaching checkpoint is a judgement, not a measurement.
- Timings are from one cloud machine (4 CPU, software GL): compare only before/after on the same machine.

## 7. Build prompts

`docs/studi/prompt_landing/` (`LEGGIMI.md` = order and waves). Waves of at most 3 chats on non-overlapping files:

| Wave | Prompt | Zone |
|---|---|---|
| 1 | L1 landing shell | `index.html` (`#start` block, `#v-data`), `applyView` in `shell.js`, `app.css` start rules, `first-paint.js`, `lista-file.js` `syncWorkEmpty`, tests §5 lists 1-2 |
| 1 | L2 measurement baseline | new `tests_e2e/e2e_landing.py` only (passes on old and new markup) |
| 2 | L3 add files in place | `eventi.js` `#addf`, `shell.js` `S.adding` / `renderLoaded` / `leaveAdding`, `stato.js` `aggiunta`, file row menu, `#loaded` → reset dialog, tests list 3 |
| 2 | L4 restore, desktop, tour | `avvio-progressivo.js`, `browser.js` restore path, `tour.js`, desktop open-with check, tests list 4 |
| 3 | L5 small screens + axe + docs sweep | `barra.js` sheet bits, e2e at 820 px, axe on the four states, `docs/agenti/` lines |

## 8. Decisions for Federico (yes / no)

1. The site opens directly on the work area (no separate loading page). **Recommended: yes.**
2. Before the first opening of a session, students still see the table where they choose type, time and concentration, and confirm with «Load data». **Recommended: yes** (it is the moment they name blank, t0 and standards).
3. Files added later (button «+» or drag onto the file list) open at once, without the table; type and time are corrected afterwards from the file's menu. **Recommended: yes** (it is already what dragging onto the list does).
4. High-resolution (research) users skip the table even the first time. **Recommended: no, not now** (one rule for everyone; reconsider after a semester).
5. The side panel stays closed until there is at least one file. **Recommended: yes.**
6. «Clear loaded data» moves into the «New session» window (it disappears from the loading page because that page no longer exists). **Recommended: yes.**
