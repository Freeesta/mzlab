# Grouping files by their names: «Group by», name variables, drag and drop

Study for Federico's request of 30/10/2026: when files arrive, mzLab understands how their NAMES split them (matrix, condition, time, role, concentration, replicate), offers a «Group by» menu built from that, keeps a manual fallback (token position, «contains»), and lets the user move files between groups by dragging, in the side list and in the loading area. Build prompts: `docs/studi/prompt_gruppi/` (order and dependencies in `LEGGIMI.md`).

## Summary for Federico
1. **The name parser already exists, in Rust** (`crates/mzlab-core/src/campioni/`): it reads role, time, concentration, replicate, matrix (`MQ_`, `TW_`) and the `MAT-COND` group from names like yours. What is missing is the general step: split every name into tokens, find which positions/kinds VARY across the set, and rank them as grouping keys. That is one new pure Rust function (`variabili`), served by the existing `api/campioni/<op>` route in the site and in the app: no new JS logic, no start-up weight.
2. **One group per file, one model**: the label `ugrp` that the FILE-GROUPS chat is adding to the side list (right-click «Group files», «Group 1», rename). «Group by» WRITES those labels (one undoable step); drag and drop EDITS them. No second, derived grouping living beside the manual one.
3. On your list the ranking is: Matrix (MQ 20 · TW 9), Condition (H2O2 8 · N2 4 · O2 4 · PMS 4 · AIR 4), Time (T00…T05, 6 each), Role (sample 23 · blank 4 · std 2 · QC 1), Concentration (2 files), Replicate (2 files). «Contains O2» does not catch H2O2 (whole tokens by default).
4. **Start-up budget is nearly spent**: the start-up scripts weigh 1 885 055 of 1 900 000 bytes (15 KB left). Every new page file is lazy (`carica()`), and the start-up scripts may grow by at most ~1 KB in total for the whole series.
5. **Seven prompts**, one after the other because they share the side list: 01 (Rust, can start now) → 02 «Group by» in the side list → 03 drag between groups in the side list (with its keyboard / menu alternative) → 04 «Group by» and dragging in the loading area → 05 drop a file on a file = new group, undo/redo → 06 group-aware actions (colour, fold, per-group role/time/concentration) → 07 accessibility and touch audit. 02 waits for FILE-GROUPS and D3 (#2207) to be merged; 04 also waits for D1b (#2211).
6. Questions for you (§9): whether «Group by» replaces existing manual groups at once (proposed: yes, undoable, with a status line), and whether new files arriving later join the groups by the same key (proposed: yes until you edit a group by hand).

## 1. What exists (reuse, do not rebuild)
- **Name parser** (`campioni/nomi.rs`, `docs/agenti/campioni.md`): `proponi` gives `tipo` (sample, blank, std, qc, dark, infusion…), `tempo` (Txx, t30min, 90 s, days), `concentrazione` + `unita` (1ppm, mg/L), `replica` (`(2)` or numbered per series), `serie` (prefix before the time), `matrice` (first word of 2-4 letters seen at the start of ≥ 2 names, with ≥ 2 such words) and `gruppo` = `MAT-COND`. `nomi-bianco.json` knows `wash`, `MQ`, `milliq` (blank only when every other token is neutral, so `MQ_PMS_T00` is a sample).
- **Positions** (`campioni/posizioni.rs`, `proponi_spiegato`): for each proposed field, the character span of the name that decided it. The tokeniser of 01 gives the same spans for every token, so the UI can underline the token a group came from.
- **ID convention** (`campioni/idcampione.rs`): `MAT-COND-Ttt`, `MAT-STD<cc>`, `QC-MAT`, `P-<name>`, `BLK`, `MQ_Wash`. Its vocabularies (`C N2 O2 AIR H2O2 PMS`, dark variants) are the known condition words of 01.
- **Name templates** (`campioni/modello.rs`, own regex engine in `regex.rs`): `{campione_id}_{polarita}_{tempo}min_r{replica}`. «Group by token N» of 01 is the regex-free little brother of this.
- **Loading table** (`renderFiles` in `shell.js`, `ruoli-carica.js`, `ruoli-celle.js`, `e2e_ruoli_tempi`): role, time, concentration per row (`ST.files`: `type`, `role`, `time`, `conc`, `prop`, `ed`); the late proposals of D3 (#2207) mark untouched proposals `gR` / `gT` (yellow). The rows keep the order of arrival (`CONVROWS.rank`); `loadGrp` gives `data-grp` (LR / HR / experiment) without group rows (`e2e_carico_gruppi`).
- **Side list** (`renderFileList` in `explore.js`): groups by experiment type in LR (`kindOf`), by `hr.<hrGroup>` in HR (`full`, `ms2`, `infusion`, `u:<ugrp>`), standard/sample/blank for MRM on the bench. Rows are already `draggable` with `bindFileReorder` / `bindDragDrop` (reorder, touch long-press 350 ms). Group headers `.fgh` fold (`E._grpOpen`, `flGrpGet` in compact HR), tick-all `.gall`. Undo of names, colours, visibility, order and `ugrp` is one `files.view` snapshot (`lista-file.js:4-11`).
- **FILE-GROUPS (running, session_01Ly8ApmEj4s4CEsXJfU3HS2)**: generalises `ugrp` to every list: multi-select, right-click «Group files» → «Group N» (`files.ugrp.name`), header `ugHead` (fold in `E._grpOpen`), F2/double-click rename, menu Rename / Ungroup / Remove group (`removeAllFiles(fs, name)`), `ugFiles(name)`, `ugrp` saved in the session and in `files.view` undo, label shown in the metadata table; tests `e2e_gruppi_utente`, `tests/test_gruppi_utente.py`. **Every prompt here builds on it**; its exact names must be read from its merged PR before starting 02.
- **Flow metadata grid** (`flow/lazy/flow-meta-grid.js`, `docs/agenti/flow-meta-griglia.md`): already «group by matrix, group or condition» as a visual band over `campioni/3` (`gruppo`, `serie`, `matrice`). 06 maps `ugrp` → `gruppo` when the grid is filled from the Data files (`F.samplesFromData`).
- **Tabui conditions** (`TABUI_GRUPPI_ESPERIMENTI.md`, `gruppi-campioni.js`): «Condition» = `serie`, «Set condition…» on ticked files. Same idea, different store: not merged here; a later step can offer «Use groups as conditions».
- **Start-up weight** (`tests/test_peso_avvio.py`): 1 885 055 / 1 900 000 bytes. New files go behind `carica(name)`; a few-hundred-byte hook in an existing start-up file at most.

## 2. Other tools: what to borrow
| Tool | What it does | Borrow |
|---|---|---|
| Skyline | On «Import results» proposes to remove the common prefix and suffix of the file names; replicate annotations; «Group by» an annotation in the peak-area charts | Constant tokens hidden and shown once as «common: MQ_…»; a «Group by» menu fed by per-file values |
| MZmine 3 | Sample metadata table (CSV import, typed columns: text, number, date); modules pick one metadata column to group or colour by | One dropdown listing the variables with their type; colour by the same key (06) |
| MS-DIAL | File property table: type (Sample/QC/Blank/Standard), Class ID, batch, order, included; class colours | Role and class are separate keys; class colour per group |
| Compound Discoverer | Study factors per sample; «Grouping & Ratios» ticks one or more factors and the groups are their combinations | «then by» a second key (Matrix × Condition = MQ·H2O2) |
| Progenesis QI | Experiment design: drag runs from a list into conditions, or create conditions from a file | Drag a file into a group header; «New group» drop target |
| XCMS (old phenoData) | Sample classes from the sub-folder names | The folder is a token too (dropping a folder tree: `PMS/MQ_T00.mzML`) |
| Excel Text to Columns, Power Query «split by delimiter», «column from examples», Flash Fill | Split by separators; infer the rule from one typed example | «Token N» fallback; later «by example»: name the group of one file, find the token that explains it |
| OpenRefine clustering (key collision, fingerprint) | Merges spelling variants (`blk`, `BLK`, `Blank`) | Case folding, leading zeros (`T0` = `T00`), role synonyms; fuzzy only as a suggestion |
| iOS / Android home screen | Dropping an icon onto another makes a folder named from the two | Drop on a file = new group, named from the common tokens (`MQ_H2O2_T00` + `MQ_H2O2_T01` → «MQ_H2O2») |
| macOS Finder stacks, Outlook «arrange by» | Group by kind/date with fold and counts | Header with count, fold state remembered |
| Lightroom smart collections | Rules «name contains …» | «Contains» filter with one group per term |

## 3. Discovery algorithm (Rust, `campioni/variabili.rs`)
Deterministic, no ML, linear in the number of names × tokens; same answer for every order of the input (the input order is only the order of `files[]` in the answer).

### 3.1 Normalise and tokenise
1. Name without folders and without the extensions of `EXT` (as `ordine.rs`: `x.mzML.mzidx` → `x`); folders are kept as extra leading tokens with `class: folder` (XCMS style) only when the names come from different folders.
2. Level-1 tokens: split on `_ - . space + , ( ) [ ]`; record `start`/`end` in characters of the full name (as `proponi_spiegato`).
3. Level-2 atoms, inside a level-1 token, only when a token has no separator and mixes kinds: letter↔digit boundaries and lower→Upper (camelCase) (`MQH2O2T00` → `MQ` `H2O2` `T00` when the letters before a digit run form a known word or chemical-looking piece; otherwise the token stays whole). Chemical-looking pieces (`H2O2`, `N2`, `O2`, `TiO2`, `Na2SO4`) are never split: letter+digit runs whose letters are element symbols stay one atom.
4. Fold for comparison: lower case, `µ`/`u` unified, leading zeros of a number dropped for the key (`T00` = `T0`), decimal `p`/`,` = `.`; the label shown keeps the most frequent spelling.

### 3.2 Classify tokens (reuse the parser, do not duplicate it)
Each token gets one class, from the vocabularies `nomi.rs` / `idcampione.rs` / `parole_bianco.rs` already hold (shared functions, no second list):
- `role` (blk, blank, bianco, wash, std, standard, cal, qc, pool, dark, buio, ctrl, control, infusion): value = the `tipo` of `proponi` for that file, so the role variable covers EVERY file (no role word = `sample`).
- `time` (T00, t30min, 30min, 2h, 90 s, `T2p5`): value in minutes, label as written.
- `conc` (number + `ppm ppb mg/L µg/L ng/L mM µM`): value converted to mg/L where the unit allows it, label as written.
- `rep`: a small integer (≤ 20) as the last token, or `(2)`, or `r2`/`rep2`; only when the same name without it occurs with another number, otherwise it is a plain number.
- `num`: any other number (an order, a vial, a date `20261030`).
- `word`: everything else; a word whose fold is in the known condition list (`C N2 O2 AIR H2O2 PMS`) is marked `cond`.

### 3.3 Build candidate variables
A variable = a rule giving each file at most one value. Candidates, all computed:
1. **Matrix**: the first word when `proponi` found a matrix (`matrice`); otherwise the first token if it has 2-6 distinct values each on ≥ 2 files.
2. **Condition**: for files with a time, the words between the matrix (or the start) and the time, joined with `_` (= `serie` minus the matrix); files without a time get no value (they are controls or standards, shown as «—»). This is what makes `MQ_std_BTZ_1ppm` and `QC_Std_5ppm_mid` stay out of the conditions instead of adding `BTZ` and `mid` as fake levels.
3. **Time**, **Concentration**, **Replicate**: from the tokens of that class (one per file; two time tokens = the one `proponi` used).
4. **Role**: `tipo` of `proponi` for every file.
5. **Matrix·Condition**: the pair, when both exist (`proponi` `gruppo` `MAT-COND`).
6. **Folder**: when the names come from ≥ 2 folders.
7. **Position N** (`pos:N`, manual): the N-th level-1 token (1-based, counted from the start; `pos:-1` = last), with the separators chosen by the user (`_`, `-`, `.`, space; default all).
8. **Contains** (`has:`, manual): a list of terms; a file goes to the first term equal (case-folded) to one of its tokens; with `substring: true` the term may be inside a token (`O2` then catches `H2O2`: the answer says how many files matched more than one term). Files matching nothing → «Other».

### 3.4 Describe and rank
For each variable: `coverage` = files with a value / files; `k` = distinct values; `balance` = normalised entropy of the value counts (1 = even); `kind` ∈ `categorical`, `ordered` (time, concentration, replicate, num), `constant` (k = 1), `identifier` (k = files with a value: grouping makes singletons).
- `score = coverage × balance × useful(k, n)`, with `useful = 0` for constant and identifier, `1` otherwise; ordered variables keep their score (time is a good grouping key: one group per time point).
- Hidden: constant variables (shown once as «common to all: …», Skyline style) and any variable with `score < 0.05`. Noise words (a `word` on one file only, like `mid`) never form a variable of their own.
- Sort: score descending, then the fixed order Matrix, Condition, Time, Role, Concentration, Replicate, Matrix·Condition, Folder, then `pos:N` by N. Ties are broken by the order, never by hash or input order.
- Values inside a variable: ordered variables by value (`T00 T01 T02 T05`), categorical by count descending then natural order of the label; «—» (no value) always last.

### 3.5 Worked example (Federico's list, completed with T00-T05)
Input: `MQ_blk, MQ_Wash_1, MQ_Wash_2, TW_blk, MQ_std_BTZ_1ppm, QC_Std_5ppm_mid`, `MQ_{H2O2,N2,O2,PMS}_{T00,T01,T02,T05}`, `TW_{AIR,H2O2}_{T00,T01,T02,T05}` (30 names; 20 MQ, 9 TW, 1 QC).
| Variable | Coverage | Values | Notes |
|---|---|---|---|
| Matrix | 29/30 | MQ 20 · TW 9 · — 1 | `QC` is a role, not a matrix |
| Condition | 24/30 | H2O2 8 · N2 4 · O2 4 · PMS 4 · AIR 4 · — 6 | `BTZ`, `mid`, `Wash` are not conditions |
| Time | 24/30 | T00 6 · T01 6 · T02 6 · T05 6 · — 6 | ordered |
| Role | 30/30 | sample 24 · blank 4 · std 1 · qc 1 | from `proponi` (if `proponi` reads `QC_Std_5ppm_mid` as std, the test follows `proponi`, never a second rule) |
| Matrix·Condition | 24/30 | MQ·H2O2 4 · MQ·N2 4 · … · TW·H2O2 4 · — 6 | H2O2 in two matrices stays two groups |
| Concentration | 2/30 | 1 ppm · 5 ppm | low score, listed last |
| Replicate | 2/30 | 1 · 2 | `MQ_Wash` with two numbers |
The 01 unit tests assert this table (exact JSON), plus the permutation and noise cases of §6.

### 3.6 Answer (`nomivar/1`)
`campioni(op="variabili", {"files":[{"name"}], "manuale"?: {"pos": N, "sep": "_-. "} | {"has": [terms], "substring"?: bool}})` →
`{"schema":"nomivar/1", "comune":"<constant prefix/suffix>", "files":[{"name","tokens":[{"testo","inizio","fine","classe"}]}], "variabili":[{"id","tipo","copertura","punteggio","valori":[{"chiave","etichetta","file":[indices]}],"senza":[indices]}], "manuale":{...same shape...}?}`.
`id` ∈ `matrix cond time role conc rep matcond folder pos:N has`; the page translates `id` with `groups.var.<id>`. A second op `nome_gruppo` (`{"names":[...]}` → `{"nome"}`) gives the name of a group made from files: their common tokens in order (`MQ_H2O2_T00` + `MQ_H2O2_T01` → `MQ_H2O2`), `""` when nothing is common (the page then uses «Group N»).
Both ops run in `mzlab_core::campioni::route`, so the `.wasm` (`lr_campioni` in `rust-worker.js`) and the desktop (`rpc_lr.rs` → same `route`) serve them with no new wiring; the CLI gets `mzlab-cli campioni variabili --names <file|->` (one name per line) with the same JSON, and a parity test runs the three on the same names (byte-equal).

## 4. Data model of groups (shared by FILE-GROUPS, «Group by», the loading table)
- **One label per file**: `ugrp` (string, trimmed, ≤ 40 characters, `""`/absent = no group), keyed by the physical file name (`fileBase`, so the two parts of a DDA move together). It lives on the open files (`E.files`, FILE-GROUPS) and, from 04, on the loading-table rows (`ST.files`); one writer `GRUPPI.set(names, label)` writes both and is one `files.view` undo step. Labels are unique case-insensitively; renaming to an existing label merges (with a status line).
- **Order and look**: `E.ugOrder` (labels in display order; new groups at the end; 06 adds reordering) and, from 06, `E.ugColor` (label → palette index of the Okabe-Ito set). Both in the session `ui` block next to the files.
- **«Group by» is an action, not a view**: choosing a key writes `ugrp` of every file from the value labels (`MQ`, `TW`; «—» files get no group). The key is remembered (`E.ugBy = {by, then?, manuale?}`) only to show it as selected and to group files that arrive later the same way; the first manual edit (drag, rename, «Group files») sets `E.ugBy = {by: "custom"}`. Undo of a «Group by» restores the previous labels and key in one step.
  Rejected alternative: a derived view plus per-file overrides. It needs two sources of truth, conflicts on every new file and makes undo and saving harder; Federico's drag-and-drop asks for edits, which a stored label serves directly.
- **Groups never change metadata**: moving a file from group «T01» to «T02» does not change its time. Changing role, time or concentration for a group is an explicit action (06: «Set role for this group…»).
- **Where the label goes next**: session (`ugrp` per file, already), Flow metadata grid (`gruppo` of `campioni/3`, 06; a `gruppo` already proposed by `proponi3` loses to a user label), exports that list files (QA/QC, report) show it as a column (06). Tabui conditions stay their own field (`serie`); «Use groups as conditions» is a later idea, not in these prompts.
- **LR nesting**: in the low-resolution list the experiment type stays the outer level (Full Scan › MQ › files), as FILE-GROUPS draws it; in HR the user group replaces the type group (`hrGroup` → `u:<ugrp>`), as today.

## 5. Interaction design
- **«Group by» control**: a small button in the side-list header (`#sb-header`, icon + «Group by ▾»; 40 px touch target) and, from 04, the same control above the loading table. The menu (a `role=menu` of `menuitemradio`) lists: «None», the ranked variables with a preview of their values («Matrix — MQ 20 · TW 9»), a separator, «Then by…» (submenu, one level), «Token position…» (a small dialog: separators, N, live preview of the groups), «Contains…» (terms separated by commas; checkbox «also inside words»), and «Custom» (checked, disabled) after a manual edit. The menu is built from one `variabili` answer cached per file set (recomputed after files arrive, debounced 300 ms).
- **Drag in the side list (03)**: a row dragged over a group header highlights the header (`aria-dropeffect` is deprecated: the state is said in a live region); dropping adds the file (and its ticked companions if the dragged row is ticked) to that group. Dropping on the gap between groups, or on «Ungrouped», removes the label. Reorder inside a group keeps working as today (`bindFileReorder`): the drop zone of a row is split in thirds: top/bottom third = insert before/after (reorder), middle third = drop ON the file (05).
- **Drop ON a file (05)**: new group with the two files (or the dragged selection plus the target), named by `nome_gruppo` or «Group N», placed where the target was; the name is selected for renaming at once (F2 behaviour).
- **Loading area (04)**: the loading table gets group rows (header with name, count, fold) only when groups exist; dragging a row onto a header or row works as in the side list; dropping NEW files from the desktop onto a group header adds them to that group. The drop zone `#drop` itself never changes size (D1b).
- **Single-pointer and keyboard alternative (03 ships it, 07 audits it)**: WCAG 2.2 SC 2.5.7 requires a non-drag way: a «Move to group ▸» submenu in the row context menu (right-click, Menu key, Shift+F10) with the existing groups, «New group…», «Ungrouped»; keyboard pick-up: Space on a focused row's grip «grabs» it, arrows move between groups, Space drops, Esc cancels, each step announced (`role=status`).
- **Touch**: the existing long-press (350 ms) starts the drag; auto-scroll near the list edges; drops on headers only (rows are too small for the thirds rule below 40 px: the middle third becomes «onto» only with a 600 ms hover).
- **Stability**: no element outside the list moves while dragging or regrouping; group rows have fixed height; the dragged row leaves a placeholder of the same height. CLS (layout-shift entries without recent input) = 0 in every new e2e; the boxes of `#tools`, `#dpanels`, header and `#drop` are unchanged before and after.

## 6. Tests the series must have
- Rust (01): Federico's list (table §3.5, exact JSON); a permutation gives the same `variabili` (same bytes after re-ordering `files`); noise (`mid`, `BTZ`, a stray `final`) creates no variable; `blk`/`BLK`/`Blank` one value; `T0`/`T00` one value; `O2` vs `H2O2` with `has` (whole token: 4 files; substring: 12, with the overlap count); camelCase and no-separator names (`MQH2O2T00`); names with folders; a single file and all-identical names (no variable, no panic); 10 000 generated names in < 50 ms in release; `nome_gruppo` cases. Parity: `.wasm` = desktop = CLI on the same input (byte-equal).
- Pages: node unit tests for the pure helpers (`tests/test_gruppi_*.py`, as `test_gruppi_utente.py`); e2e per prompt (`e2e_gruppi_per.py`, `e2e_gruppi_trascina.py`, `e2e_carico_gruppi_trascina.py`, `e2e_gruppi_su_file.py`, `e2e_gruppi_azioni.py`, `e2e_gruppi_a11y.py`) with: the synthetic names of §3.5 (files made by `tools/dati_sintetici.py` renamed, no lab data), CLS 0, undo/redo, reload keeps groups, `window.MZLAB_LR_MISS` empty, axe clean on the new controls.

## 7. Risks
- **Collisions**: FILE-GROUPS (`lista-file.js`, `lista-file-menu.js`, `explore.js` `renderFileList`, `ordine.js` `hrGroup`), D3 #2207 (`shell.js`, `explore.js`, `open-now.js`, `upload-rows.js`, `ruoli-carica.js`, `ruoli-celle.js`, `area-lavoro.js`, `carica.js`, `app.css`), D1b #2211 (`index.html`, `app.css`, `shell.js`, `tpmine-loader.js`, `lang/*/load.js`). Hence the strict order in `prompt_gruppi/LEGGIMI.md`; each prompt names its zone.
- **Start-up cap**: 15 KB left. All new page code lazy; the hook in start-up files ≤ 1 KB for the whole series; `test_peso_avvio.py` is never raised.
- **Wrong guesses**: a name scheme the algorithm misreads gives silly groups. Mitigation: every group is visible and editable, «Token position» and «Contains» are always there, nothing changes metadata, undo is one step.
- **Drag in a list that re-renders**: `renderFileList` rebuilds `#flst` while files arrive (D3). A drag in progress must survive a repaint (03: suspend repaints while a drag is active, apply them on drop).
- **Two places, one label**: the loading table and the side list show the same `ugrp`; a write in one repaints the other (one writer, `GRUPPI.set`).
- **Didactic rule**: groups are organisation, not answers. No group is named after a TP, a result or a «verdict»; the LR principle 1 is untouched.

## 8. Order (details in `prompt_gruppi/LEGGIMI.md`)
01 now (Rust only; zone `campioni/variabili.rs`, `campioni/mod.rs` route lines, CLI, tests, `docs/agenti/campioni.md`) → after FILE-GROUPS and D3 merged: 02 → 03 → 04 (also after D1b) → 05 → 06 → 07. 06 may run beside 04 if it stays in the side list.

## 9. Questions for Federico
1. «Group by» when manual groups exist: replace them at once (undoable, status line «Groups replaced: Undo») — proposed — or ask first?
2. New files arriving later: join the groups by the same key (proposed) until a manual edit, then land in «Ungrouped»?
3. Should Tabui offer «Use these groups as conditions» (writing `serie`) later? (Not in these prompts.)
