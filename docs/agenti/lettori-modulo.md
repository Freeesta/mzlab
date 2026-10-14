# Modulo lettori: i formati che il motore non porta

Un `.wasm` a parte, caricato a richiesta, che apre i file dei produttori che il motore non legge e ne scrive l'mzML in OPFS; il motore lo legge poi come ogni mzML. Il `.wasm` del motore non contiene nulla del modulo (prova: stesso SHA-256 di `mzlab_wasm.wasm` prima e dopo; `tests/test_lettori_wasm.py`). Studio e piano dei pacchetti: `docs/studi/FORMATI_NATIVI_MS.md` §4.

## Crate
- `crates/mzlab-core`, feature Cargo `estesi` (vuota): varianti dei lettori del motore servite solo dal modulo, dietro `#[cfg(feature = "estesi")]`. `mzlab-wasm` non la accende (`cargo tree -e features -p mzlab-wasm`).
- `crates/mzlab-lettori` (libreria, accende `estesi`): `LETTORI_EXTRA` (i formati nuovi, una `VOCE` ciascuno), `registro()` = `LETTORI_EXTRA` poi `lettori::LETTORI`, `riconosci`, `apri(fs, limiti)` (il lettore più sicuro, avvolto da `SrmDaScansioni`), `estensioni()`, `motivi(l, c)`. 
- `srm.rs`: `SrmDaScansioni` aggiunge un cromatogramma `ChromKind::Srm` per transizione, id `SRM SIC Q1=<q1> Q3=<q3>` (letto da `mzmlhead::srm_id`), dalle scansioni di livello 2 con filtro SRM/MRM e precursore (i punti sono i prodotti; tolleranze `TOLLERANZA_Q1`/`TOLLERANZA_Q3`); non fa nulla se il lettore ha già cromatogrammi SRM.
- `crates/mzlab-lettori-wasm` (ponte `wasm-bindgen`): `estensioni()`, `probe(nomi, dimensioni, read)` → `{formato, campioni:[{id,name,scans}], motivi}` con `motivi` ⊂ `formato` (il motore non ha il lettore), `variante` (il motore lo ha ma rifiuta il file), `srm`, `pda`; `to_mzml(nomi, dimensioni, read, c, sink, progress)` scrive l'mzML a pezzi di 64 KB con `lettori_mzml::scrivi`. `Membri` e `error_json` sono copiati da `mzlab-wasm/src/nativi.rs`.
- Il desktop (`mzlab-desktop`: `nativi.rs`, `idxbuild_nat.rs`) apre con `mzlab_lettori::apri`: nessun tetto, nessun modulo da caricare.

## Costruzione e tetti
`tools/build_lettori_wasm.sh <site/static>` (ultima riga di `tools/build_feat_wasm.sh`, che `pages.yml`, `release.yml` e `sito-rust.yml` eseguono già) scrive `wasm-lettori/` e `manifest.json` con lo SHA-256. Tetti dopo `wasm_opt -Oz`: `lettori` 1 048 576 B; `lettori-tdf` 655 360 B (riga commentata nello script, per Bruker). Non salgono (`tests/test_lettori_wasm.py`).

## Pagina
- `lettori-worker.js` (worker modulo): scarica il `.wasm`, controlla lo SHA-256 del manifesto, risponde a `probe` e `toMzml` (scrive con `rawSink` di `rust-opfs.js`, avanzamento `{scans, scansTotal, written}`); un `toMzml` in corso non ascolta: la pagina ferma il worker (`cancel`).
- `lettori-modulo.js` (`// @ts-check`, modulo di `carica.js`): globale `LETTORI_MOD` con `extensions()` (le estensioni dei soli formati del modulo, `EXT`: ogni pacchetto aggiunge le sue), `probe(membri)`, `toMzml(membri, c, dest, onProgress, signal)`, `retriable(chiave)`, `stand(nome, membri, c)`, `mzmlName`, `loaded`. Il worker nasce alla prima richiesta.
- **Regola R**: `moduloToMzml` (`shell.js`, in `upload0` dopo `rawToMzml`) manda al modulo (a) i file con un'estensione di `extensions()`, (b) un campione che il motore rifiuta con `err.file.agilent`, `err.file.wiff` o `err.file.numpress` (`agilent-d.js` e `wiffToMzml` ne fanno un `File` con `mzlabModulo` tramite `stand`), (c) un insieme `.raw`/`.wiff`/`.D` per cui `probe` dà dei `motivi`, ma solo se il modulo è già caricato,
  (d) con `?lettori=modulo` ogni cartella `.D` (prova del percorso).
  L'mzML va in OPFS, entra in `upload()` come ogni mzML, sta in `CONV` («Scarica .mzML» non riconverte); righe «In conversione» di `CONVROWS`, pannello `ConvUI` con «Annulla». `fileKind` non rifiuta le estensioni del modulo. Errore di caricamento: `err.file.lettoriLoad`.

## Prove
`cargo test -p mzlab-lettori` (SRM con un lettore finto, `tests/robustezza.rs`); `tests/test_lettori_wasm.py`; `tests_e2e/e2e_lettori_modulo.py` (cartella `.D` sintetica con `?lettori=modulo`: stesso TIC del motore, mzML identico byte per byte, `MZLAB_LR_MISS` vuoto; serve `tools/build_lettori_wasm.sh site/static`). Nessun formato nuovo è letto ancora dal modulo: non c'è nessun file vero da confrontare con msconvert.
