# RM-5 — Nel browser: più worker per intervalli di scansioni, byte uguali al nativo

Prompt per una chat DEV: copia tutto ciò che segue la riga «---». Dopo RM-4.

---

Pacchetto RM-5 del convertitore Thermo `.raw` → mzML di mzLab. Leggi prima `docs/studi/raw-mzml-immediato/prompt/COMUNI.md` (parte «Due bersagli»), poi in `docs/studi/raw-mzml-immediato/` i criteri V8, V9 e F9 di `CRITERI.md`. Leggi anche `docs/agenti/dati.md` (worker, OPFS, `rust-worker.js`), `docs/agenti/wasm-tetti.md` e la riga SharedArrayBuffer di `docs/agenti/mappa-fluida.md`.

**Leggi**:
- il punto in cui la pagina chiede la conversione in mzML (`grep -rn 'mzML' mzlab/web/*.js | grep -i 'conver\|scriv'`);
- la rotta Rust che la serve.

**Tocca**:
- il codice Rust della conversione nel `.wasm` (un intervallo di scansioni per istanza, con l'uscita in pezzi ordinati);
- lo script del worker che li unisce in OPFS;
- i test e2e del caso;
- `docs/agenti/raw-mzml.md`.

**Non tocca**: il nativo (RM-4), salvo per estrarre in comune il nucleo per intervallo.

## Passi
1. **Nucleo per intervallo** [a, b) di scansioni: produce i byte degli spettri e i loro offset relativi, identici a quelli del nativo. Lo stesso codice serve il nativo a 1 thread.
2. **W worker** (W = `navigator.hardwareConcurrency − 1`, al massimo 4), ognuno con la sua istanza e il `.raw` letto a offset da OPFS (`createSyncAccessHandle`). L'unione è nell'ordine del file; SHA-1 e indice si calcolano nel worker che scrive.
3. **Misura** con 1 e W worker su LAB-A e LAB-B in Chromium (Playwright): tempo, picco di memoria (`performance.measureUserAgentSpecificMemory` se c'è) e dimensione del `.wasm`.

## Accettazione
- V8: 1 worker ≤ 1,7× il nativo a 1 thread; W worker ≥ 0,6·W.
- F9: SHA-256 del file scritto uguale al nativo.
- `.wasm` entro +10 % dichiarato e aggiornato nei tre file dei tetti.
- `e2e` del caso verde e `window.MZLAB_LR_MISS` vuoto.

## Prove
Il test e2e confronta l'SHA-256 dell'mzML scritto nel browser con quello della CLI sullo stesso file costruito.
