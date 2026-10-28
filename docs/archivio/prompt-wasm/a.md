# WASM-A · modulo `capacita` e avviso comune «non disponibile in questo browser: usa l'app»
**Ramo**: `claude/capacita` da `origin/main`. **Dipende da**: nulla. **Prima di**: D, E, F (leggono `capacita`).
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-A: ricevuto».

1. **Ruolo**: DEV Sonnet, pagina (JS + `lang/`). Nessun calcolo nuovo, nessuna dipendenza.
2. **Contesto**: regola di Federico (28/10) in `docs/studi/wasm-potenza.md` §1 e §6: si controlla funzione per funzione con feature detection (mai lo user agent); la funzione che manca si disattiva da sola con un avviso e un link all'app, si riattiva da sola quando il browser la supporta (si rileva a ogni caricamento, niente salvato); il resto del sito funziona sempre; nessun motore di riserva.
   - Oggi i controlli sono sparsi: `rust-supervisor.js:16` (`simdOk`, test `MZLAB_NO_SIMD`), `rust-bridge.js:93` (`error.noSimd`), `cartella-puntata.js:35`, `libreria.js:65,139,140` (`showDirectoryPicker`), `archivio.js:65` (OPFS), `libreria-worker.js:254` (`probeOpfs`), `mappa3d.js:213` (`nowebgl2`).
   - Leggi `docs/agenti/dati.md` (ordine degli script, `carica.js`, `QICON`), `docs/agenti/lingue.md`.
3. **Perimetro**:
   - nuovi `mzlab/web/capacita.js` (modulo ES, `// @ts-check`, in un progetto di `tools/tipi/`) e `mzlab/web/avviso-capacita.js`; chiavi nuove in `lang/it.js` e `lang/en.js`; i punti sopra passano da `capacita` (stesso comportamento visibile salvo l'avviso comune); un e2e `tests_e2e/e2e_capacita.py`; una riga in `docs/agenti/dati.md`.
   - Non toccare i ripieghi elencati in §5 dello studio (decide Federico), `sw.js` (WASM-D), il motore.
4. **Lavoro**:
   - `capacita.js`: `rileva(): Promise<Record<Nome, boolean>>` una volta per pagina e `ha(nome)`. Le sonde della pagina sono sincrone e costano ≤ 1 ms (misurato: 0,7 / 1 / 1 ms in Chromium 153, Firefox 157, WebKit 27.2);
   - quelle che vivono solo nei worker (`createSyncAccessHandle`, `OffscreenCanvas` 2D) NON aprono un worker apposta (worker nuovo + sonda OPFS: 491 / 175 / 45 ms): le riporta il worker del motore nel suo messaggio `ready`, e fino ad allora la funzione legata resta «in verifica», non disattivata.
   - Nomi: `simd128`, `opfs`, `opfsSync`, `cartellaLocale`, `webgl2`, `isolamento` (`crossOriginIsolated` e `SharedArrayBuffer`), `offscreen2d`, `locks`. Sonde wasm con `WebAssembly.validate` (byte come `banco-browser.txt`, funzione `wasmCaps`).
   - Tabella unica `REQUISITI`: funzione → capacità (`motore`: simd128+opfs+opfsSync; `cartella`: cartellaLocale; `mappa3d`: webgl2; `indiceParallelo`: isolamento). `serve(funzione)` → true, oppure false e mostra l'avviso.
   - `avviso-capacita.js`: sul comando disattivato `aria-disabled="true"` + testo `cap.missing` («{funzione}: non disponibile in questo browser. Usa l'app») con il link all'app (lo stesso di `aggiorna.js`); una sola riga riassuntiva per sessione nel registro degli avvisi esistente; colori da variabili CSS, tastiera, `aria-live="polite"`.
   - Interruttore di prova `?cap-off=nome1,nome2` (e `globalThis.MZLAB_CAP_OFF`), che assorbe `MZLAB_NO_SIMD`.
5. **Accettazione**:
   - `e2e_capacita.py`: per ogni nome in `cap-off` la funzione legata è disattivata con l'avviso visibile e il link, e Dati con un file d'esempio funziona per tutte le capacità che non sono del motore; senza `cap-off` nessun avviso.
   - `e2e_capacita.py` stampa il costo di `rileva()` nella pagina (mediana di 5) e fallisce sopra 5 ms; il numero va nella PR (riferimento: `python3 run.py chromium "only=none"` di `banco-browser.txt`, campo `wasmCaps_ms` = 0,7 ms).
   - `tools/controlla_i18n.py`, `tsc` (0 errori nuovi), `e2e_tour.py` verdi.
6. **Prove**: `python3 tests_e2e/e2e_capacita.py`, `python3 tools/verifica.py --cambiati`, `python3 tools/pre_pr.py`.
7. **Consegna**: PR aperta su `claude/capacita`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
