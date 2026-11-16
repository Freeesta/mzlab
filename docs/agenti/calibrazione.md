# Retta di taratura sui picchi integrati (`quant.curve@1`)
File: `crates/mzlab-core/src/calib.rs` (matematica pura: pesi, QR, R², inversa), `crates/mzlab-ops/src/run/table/curve.rs` (lettura dei CSV, scelte, testi JSON/CSV), `crates/mzlab-cli/src/flow_curve.rs` (descrittore e file del passo), `crates/mzlab-flowcal-wasm/src/lib.rs` (`flow_curve` nel browser), `flow/lazy/flow-web-quant.js` (esecutore del sito). Diverso da `quant.calibrate`, che applica curve importate e non stima nulla.

## Ingressi
- **responses** (CSV, `;`, tab o `,`; virgola decimale accettata se il separatore non è `,`): una riga per picco. Colonne: `sample`, nome dell'analita (`analyte` > `label` > `name` > `channel`), `area` e/o `height`; facoltative `snr` e `peak`. È la forma dell'esportazione delle integrazioni e di `chrom.transitions`; le righe `sum` sono saltate; `peak` diverso da `ok`/`edge` = nessun picco.
- **standards** (CSV): `sample`, `concentration` obbligatorie; facoltative `analyte` (vuota = tutti gli analiti), `level`, `unit`, `role`, `dilution` (fattore per la concentrazione finale, 1 se manca). Ruoli: `standard` (`std`, `calibration`, `cal`), `qc`, `sample` (`unknown`, `empty`), `blank`. Un campione dei picchi assente dagli standard è trattato come incognito (avviso `warn.flow.curve.unlisted`).
- Analita ripetuto nello stesso campione = errore `err.flow.curve.duplicate`; unità diverse tra gli standard di un analita = `err.flow.curve.unit`.

## Parametri
- `signal` `area` (predefinito) o `height`; `model` `linear` (predefinito) o `quadratic`; `weight` `1`, `1/x` (predefinito, proprietario), `1/x2`, `1/y`, `1/y2`; `zero` (retta per l'origine, falso).
- `istd`: un nome per tutti gli analiti o coppie `analita|IS` separate da `;`. La risposta diventa area/area dell'IS dello stesso campione; gli IS non sono tarati (riga del registro `internal_standard`).
- `exclude`: `analita|campione|motivo` separati da `;`, `*` = ogni analita. È l'unico modo di escludere un punto: nessuna esclusione automatica di anomali; ogni esclusione va nel registro con il motivo scritto dall'utente.

## Modello e solutore
- Minimi quadrati pesati con pesi relativi (scalarli non cambia la retta). Il disegno (1, x, x²; senza la colonna 1 se `zero`) è moltiplicato per √w e x è diviso per max|x|; il sistema si risolve con QR di Householder, mai con le equazioni normali (il condizionamento resta quello del disegno, non il suo quadrato).
- Rango insufficiente (colonna sotto 1e-10 della propria norma) = `err.flow.curve.singular`; meno punti dei parametri = `err.flow.curve.points`; valore o peso non finito = `err.flow.curve.value`. Questi tre errori restano nella curva (campo `error`), il passo continua con gli altri analiti (avviso `warn.flow.curve.failed`).
- Peso non definito (1/x con x = 0, 1/y con y ≤ 0): il punto è escluso con motivo `weight_undefined`. Uno standard a concentrazione zero entra quindi solo con pesi 1, 1/y, 1/y² (se y > 0).
- R² pesato centrato (anche con `zero`; `null` se tutte le y sono uguali); `s_yx` = deviazione standard dei residui non pesati, n − p gradi di libertà (il σ dell'ICH); `s_w` = la pesata. Con n = p non c'è σ (`null`).
- Inversa: lineare (y − a)/b; quadratica con la radice stabile 2(y − a)/(b + segno(b)·√(b² + 4c(y − a))), che vale anche per c → 0; nessuna radice reale = nota `no_root`.
- Test: `calib_tests.rs` (retta esatta, esempi di Miller & Miller §5.4 non pesato e §5.10 pesato, forma chiusa 1/x, Σxy/Σx² per l'origine, quadratica su tre decadi, disegni singolari, prova su 500 casi: ritorno degli standard entro 1e-9).

## Uscite
- **calibration** `<id>.json`, formato `taratura/1`: `signal`, `model`, `weight`, `zero`, `curves[]` e `log[]`. Ogni curva: `analyte`, `istd`, `unit`, `error`, `a`, `b`, `c`, `n`, `r2`, `s_yx`, `s_w`, `range` [min, max] degli standard usati, `lod[]`, `loq[]`, `points[]` (`sample`, `level`, `x`, `y`, `weight`, `used`, `reason`, `fit`, `residual`, `back`, `accuracy_pct`).
- LOD/LOQ, sempre con il metodo nel nome: `ich_3.3_s_yx_over_slope` / `ich_10_s_yx_over_slope` (3,3σ/b e 10σ/b, σ = `s_yx`, b = pendenza, per la quadratica la pendenza in zero; `null` se b ≤ 0) e `sn_3_lowest_standard` / `sn_10_lowest_standard` (media su k·x/(S/N) dello standard usato più basso con S/N). Il secondo è un'estrapolazione: lo dice il nome.
- Registro `log[]`: `analyte`, `sample`, `choice` (`excluded`, `treated_as_sample`, `not_fitted`), `reason` (`excluded_by_user`, `no_response`, `no_istd`, `weight_undefined`, `not_in_standards`, `internal_standard`), `note`.
- **concentrations** `<id>.csv`: `analyte,sample,role,level,signal,istd_signal,response,concentration,unit,accuracy_pct,note`; concentrazione = valore ricalcolato × `dilution`; `accuracy_pct` = 100·ricalcolato/nominale per standard, QC e bianchi con nominale > 0. Note: `ok`, `below_range`, `above_range` (solo per non-standard: fuori dagli standard usati, il valore resta), `no_response`, `no_istd`, `no_root`, `no_curve`, `excluded_by_user`, `weight_undefined`.
- Numeri con il `Display` di Rust (`{}`), uguale su nativo e wasm: stessi byte nell'app, nella CLI e nel sito.

## Dove gira
- CLI/app: `quant.curve` in `EXECUTORS` di `flow.rs`, due uscite (`Made.extra`); descrittore in `docs/schemi/steps-cli-1.json` (rigenerato con `MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib flow_ms2`).
- Sito: `flow_curve` del modulo `flowcal` (bridge `flowcal/2`, `wasm-tetti.md`), op `flow_curve` di `feat-worker.js`, esecutore `F.webQuant.curve` (`flow-live-jobs.js`): i picchi vengono da un file del piano o dal testo di un passo precedente; entrambe le uscite restano in OPFS.
- Parità: `crates/mzlab-cli/src/flow_curve_tests.rs` (file golden `crates/mzlab-cli/tests/data/calibration/golden.json` e `.csv`, `MZLAB_UPDATE_GOLDEN=1` per riscriverli), `tests/test_flowcal_curve_wasm.py` (wasm in Node = `flow run`, quattro gruppi di parametri), `tests_e2e/e2e_flow_web_quant.py` (sito = nativo).
- Dati di prova: `tools/synthetic_data_calibration.py` (8 livelli 0,5–100 ng/mL, bianco, QC a 15, quattro incogniti di cui uno sotto e uno sopra l'intervallo, analita lineare A, analita curvo B, IS costante, volume d'iniezione ±5 %), chiamato da `tools/dati_sintetici.py` nella cartella `taratura`.

## Non ancora
- Nessuna vista: il grafico della retta con i residui, la tabella degli standard con accuratezza, le esclusioni a clic (che scrivono `exclude`) e il collegamento diretto dalla tabella delle integrazioni sono il passo successivo.

## Letteratura
- ICH Q2(R2), Validation of analytical procedures (2023): LOD = 3,3σ/S, LOQ = 10σ/S, σ dai residui della retta; S/N 3 e 10.
- Almeida, de Castro Barbosa, Nogueira, J. Chromatogr. B 774 (2002) 215–222: scelta dei pesi 1/x, 1/x², 1/y, 1/y² per l'eteroschedasticità in bioanalisi.
- Miller & Miller, Statistics and Chemometrics for Analytical Chemistry, 6ª ed., §5.4 e §5.10: esempi usati nei test.
