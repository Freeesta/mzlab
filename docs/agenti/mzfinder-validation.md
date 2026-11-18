# Feature validation (`tools/validate_feat.py`, `tools/valida/`)
The mzFinder core is in `mzfinder.md`. Tests: `tests/test_valida_tp.py`, `tests/test_valida_lr.py`, `tests/test_valida_hr.py` (synthetic series with truth built in the test).

- **Command**: `python3 tools/validate_feat.py` with one mode among `--cancello`, `--cancello-tp`, `--lr`, `--hr` (sections below); validation of the Rust engine tables is done by the gates.
  The truth (`{parent{mz,ion,rt_obs}, tps[{id,mz,ion,rt_obs[]}]}`) lives only in the private repository and is passed on the command line; none of it enters the public repository.
- **Tables** (`tabelle.py`): `trova_serie` lists the files of the series; `leggi_mzmine` reads the CSV export (`row m/z`, `row retention time`, `datafile:<F>:area` or `<F> Peak area`) on the columns of the series.
- **Metrics** (`metriche.py`): rows and rows per file; sensitivity (thresholds `floor` and `min_height` × 0.8 and × 1.2); time and peak memory; rows present in the blanks.
- **Isotopologues** (`isotopologhi.py`): reference peaks M, M+1, M+2 from the truth formula (the most abundant fine isotopologue of each nominal group, XIC ±5 ppm from the file where M is strongest); only those with height ≥ `min_height` count;
  completeness per table and relative error of M+1/M against the formula, on the table and on the reference traces.
- **True TPs and comparison**: TPs within ±5 ppm and ±0.15 min of the observed RT, with rank by maximum area in the treated files among all rows and among the filter candidates; Jaccard between two aligned tables by class of maximum area (< 10×, 10-100×, > 100× `EPS_FACTOR`·`min_height`).
- **Matching** (`abbina.py`): in unique pairs, by smallest normalised distance (ties: smaller index in the first list, then in the second). Two runs give the same JSON byte for byte.
- **Synthetics with identity truth** (`tools/valida/sintetici_identita.py --seme N --profilo piccolo|completo --out DIR`, tests `tests/test_sintetici_identita.py`): HR DDA series (`SIN_blank`, `SIN_dark`, `SIN_t0`, `SIN_tNNmin`) with invented molecules (`SIN-001`…); same seed = same bytes.
  - `verita.csv` and `verita.json` (format of `timolol_truth.json`: `parent`, `tps` = tp1/tp2/isomer, plus `ioni` with all rows): columns `id, ruolo, mz, rt_obs, ion, famiglia, progenitore, generazione, forma, purezza, isf_in_ms2, presente, k_form, k_dec, note`;
    roles and kinetics (saturated parent, monotonic, bateman, delay, flat, decreasing) are in the module docstring.
  - The truth is the acceptance criterion of the pipeline; the columns are fixed.
  - **Extended series V1X** (`--profilo completo --esteso` = profile `V1X`; `piccolo --esteso` = `V1X-piccolo`; modules `sintetici_esteso.py` M1/M2/M5, `sintetici_esteso_chimica.py` M3/M4, `sintetici_esteso_verita.py`; tests `tests/test_sintetici_esteso.py`):
    same model as the base profile (V1), plus the modules below, with their own random streams (`default_rng([seme, crc32("v1x/<modulo>")])`); without the option the V1 bytes do not change (SHA-256 of `verita.csv`/`verita.json` fixed in the test).
  - Besides the V1 files the extended profile writes `verita_tp.json` (`verita_tp/1`, `dataset: D-V1X`; above 25 KB the large lists go in `righe_esterne.json`) and `manifest.json` (seed, parameters, SHA-256).
  - **M1 chains**: 2nd and 3rd generation products with steps from the `fotocatalisi` table, in a tree (the first products are shared); the total change from the parent is never a rule, and for the 3rd generation neither are two (`raggiungibile_2_passi: false`).
  - M1 (continued): delayed maxima (chain of first-order steps, `t_max`); a family with one cleavage and three oxidations; a chain with the 3rd generation at 3-10× the noise.
  - **M2 product ISF**: constant ratio in [0.03; 0.3] and RT ±0.01 min; `n_isf_variabile` true products that co-elute with the ISF of another (`rapporto_costante: false`, `coeluisce_con_tp`); three cascading ISF, K adduct and dimer of the parent.
  - **M3 halogens**: Cl, Br, ClBr, Cl2 parents with one product that keeps the halogens and products −X+H and −X+OH (`alogeni` per row).
    **M4 isomers**: groups of 2-3 with ΔRT in {0.05; 0.1; 0.15; 0.2; 0.3} min, FWHM 0.08-0.15 min, partly identical fragments (70 % and 30 % of the first); `separabile_atteso` = two apexes in the sum profile.
  - **M5 disguised**: product ISF and adducts with an RT delay of exactly 0 or 0.02 min (`travestito`).
  - **Parameters** (`ESTESO` in `sintetici_identita.py`; default V1X → V1X-piccolo): `catene` {3: 6, 2: 12} → {3: 2, 2: 3}; `f_isf_tp` 0.5; `n_isf_variabile` 6 → 2; `n_travestiti` 8 → 4; `n_alogenati` 8 → 4; `gruppi_per_dt` 3 → 1;
    `escala_k` 1 → 3 (step speed); `t_limite` 72 → 16 min; `gap_tmax` 1.5 → 0.5 min. The rows of the added modules are less than 30 % of those of V1; `meta.budget_mb` declares the file size.

## Quality gate (`tools/valida/cancello.py`, `cancello_misure.py`, `nativo.py`; tests `tests/test_cancello.py`)
- **Command** (the reference measurement uses `--debole 0.006`, the synthetic products at 3-10× the noise): `python3 tools/validate_feat.py --cancello [--serie CARTELLA --verita V.json] [--mzmine T.csv] [--sintetico piccolo|completo] [--seme N] [--debole F] [--data AAAA-MM-GG] [--lavoro DIR] [--cancello-json FILE] [--senza-wasm]`.
  - Builds `mzlab-feat-wasm --example native` (the site pipeline) on every series and writes `TP_Mine/data/cancello.json` (`cancello/2`: `crate_version`, `data`, `vista_tabella`, `vista_candidati`, `concordanza_mzmine`, `criteri[] {id, valore, soglia, ok}`).
  - In `--lavoro` it writes `cancello_rapporto.json` and `.md` with all the numbers: the report with the real series stays in `.verifica/`, only aggregate numbers and versions go in the public repository.
  - The file is deterministic (sorted keys: same version, same data and same `--data`, same bytes); `valido(c, crate)` is the rule of `validated` in `PIPE.gateState`.
- **Rule**: `vista_candidati` is true only if all criteria of `ID_RICHIESTI` are present and `ok` (thresholds unchanged); `vista_tabella` (L1-L7 table and exports) only if those of `ID_TABELLA` are (rows, recall, area, noise false positives, isomers, determinism), a subset of the former:
  the criteria on kinetics, families, ISF, decoys and purity close only the candidates. `valido` and `valido_tabella` also read `cancello/1` (the table follows `vista_candidati`).
  - null `ok` = not measured: it closes the gate for the required ones, not for `mzmine_*` (optional: the CSV is missing) nor for `prestazioni_*` (speed and memory are not measured, by project decision).
  - Without a real series the four `timololo_*` stay not measured and the gate stays closed.
- **Real series** (13 files and truth; the parent is the one of the truth, with its `smiles` if the truth gives it: the native example passes it as the page does, `--smiles`, and the breakage fragments and groups come from it; without `smiles` the breakage hypotheses are missing):
  thresholds from V0, ≥ 21 TPs found, ≥ 19 among the candidates, ≤ 494 candidates, ≤ 25 606 rows.
  - Found = a row at ±5 ppm and ±0.15 min **or** a member of the identity of a row (the aligned table keeps the ions of a family in one row).
  - Candidate = identity with at least one piece of evidence in favour (`totale` > 0: the list the panel shows, the rest is "discarded"; frequency and controls alone give `totale` 0, `docs/agenti/mzfeat-tests.md`); in the details `candidati_con_ipotesi` counts those that confirm a hypothesis of the parent.
- **Synthetics** (built in the process by `sintetici_identita`: the truth thus has heights and noise peaks):
  - recall above 10× the noise (2e5) and between 3× and 10× (variant `--debole F`, product heights × F); the row or an identity member counts, and an isomer fused with its partner (valley ≥ 50 %) counts through the partner's row;
  - isomers with valley < 50 % on two rows; median area error across files (the area follows the height); rows in the blank that are neither noise nor background; families (isotopes and adducts in the identity of the base); declared ISF without MS2; decoys with a level better than 4;
  - kinetics via the identity: shape ("flat" accepts unresolvable formation) and generation or undecidable; purity: median error ≤ 0.05.
- **Determinism**: same SHA-256 of every `.mzfeat` and `.mzfeatf` in two native runs and between native and `.wasm` via Node (needs `wasm-bindgen` of the `Cargo.lock` version; without the tools the criterion stays not measured).
- **mzmine** (optional): mzmine rows with area ≥ 10× the threshold, matching at ±5 ppm and ±0.1 min; recall ≥ 0.90, precision ≥ 0.80, Spearman of the log areas ≥ 0.9, median of |log2 ratio| ≤ 0.25 after a single scale factor.
- **TP view gate** (`tools/valida/cancello_tp.py`, `rapporto.scrivi_cancello_tp`; tests `tests/test_cancello_tp.py`):
  `python3 tools/validate_feat.py --cancello-tp --serie CARTELLA --verita VERITA.json [--dataset NOME] [--cancello-tp-json FILE] [--uscita DIR] [--lavoro DIR] [--data AAAA-MM-GG]`
  writes `cancello_tp.json` in `TP_Mine/data/` (`cancello_tp/1`: `crate_version`, `data`, `dataset`, `vista_tp`, `criteri[] {id, valore, soglia, ok}`) and, in `--uscita`, `cancello_tp_rapporto.{json,md}`; G1 (`cancello.json`) stays as it is.
  - The truth is `verita_tp/1` or the historical keys (`tps[]`, `righe_isomeri[]`, `ereditati[]`); an entry of `tps` counts once (found if one of its RTs, or the m/z alone if it has none, has a row or an identity member).
  - The "raw view" is the set of pipeline candidates in panel order (points, then maximum treated area).
    Criteria: `tp_trovati` ≥ 22, `tp_in_vista` ≥ 20, `vista_dimensione` ≤ 200, `rango_mediano` ≤ 10, `rango_90` ≤ 40, `isomeri_333_righe` ≥ 2, `ereditati_come_tp` = 0 (inherited with constant ratio in view without `coeluisce_con_tp`).
  - Measured by the engine (`cancello_tp_motore.py`, requires the truth with `gen_attesa` and `level`): `generazione` (the generation written by the engine equals `gen_attesa`; `_n` = entries compared), `livelli` (`level`) and `determinismo_tp` (two runs, needs `--cancello-tp` with the second in `lavoro/bis`).
    `regione` stays not measured because the engine does not write the region (`regione_n` counts the entries); `richiamo_deboli` and `esche` are not measured; null `ok` keeps the view closed; `lr_tp` has no threshold and does not count. With the historical keys (real data) all stay not measured.
  - Raw measurement on the real HR parent series (13 files): 22/23 TPs found, 12 in view (132 candidates), median rank 10 and 90th percentile 35, 333 in 1 row, 0 inherited in view; view closed.
- **After every rebuilt `.wasm`** with a different version or behaviour, `cancello.json` is regenerated (the panel closes it if `crate_version` does not match).

## Low-resolution table (`tools/valida/lr*.py`; tests `tests/test_valida_lr.py`)
- **Command**: `python3 tools/validate_feat.py --lr [--serie-lr CARTELLA] [--verita V.json] [--lavoro DIR] [--lr-json FILE] [--seme N]`; prints the criteria and writes `cancello_lr.json` (`cancello_lr/1`: `crate_version`, `data`, `vista_tabella_lr`, `criteri[] {id, valore, soglia, ok}`) and `cancello_lr_rapporto.md` (aggregates only). The tool does not commit the JSON.
- **Synthetic series** (always, IDs `lr_sint_*`): full scan `TP_Mine/tests/lr_synth.py` and 6-level MRM (`lr_sintetici.py`). **Real series** (IDs `lr_*`): `B_MRM-*`, `B_FullMass-*` + `B_MS2-*` under `--serie-lr` or `$MZLAB_DATI/mzML`; the full-scan parent comes from the `parent` of `--verita`.
  Without a folder or without a parent the criteria stay null `ok` ("skipped", never "passed") and the view stays closed.
- **Engine**: `nativo.esegui` (`mzlab-feat-wasm --example native`, recipe chosen on the first index as the site does) for the full scan, `--example native_mrm` for MRM (`lr_nativo.py`); times from the names `-t30`/`_t30` become `_t30min`.
- **Full-scan reference**: the synthetic full scan is compared with the frozen ions of the old Python engine (`tests/golden/finder_lr_riferimento.json`: `forte`/`possibile` rows that are not the parent, extracted m/z): agreement within tolerances, never numerical parity.
  On the real series there is no public reference: the full-scan criteria stay not measured and only determinism is measured.
- **MRM** (`lr_criteri.criteri_mrm`): `lr_mrm_rilevate_livelli` = 1 (every transition with at least one detection in the standards is DETECTED at all levels above the lowest), `lr_mrm_area_cresce` ≥ 0.90 (Spearman = 1 over the detected levels),
  `lr_mrm_stati_coerenti` = 1 in the `-t*` files (detected or filled = finite area > 0, absent = no area, no targeted cell, every row with a detection). The log-log R² is reported in the details, it is not a criterion.
- **Full scan** (`criteri_fullscan`): `lr_ioni_python_in_tabella` ≥ 0.95 (every frozen ion has a row at ±0.35 Da and ±0.25 min), `lr_candidati_con_prove` = 1 (candidate = identity with formula; evidence = at least one "in favour" contribution, all with type, sign and source),
  `lr_isomeri_due_righe` = 1 (pairs with valley < 50 %: in the synthetics from the Gaussians, on real data not measured). `lr_determinismo` = 1: two runs, same SHA-256 of the containers.
- **Rule**: `vista_tabella_lr` is true only if all criteria (real and synthetic) are measured and pass; thresholds are not tuned on the TPs or on the standards, a failing criterion reports its number.

## HR robustness and agreement with mzmine (`tools/valida/hr*.py`; tests `tests/test_valida_hr.py`)
- **Command**: `python3 tools/validate_feat.py --hr [--robustezza CARTELLA] [--serie CARTELLA --mzmine CSV] [--lavoro DIR] [--hr-json FILE]`; writes `cancello_hr.json` (`cancello_hr/1`: `vista_tabella_hr`, `concordanza_mzmine`, `criteri[]`) and `cancello_hr_rapporto.md` (aggregates only). The tool does not commit the JSON.
- **Robustness** (files `Orbitrap_{Exploris120,Fusion}_DDApos_*.mzML` under `--robustezza` or `$MZLAB_DATI/HRMS`, without truth, one at a time with `nativo` without a parent): `hr_robustezza_gira` (the pipeline runs and finds rows), `hr_robustezza_deterministica` (two runs, same SHA-256),
  `hr_robustezza_soglia` (the preview proposes a finite `noise.floor` > 0), `hr_robustezza_stati_coerenti` (detected = area > 0, filled or targeted = area ≥ 0, absent = no area, no state outside 0-3); each = 1.
- **D5** (only with `--serie` and `--mzmine`): `mzmine_richiamo` ≥ 0.90, `mzmine_precisione` ≥ 0.80, `mzmine_spearman` ≥ 0.9, `mzmine_log2_mediano` ≤ 0.25 (`cancello_misure.concordanza_mzmine`, area ≥ 10× 50 counts·s, ±5 ppm and ±0.1 min, a single scale factor).
- **Rule**: `vista_tabella_hr` is true if the four robustness criteria are measured and pass and, if the CSV is present, so do the four mzmine ones; without files or without the CSV the criteria are null `ok` ("skipped", never "passed") and the view stays closed for robustness, open with respect to mzmine.
