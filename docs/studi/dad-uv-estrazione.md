# UV/Vis detector (DAD) and wavelength extraction (XWC): study

Study for the request: «on the 3200 QTRAP every file also has UV/Vis; when a UV/Vis signal is present I must be able to change detector (MRM today cannot show the DAD); wavelength extraction does not work; it must work like the XIC: easy, fast, the program proposes a range from the data». Plus: the integration guard on the UV trace speaks of ions (`int.guard`). State read on `origin/main` at `e8029d4f`. No product code here; the PRs are in «Prompts».

## 1. Where the UV/Vis channel is read

| Source | What is decoded | Where (Rust) |
|---|---|---|
| SCIEX `.wiff` + `.wiff.scan` (3200 QTRAP with DAD) | full matrix time × λ in mAU, TWC from `DADIdx` | `crates/mzlab-core/src/wiff/dad.rs` (`DadData`, `read_dad`), `wiff/dad_uv.rs` (`dad_spectrum_json`, `dad_wavelength_json`), `wiff/dad_map.rs` (feature `mappa-dad`); layout in `docs/agenti/wiff-varianti.md` «DAD» |
| mzML written by mzLab (`convert.mzml` with `dad`, always on in the `sciex` module's `to_mzml`) | same matrix as `MS:1000804` spectra | `mzmlwrite/uv.rs` (write), `mzml_dad.rs` + `Mzml::dad_memo` (read once per file) |
| mzML from ProteoWizard of the same `.wiff` | TWC only (`docs/agenti/dati-3200.md`): no matrix | `mzml.rs` (`"TWC" => ChromKind::Pda`) |
| Thermo `.raw` | `UV_VIS_n` single-λ traces as `pda`; the 3D field is only a warning | `mzlab-lettori` thermo, test `thermo_data.rs` |
| Agilent `.D` | DAD (`AcqData/DAD1.*`) not decoded, listed with `meta.nonDecodificato`; DAD method read | `agilent_ext/reader_lc.rs`, `lcmodules.rs` |
| Waters | one UV wavelength as an analog trace | `mzlab-lettori/src/waters/chroms.rs` |
| Shimadzu | `PDA 3D Raw Data` not decoded | `mzlab-lettori/src/shimadzu/` |

Facts of a file: `Facts.pda` = the file has a `pda` chromatogram (TWC), `pda_rt` its range (`iteminfo.rs:782`). There is no fact «has a matrix»: the page probes it with `api/map?kind=pda` (`f._pdaMap`, `grafici.js:530`).

Private data (`Freeesta/mzlab-dati`): not reachable from this chat (attaching it was refused). Structural facts from the public docs only: the 3200 DAD has 191 wavelengths, 210–400 nm, 1 nm step, one row every 0.24 s (~5 000 rows in 20 min); the real-file test reads 32 files and 388 samples (`wiff-varianti.md`). PR1 below asks the chat to confirm on real files, counts only.

## 2. How it reaches site and app (parity)

| Route | Site (wasm) | App (desktop) | CLI |
|---|---|---|---|
| TWC `api/chrom?kind=pda` | `lr-method.js` → op `lr_pda` → `rust-worker-lab.js` → `item_pda` (`mzlab-wasm/src/items.rs`) → `itemroutes::pda_trace` | same JS → `rpc_lr.rs` `lr_pda` | `mzlab-cli channels FILE --channel uv:twc` |
| λ trace `api/chrom?kind=pda&mz0&mz1` (nm) | op `lr_pda_wavelength` → `item_pda_wavelength` → `itemroutes::pda_wavelength` | `rpc_lr.rs` `lr_pda_wavelength` | `--channel uv:<nm>±<bw>` (`channels_uv.rs` `Reading::Nm`) |
| UV spectrum `api/uvspec` | `lr_pda_spectrum` | same | `--channel uv:twc --rt T0[,T1]` |
| map `api/map?kind=pda` | `lr_pda_map` | same | — |
| UV peaks (Flow) | — | — | `chrom.uv_peaks@1` (`flow_uv.rs`, `mzlab_feat::uv_bands`) |

Rust logic is shared; the halves already match byte for byte where both are reached (`crates/mzlab-desktop/tests/dad.rs`, `mzlab-lettori/tests/sciex_dad_method.rs`, `channels_uv/tests.rs`).

## 3. Which panels offer the detector today

| Place | Detector switch | Why |
|---|---|---|
| Full Scan / EMS / Q1 tab, chromatogram | yes: picker «Rivelatori» → «PDA» (`canali.js` `others`, `hasPda` = a file of the tab has `pda`) | `pannelli.js:715` passes `hasPda` to `CANALI.html` |
| MS2 (Product Ion) tab, chromatogram | no | `ms2c`: the kind menu is a disabled «MS2 scans» (`pannelli.js:712`) |
| MRM tab (`p.type === "mrm"`) | no | `CANALI.html(p)` with no context: «only» mode lists transitions/ions, never `others()` (`canali.js:177-188`) |
| SIM files (Q1MI, shown in the MRM-type panel) | no | same as MRM |
| MRM file, chromatogram added by hand | no | `addPanel` never puts a `chrom` in the MRM tab (`pannelli.js` `addPanel`: tab → `full`/`ms2`); in that tab `hasPda` is false and `ctl` turns `kind=pda` back into `tic` (`pannelli.js:698`) |
| Map | yes, MS / PDA select when the matrix exists (`pdaMapOk`) | — |
| UV spectrum | from a double click on the PDA trace or map (`uv-spettro.js`) | — |

Reproduced on a synthetic 3200 `.wiff` with one MRM experiment and a DAD (`tools/wiff_riferimento.py` `Writer.sample(..., dad=...)`): one tab «MRM», one `mrm` panel, no «PDA» row; `E.files[0].pda === true`; `addPanel('chrom', {kind:'pda'})` lands in tab `full` as `kind: "tic"`.

## 4. Why wavelength extraction fails (root cause)

Reproduced on synthetic data (built site, Chromium):

1. **MRM/SIM files get the TWC for any wavelength.** `rust-bridge.js:175` sends `api/chrom` of a file with no scans to `infoRoute`; `rust-bridge-info.js:52-58` calls op `mrmchrom` with `kind` only, so `mz0`/`mz1` (the nm) are dropped and `itemroutes::mrm_chrom` returns the first `pda` chromatogram, the TWC. The request never reaches `lr-method.js`, which handles the λ correctly. Measured: `api/chrom?kind=pda&mz0=270&mz1=278` on the MRM+DAD file answers max 9 111.9 (the TWC; the 270–278 nm band peaks near 300 mAU) and no `nm0`/`nm1`.
  The same file with Full Scan blocks answers the band (`255–265`: max 383.9 mAU, `nm0: 255`). By reading (not run): the app uses the same `rust-bridge.js` and its `rpc.rs` `mrmchrom` ignores the nm as well, so both halves are wrong; a SIM-only file (no scans) takes the same branch. The 3200 lab files are MRM, so «extraction does not work» there.
2. **The λ panel of an MRM file is turned into a TIC.** `openWl` adds `chrom {kind:"pda", mz0:250, mz1:260}`; `addPanel` puts it in tab `full`, where no file has `pda`, and `ctl` sets `kind = "tic"`: a «TIC m/z 250–260» with nothing to draw.
3. **No range from the data.** The «λ» button and `chrom.wl` always open 250–260 nm (`eventi.js` `wlOpen` defaults); the TWC panel has no λ fields in «Intervallo» (`wlCtl` shows them only when `mz0` is set); `ctx.chrom.wl` ends in «…» but opens no dialog.
4. **λ offered without a matrix.** On a TWC-only mzML (ProteoWizard) the «λ» button is shown; clicking it gives the long toast `uv.none` (a how-to sentence).
5. **Integration refused on a single wavelength.** `picchi.js:69` `canInt` refuses every `chrom` outside MS2, so an XWC (a real, compound-specific trace) shows `int.guard`, which speaks of ions and offers an ion XIC.
6. **Weak test.** `e2e_uv_spettro.py` «wl_panel» asserts only `max(y) > 0`, which the TWC also passes; no test covers a DAD on an MRM `.wiff`.

Secondary facts: the y axis and export of a PDA chromatogram say «Segnale PDA (unità arbitrarie, AU)» (`export.col.pdaSignal`) while the values are mAU; all λ panels share the series key `c|k|pda` (fine one-per-panel, wrong if several λ go in one panel).

## 5. How the XIC works (the model to mirror)

- **Spectrum «XIC» split button** (`xic-spettro.js`): the main part extracts at once the chosen peak, or the most intense one when none is chosen (`step`), with the files' tolerance (`defTol`: ppm in HR, ±0.5 Da at unit resolution); Shift adds a trace to the same panel; the arrow opens a small popover with two fields (m/z, tolerance), Enter extracts, Esc closes; the XIC panel sits right under the spectrum, follows its scan, and says the apex.
- **Lens** (`xic-lente.js`, HR MS1 only): hover a peak → small transient XIC; click commits.
- **Dialog «Estrai ioni»** (`openXic`, `#xicdlg`): 1–10 rows, m/z or formula + adduct, remembers the last (`XIC_LAST`).
- **Edges** in the panel header (`xic-bordi.js`): «from / to» read on blur, refused pairs restore the old values, undoable (`COMANDI`).
- Same legend, `TDATI` table, export (`plotSheets`) and integration for every trace.

XWC mirror (proposal): on the TWC panel and on the UV spectrum a split button «XWC»: main part = one click, the engine proposes the band (λmax of the strongest UV peak, or of the spectrum in view); Shift = add as one more trace; arrow = popover with λ and bandwidth (nm), Enter/Esc; the trace opens under the source, linked in time; edges editable as today (`xic-bordi.js` already handles nm).
  Proposal rule (Rust, `wiff/dad_uv.rs`): apex of the strongest TWC peak (`uv_bands::uv_peaks` on the TWC), its spectrum minus the baseline spectrum (median of the rows at the run ends), λmax ignoring the lowest 10 nm (end absorption) unless nothing else absorbs, band default 4 nm (λ ± 2 nm, the usual DAD signal bandwidth); from a UV spectrum: λmax of the spectrum in view. A default, never an identification.

## 6. Terminology decision

**DAD** for the detector, **TWC** for the total trace, **XWC** for an extracted wavelength trace; **XIC** stays for ions. Reasons: SCIEX Analyst (the software of the 3200 QTRAP) names the device DAD and its traces TWC/XWC, the `.wiff` streams are `DAD/...`, the reader and docs already say DAD; Agilent, the usual LC in front of a 3200, also says DAD; «PDA» (Waters, Shimadzu, Thermo) is the same diode-array detector and goes in the tooltip: «DAD (PDA): UV/Vis diode-array detector». Group name in the picker: «UV/Vis».

Texts to change (values only, key names stay; IT / EN):

| Key | IT | EN |
|---|---|---|
| `ctl.kind.pda` | DAD (TWC) | DAD (TWC) |
| `ctl.kind.pda.title` | DAD (PDA): rivelatore UV/Vis a serie di diodi | DAD (PDA): UV/Vis diode-array detector |
| `chk.pda` | DAD | DAD |
| `panel.title.pdaAll` | DAD TWC | DAD TWC |
| `panel.title.pda` | XWC {nm} nm | XWC {nm} nm |
| `panel.title.mapPda` | Mappa RT-λ (DAD) | RT-λ map (DAD) |
| `export.col.pdaSignal` | Assorbanza (mAU) | Absorbance (mAU) |
| `disp.pdaDelay`, `disp.pdaDelay.title`, `disp.pdaAlign`, `disp.pdaAlign.title`, `a11y.mapPda`, `method.pda.title` | «PDA» → «DAD» | «PDA» → «DAD» |
| `panel.wl.title`, `ctx.chrom.wl`, `scorc.wl`, `cmd.mapxic.menuWl`, `ctx.uv.wlAt` | «XWC» (λ in the tooltip) | «XWC» |
| `uv.none` | Nessuna matrice DAD in questo file. | No DAD matrix in this file. |
| `int.guard` | unchanged for TIC/BPC (ions only: drop «/PDA») | same |
| new `int.guard.uv` | <b>TWC: somma di tutte le λ.</b> Estrarre una XWC? | <b>TWC: sum of all wavelengths.</b> Extract an XWC? |

Must NOT change (saved sessions, notebooks, tests, engine): `p.kind = "pda"`, `p.mapKind = "pda"`, `kind=pda` in every route, `api/uvspec`, ops `lr_pda*`, wasm `item_pda*`, `Facts.pda`/`pda_rt`, `ChromKind::Pda`, chromatogram ids `DAD TWC`/`DAD <nm> nm`/`TWC`, channel ids `uv:twc`/`uv:<nm>±<bw>`, `p.pdaAlign`/`p.pdaDelay`/`pdadelay`, `data-kind="pda"`, `data-o`/`data-a` values, `conv.dad`, feature `mappa-dad`, every lang key name.
  Tests that assert the old words are updated in the rename PR (expected text, not skipped): `tests/test_titoli.py:52-55`, `tests_e2e/e2e9.py:22`, `tests_e2e/e2e10.py:27`.

## 7. Order and files

| PR | Files it touches | After |
|---|---|---|
| 1 Route fix (MRM/SIM λ) | `mzlab/web/rust-bridge-info.js`, `crates/mzlab-core/src/itemroutes.rs` (`mrm_chrom` doc + test), `crates/mzlab-desktop/tests/dad.rs`, `tests_e2e/e2e_uv_spettro.py`, new `tests_e2e/e2e_dad_mrm.py`, `docs/agenti/wiff-varianti.md` | — |
| 2 Detector switch everywhere | `canali.js`, `pannelli.js` (`addPanel`, `ctl`), `sim-ui.js`, `esperimenti.js`, `lang/{it,en}/canali.js`, `tests_e2e/e2e_dad_mrm.py`, `docs/agenti/canali-dati.md` | 1 (shares the e2e file) |
| 3 XWC extraction | `wiff/dad_uv.rs`, `mzml_dad.rs`, `itemroutes.rs`, `mzlab-wasm/src/items.rs`, `rpc_lr.rs`, `rust-worker-lab.js`, `lr-method.js`, `mzlab-cli` `channels_cmd.rs`, new `mzlab/web/xwc.js`, `eventi.js`, `uv-spettro.js`, `uv-azioni.js`, `pannelli.js`, `lang/{it,en}/altre-press-xic.js`, `docs/agenti/wasm-tetti.md`, `wiff-varianti.md` | 1, 2 |
| 4 UV guard, XWC integration, rename | `picchi.js`, every `lang/{it,en}` file of §6, `tests/test_titoli.py`, `e2e9.py`, `e2e10.py`, new `tests_e2e/e2e_dad_integra.py`, `dati-3200.md`, `wiff-varianti.md` | 3 |
| 5 UV at an MRM peak, overlay | `uv-spettro.js`, `mrm` drawing in `grafici.js`, `sim-ui.js`, lang | 4 |

PR1 and PR2 touch different code files and may run in parallel only if PR2 creates its own e2e file (`e2e_dad_mrm_ui.py`); default: sequential. Defaults taken without Federico: DAD/TWC/XWC; band 4 nm; λmax search skips the lowest 10 nm.

## Prompts

Common rules for every prompt below (paste them with it):
- Read `CLAUDE.md` first and follow it: English in code, commits and PRs; branch `claude/<topic>` from `origin/main`; never push to `main`; one PR ready for review, never merged by you; never skip a test; no real data, names or paths in the public repo, synthetic fixtures only (e.g. `tools/wiff_riferimento.py` `Writer.sample(..., dad=(times, nm0, step, rows))`).
- All logic in Rust with wasm + CLI parity and a byte-for-byte parity test; JS is interface only.
- Federico's standing UI rule: no explanatory or how-to sentences on screen, only controls, labels, values, results and real errors; an explanation at most as a tooltip with `aria-describedby`.
- WCAG 2.2 AA: colours from CSS variables, Okabe-Ito, keyboard and `aria-label`, targets ≥ 24×24 px, every new chart with its `TDATI` table. Texts only via `lang/it` and `lang/en` keys (`tools/controlla_i18n.py`).
- A `.wasm` grows at most +10 % per new function, before/after in `docs/agenti/wasm-tetti.md`; never raise a cap in a workflow. Never raise the startup JS budget (`e2e_budget_avvio`); new JS is an ES module loaded lazily with `carica()`.
- Keep the guided tour valid (`tour.js`, `tests_e2e/e2e_tour.py`). e2e tests in `tests_e2e/` with `tests_e2e/lib.py`; WebKit/Safari must work. Before pushing: `python3 tools/controlla.py` and `python3 tools/pre_pr.py`.
- Docs in `docs/agenti/` describe the current state only. One PR at a time; tell the Coordinator the PR number and head SHA only when CI is green. Study: `docs/studi/dad-uv-estrazione.md`.

**P1 — The wavelength of an MRM/SIM file is the TWC (route fix).** Goal: `api/chrom?kind=pda&mz0=&mz1=` on a file with no scans (MRM, SIM) returns the wavelength trace, not the TWC, in the site and in the app.
- Read first: study §4 point 1, `mzlab/web/rust-bridge.js:160-176`, `mzlab/web/rust-bridge-info.js:40-60`, `mzlab/web/lr-method.js`, `crates/mzlab-core/src/itemroutes.rs` (`mrm_chrom`, `pda_wavelength`), `crates/mzlab-desktop/src/rpc.rs` (`mrmchrom`).
- Work: in `rust-bridge-info.js` the MRM `/api/chrom` branch serves only the kinds `mrm_chrom` serves (`tic`, `bpc`, `srm`) and returns `null` for `pda` and analog kinds, so `lr-method.js` answers them (no new logic in JS); in Rust document that `mrm_chrom` is not the UV route and add a test that a `pda` request with nm through the desktop ops gives `pda_wavelength`'s bytes.
- Tests: a synthetic MRM `.wiff` with DAD (two transitions, peak at 274 nm, weak one at 230 nm); new `tests_e2e/e2e_dad_mrm.py` asserts the 270–278 nm trace has its maximum near the synthetic value and differs from the TWC, has `nm0`/`nm1`, and the same for a SIM file; make `e2e_uv_spettro.py` «wl_panel» compare with the synthetic absorbance instead of `> 0`; desktop parity in `crates/mzlab-desktop/tests/dad.rs`. No wasm size change expected (say so).
  Also, with `tools/dati_percorsi.py` if the private repo is present, report only counts: how many lab `.wiff` samples have a DAD, their λ range and row count.

**P2 — The detector switch in every panel that has a UV/Vis signal.** Goal: wherever the files of a panel have a DAD or an analog trace, the channel picker offers a «UV/Vis» group (DAD TWC; pressure and the other analog traces as today); hidden when no file has it.
- Read first: study §3, `mzlab/web/canali.js` (`html`, `others`, `isOther`, `ensure`), `mzlab/web/pannelli.js` (`addPanel` tab choice, `ctl` lines with `hasPda`/`anKinds`, the `ms2c` kind menu), `mzlab/web/esperimenti.js`, `mzlab/web/sim-ui.js`, `docs/agenti/canali-dati.md`.
- Work: the MRM/SIM picker («only» mode) gets the group; choosing DAD opens a `chrom` panel `kind=pda` in the MRM tab, stacked under the MRM panel and time-linked (`addPanel` may place a `chrom` of kind `pda`/`press`/`an:*` in tab `mrm`; `ctl` computes `hasPda` from that tab's files, so it no longer falls back to TIC); the MS2 tab chromatogram gets the same group next to «MS2 scans». Interface only, no Rust.
- Tests: `tests_e2e/e2e_dad_mrm.py` (extend P1's file): MRM `.wiff` with DAD shows the row and opens a TWC panel in the MRM tab with the right values; a SIM file and an MS2 file the same; a file without DAD shows no row; keyboard in the tree (arrows, Space, Enter, Esc), axe clean; WebKit run.
- Texts: `lang/{it,en}/canali.js` group name «UV/Vis». Update `canali-dati.md`.

**P3 — XWC: extraction like the XIC, range proposed by the engine.** Goal: one click extracts an XWC with a band proposed from the data; Shift adds it to the same panel; a popover takes λ and bandwidth.
- Read first: study §4 points 2-4 and §5, `crates/mzlab-core/src/wiff/dad_uv.rs`, `crates/mzlab-core/src/mzml_dad.rs`, `crates/mzlab-feat/src/uv_bands.rs`, `crates/mzlab-core/src/itemroutes.rs`, `crates/mzlab-wasm/src/items.rs`, `crates/mzlab-desktop/src/rpc_lr.rs`, `mzlab/web/lr-method.js`, `mzlab/web/rust-worker-lab.js`, `crates/mzlab-cli/src/channels_cmd.rs`, `mzlab/web/xic-spettro.js` (the model), `mzlab/web/eventi.js` (`wlCtl`, `wlOpen`), `mzlab/web/uv-spettro.js`, `mzlab/web/uv-azioni.js`, `mzlab/web/xic-bordi.js`.
- Work: Rust `dad_propose(dad, rt0, rt1)` → `{nm, bw, rt, n}` (strongest TWC peak via `uv_peaks`, spectrum minus baseline spectrum, λmax skipping the lowest 10 nm unless nothing else absorbs, bw 4 nm; with a time window: λmax of that spectrum), route `api/uvpropose?k=&rt0=&rt1=`, op `lr_pda_propose` in wasm and desktop, CLI `mzlab-cli channels FILE --channel uv:auto` printing the same JSON, parity test of the three. If moving `dad_propose` next to `uv_peaks` needs `mzlab-feat` in `mzlab-core`, put the peak search in `mzlab-core` instead and say why.
- Interface: new lazy module `mzlab/web/xwc.js` (via `carica()`): a split button «XWC» on the TWC panel and on the UV spectrum (replaces «λ»), main part = proposed band, Shift = one more trace in the panel (series key with the nm), arrow = popover λ + bandwidth, Enter/Esc; the TWC panel's «Intervallo» gets the λ fields; `chrom.wl` and `uv.wlView` keep working and are undoable (`COMANDI`); the button is hidden when the file has no matrix (`f._pdaMap` false), so no `uv.none` toast on a TWC-only mzML. Same legend, `TDATI`, export and file colours as the XIC.
  Record the wasm size before/after in `docs/agenti/wasm-tetti.md`.
- Tests: Rust unit tests on synthetic matrices (two peaks, end absorption, flat), CLI test, desktop/wasm parity, `tests_e2e/e2e_xwc.py` (one click proposes 274 ± 2 nm on the synthetic MRM file, Shift adds 230 nm, popover by keyboard, TWC-only mzML has no button), tour valid, WebKit.

**P4 — UV guard, integrating an XWC, and the DAD/TWC/XWC words.** Goal: an XWC can be integrated like an XIC; integrating the TWC asks one short question and opens the XWC; the screen says DAD/TWC/XWC.
- Read first: study §4 point 5 and §6, `mzlab/web/picchi.js` (`canInt`, `guardInt`), `mzlab/web/titoli.js`, the lang files listed in §6, `tests/test_titoli.py`, `tests_e2e/e2e9.py`, `tests_e2e/e2e10.py`.
- Work: `canInt` true for `chrom` `kind=pda` with a band (`mz0` set); for the TWC `guardInt` uses the new `int.guard.uv` and «yes» opens P3's XWC with the proposed band; TIC/BPC keep `int.guard` without «/PDA»; change the values of §6 in IT and EN (key names unchanged); fix the y axis and export to «Absorbance (mAU)»; update the tests that assert the old words; rewrite `dati-3200.md` and `wiff-varianti.md` lines with the new words.
- Tests: new `tests_e2e/e2e_dad_integra.py` (integrate an XWC: area in the table; TWC: the guard text, «yes» opens an XWC), `tools/controlla_i18n.py`, tour valid, WebKit.

**P5 — UV spectrum at an MRM peak and TWC overlay (only after P4).** Goal: from an MRM transition peak, the UV spectrum at that time in one action, and the TWC as a second axis on the MRM panel.
- Read first: study §3, `mzlab/web/uv-spettro.js` (`open`, `p.link`), `mzlab/web/eventi.js` (`chrom.uvAtRt`), `mzlab/web/sim-ui.js`, the drawing of `mrm` panels in `mzlab/web/grafici.js`, `mzlab/web/gradiente.js` (second axis for pressure, the model).
- Work: a menu action and double click on the MRM panel open the linked UV spectrum when a file has a matrix (shifted by the DAD → MS delay when «Allinea» is on); a check «TWC» in the MRM panel's right-click menu draws the TWC on a right axis like the pressure overlay; interface only (routes exist).
- Tests: `tests_e2e/e2e_dad_mrm_overlay.py`, `TDATI` gets the TWC column, axe, tour, WebKit.
