# A simpler Flow for transformation products (study, no code)

## Riassunto (per Federico)
- Oggi il modello «Find transformation products» disegna 6 carte: Import data, Sample list, Parent structure, Build feature table, Predict transformation products, Link product spectra. «Sample list» e la tabella dentro «Import data» sono **lo stesso documento** mostrato due volte.
- «reused · 0 of 7 files centroided…» non è un errore: i 7 file erano già centroidi; la frase intera è «0 of 7 files centroided, 7 already centroided», tagliata dalla carta.
- I «6 errors» della tabella sono quasi sempre i tempi letti dai nomi dei file e non ancora confermati: il programma conta come errore una sua stessa proposta.
- Proposta: 5 carte da sinistra a destra: **Parent → Data → Features → Candidate TPs → Evidence & decisions** (+ Export). La lista dei campioni sparisce come carta e diventa «Advanced» dentro Data.
- Data mostra solo: quanti file, strumento (HR/LR) e modo (profilo/centroide) riconosciuti da soli, la tabella dedotta dai nomi in forma breve con «Edit». Il resto (CSV, JSON, SHA-256, colonne) in «Advanced».
- A bassa risoluzione (il vostro 3200 QTRAP) il modello deve usare «Dig unit-resolution series» (`tp.dig_lr`), non `tp.link` che è solo HR: oggi il modello non lo sceglie da solo.
- Dopo «Predict TPs» manca la carta che mostra le prove (spettri come Attributio, tabella come Tabui) e permette di decidere: c'è già quasi tutto (vista Products, `tp.review`), va solo messo in fila.
- Piano: 6 PR piccole; le decisioni tue sono segnate «Federico» (§6).

## 1. Inventory of the current TP template

`F.TEMPLATES[0]` (`flow/lazy/flow-names.js:94`) = base (`F.BASE`, `flow-names.js:62`) + add-on `tp` (`tp.generate`, `tp.link`). In Simple mode `F.canvasModel` (`flow/lazy/flow-fix.js:140`) folds the base into two group cards and draws every non-raw input as its own card, so the canvas shows:

| Card | Steps / input | Purpose | In → Out | Shown by default | Rarely needed | Duplicate? |
|---|---|---|---|---|---|---|
| Import data | `spectra.centroid` (only if added), `index.mzidx` | read files once, fast index | raw/mzML → mzidx | Add files/folder; metadata grid (`F.importBox`, `flow/lazy/flow-import.js:63`) | `budget`, all grid tools | grid edits the same `samples` input as the Sample list card |
| Sample list | input `samples` (`campioni/3`) | roles, times, series of each file | — → samples | the whole meta grid (`flow-meta-grid.js`) | Columns…, CSV/JSON, SHA-256, filters, grouping | **yes**: same document as the Import grid (`importBox` picks the first `samples` input, `flow-import.js:65`) |
| Parent structure | input `structure` (`flow-parent.js`) | formula/SMILES/RT of the parent | — → structure | text field | — | no (FLOW-PARENT reworks it) |
| Build feature table | `features.detect`, `align`, `gapfill` | one row per compound | mzidx → table | detect: polarity, floor, min_height, rt_range; align: mode, ppm, rt_gap, rt_warp; gapfill: floor, ppm, rt_tol | 16 advanced params | no |
| Predict transformation products | `tp.generate` | hypotheses from rules | structure (+table, mzidx) → transform | polarity, max_steps, min_mz | max_suspects, other_polarity, nominal | no |
| Link product spectra | `tp.link` (HR only: `requires.resolution: ["hr"]`) | match hypotheses to features, MS2 | table+samples+structure+mzidx+transform → tp | polarity, select_ppm, purity_min | purity_ppm, match_ppm | none, but wrong for LR data: LR needs `tp.dig_lr` |

Other TP add-ons: `tp.filter` (decoys, HR), `tp.position`, `tp.network`, `tp.review` (decisions, not in `F.NAMES`), `tp.dig_lr` (LR, 14 basic params, none advanced), `table.filter` (15 basic params, none advanced). Polarity is asked in three steps although `F.fileNature()` (`flow-samples.js:17`) already knows it.

**The «reused · 0 of 7 files centroided» message.** Built in two places:
- `flow/lazy/flow-live-view.js:36`: state `reused` → word «reused».
- `flow/lazy/flow-live.js:364-367` (`centroidNote`): `${done} of ${n} files centroided, ${already} already centroided`, where `n` = all files, `done` = profile files with a centroid index (`it.cen`, built by Data, maybe long ago), `already` = files Data classed as centroid. `flow/lazy/flow-live-card.js:36` appends it after « · ».
- The card cuts it with an ellipsis (`.fln-lw{text-overflow:ellipsis}`, `flow-shell-css.js`), so the user reads only «0 of 7 files centroided».
- `0` = no profile file needed centroiding; with 7 centroid files the full text is «…, 7 already centroided». The job is `cached` (`flow-live-jobs.js:33`), hence «reused». «0 … 0 already» also appears before Data classes the files.

**«6 errors, 1 warnings, 2 notes»** (`flow-meta-grid.js:191`): from Rust `valida`/`valida3` (`crates/mzlab-core/src/campioni/valida.rs:37-43`). Every `sample` row whose time was guessed from the name has `tempo_confermato: false` (`campioni/nomi.rs:1`) and gives `timeUnconfirmed` (error).

## 2. Proposed default workflow (5 cards + Export)

```
[Parent] ─┐
          ├─> [Find features] ─> [Candidate TPs] ─> [Evidence & decisions] ─> [Report / export]
[Data]  ──┘
```

| Today | Proposal |
|---|---|
| Parent structure | **keep** as «Parent» (FLOW-PARENT owns its content) |
| Import data + Sample list | **merge** into «Data»; the `samples` input is no longer a card in Simple (Advanced still shows it) |
| `spectra.centroid` | **hide**: inserted or not by the files (profile → added; centroid → absent); one line in Data |
| Build feature table | **keep** as «Find features» |
| Predict TPs + Link product spectra / Dig LR | **merge** into «Candidate TPs»: `tp.generate` + `tp.link` (HR) or `tp.dig_lr` (LR), chosen by `F.fileNature().resolution` |
| `tp.filter`, `tp.position`, `tp.network` | **hide in advanced** (add-ons of «Candidate TPs», on by default only in HR templates) |
| `tp.review` + Products view | **new card** «Evidence & decisions» (a view card like `F.STEP_VIEWS`) |
| export | **new card** «Report / export» |

**What «Data» shows by default** (progressive disclosure):
1. One line: «7 files · unit resolution (LR) · positive · centroided», with «edit» only where a choice exists.
2. Add files / Add a folder.
3. A compact read-only list: file, role, time, replicate (four columns, guessed values in italics with «guessed»), with «Confirm guesses» (one click sets `tempo_confermato` on all guessed rows) and «Edit table» (opens the grid).
4. One plain status line (see §3).
5. `Advanced ▸`: the full grid, Upload table, Fill from name pattern, Columns…, Export CSV, Download JSON, SHA-256, grouping, filter.

**Auto-detection rules** (all already in the engine or Data):
- Resolution HR/LR: `F.fileNature().resolution` (MS1 scans of the files). Mixed → ask.
- Profile/centroid: `ST.files[].spectrum_mode` (Data). Profile → add `spectra.centroid`; mixed → keep today's `profileIssues` choice.
- Polarity: `F.fileNature().polarity`, written into every step that has `polarity` (today only presets do part of it).
- Role, time, replicate, series: `proponi3` / `campioni::nomi` from the names; then `dal_file` (acquisition date, instrument, ranges).
- Still asked: the parent, and a confirmation of guessed times for a time series.

**Format compatibility**: merging cards changes only `F.canvasModel`, not `mzflow/2` (the `samples` input stays). `tp.dig_lr` vs `tp.link` affects new plans only; old `.mzflow` files open unchanged; no `step/1` bump.

## 3. Node status messages (before → after)

| Situation | Today | Proposed |
|---|---|---|
| centroid files, job cached | reused · 0 of 7 files centroided… | 7 files ready (already centroided) |
| profile files, index from Data | reused · 7 of 7 files centroided, 0 already… | 7 files ready (centroided earlier) |
| mode not known yet | reused · 0 of 7 files centroided, 0 already… | Reading 7 files… |
| any step, nothing changed | reused | Up to date (nothing changed) |
| stale | out of date | Settings changed: runs again on Run |
| blocked platform.computer | needs a computer | This step needs the desktop app (profile mzML). Use the app, or open the vendor file. |
| blocked permission.memory | needs your OK for memory | Large files: allow more memory. [Allow] |
| error | error: <engine text> | One plain sentence from `F.message` + one action button (`fixFor`) |
| grid summary | 6 errors, 1 warnings, 2 notes. | 6 times guessed from file names: check them. [Confirm all] · No blank in this series (optional) |

Flow stays English-only (no `lang/` keys).

## 4. After «Predict TPs»: candidates, evidence, export

- **Matching** (exists): HR `tp.link` (hypotheses ↔ features, MS2, `interp/1`) then optional `tp.filter` (decoys); LR `tp.dig_lr` (score + label, `report` JSON). Both read t0/blank/time from the sample list.
- **Evidence card** «Evidence & decisions», one row per candidate, Tabui-like:
  - table: m/z, RT, proposed change (Δformula), trend over time, score/label, level proposed (HR) — exists as the Products view (`flow-views-tabui.js:115`, `flow-views-tptab.js` kinetics with data table);
  - selecting a row (`F.select`) opens: XIC of the candidate vs parent, MS1 isotope pattern, MS2 vs parent MS2 as mirrored sticks (`F.sticks`, `flow-views-attributio.js`), the interpretation (`interp/1` view);
  - decision buttons (level, note, «not a TP») → `tp.review` (exists as step and «Decisions on the products» view, `flow-decisions.js:66`).
- **Missing**: (a) a view of the LR `dig` report (`tp.dig_lr` writes `report`, only the generic table view opens it); (b) a mirrored parent-vs-candidate MS2 plot in Flow (Attributio has sticks, not mirrored vs parent); (c) a «Report / export» card: CSV of decided candidates + evidence JSON. In the LR world (students) the card must show evidence only, never «these are the TPs» (principle 1).

## 5. Implementation plan (separate PRs)

| # | PR (zone) | Files | Risk | Tests | Depends on |
|---|---|---|---|---|---|
| 1 | Clear status words (Flow live) | `flow-live.js` (`centroidNote`), `flow-live-view.js` (`look`), `flow-live-card.js` | low | `tests/` node test of `look`/`centroidNote` strings; `e2e_flow_live*` text checks | none |
| 2 | Grid summary in plain words + «Confirm all guessed times» | `flow-meta-grid.js` | low; `tempo_confermato` already a field | `e2e_flow_meta_grid` | none |
| 3 | Merge Sample list into Data card; Data summary + Advanced expander | `flow-fix.js` (`canvasModel`), `flow-import.js`, `flow-inspect.js` | medium (tour, e2e selectors) | `e2e_flow_import`, `e2e_tour`, canvas node test | after FLOW-PARENT merges (both touch `canvasModel` input cards) |
| 4 | Auto polarity/resolution/centroid in the TP template; `tp.dig_lr` for LR | `flow-fix.js` (`templatePlan`), `flow-samples.js`, `flow-names.js` | medium: changes new plans | `tests/test_flow_grafico.py` cases HR/LR; parity: CLI `flow` on same plan | after 3; coordinate with TABUI-FLOW-VISIBILI (`flow-names.js`) |
| 5 | «Evidence & decisions» view card (Products + selected row: XIC, MS2 mirror, decisions) | new `flow-views-evidence.js`, `flow-names.js` (`STEP_VIEWS`) | medium; needs `TDATI` tables for every new plot | node reader tests, `e2e_axe_grafici` | after 4 |
| 6 | Report/export card + LR dig view | new `flow-views-dig.js`, export wiring | medium; parity rule: CLI must write the same files | parity test CLI ↔ browser | after 5 |

PRs 1-5 are page-only JS in `flow/lazy/`; 6 needs the same export in `mzlab-cli` (parity rule). No new dependency, storage or `.wasm`.

## 6. Decisions for Federico

1. Default card names (Parent, Data, Find features, Candidate TPs, Evidence & decisions, Report).
2. Whether guessed times count as **warnings** instead of errors (changes `valida` severity in Rust → `campioni` parity and saved documents' reading; PR 2 can instead only add «Confirm all»).
3. Whether `tp.filter`/`tp.network`/`tp.position` stay on by default in the HR template.
4. Whether the student (LR) world sees the Evidence card at all, and with which columns (principle 1).
5. Export content and file names of the Report card (new output files).
6. Any `mzflow/2` or `step/1` version change (none proposed).
