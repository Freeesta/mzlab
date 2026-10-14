# DEV-00 — Infrastruttura comune dei moduli per famiglia

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-00 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Infrastruttura comune dei moduli per famiglia.
File d'area da leggere oltre a quelli comuni: docs/agenti/rust.md, docs/agenti/dati.md (sezioni caricamento e moduli), docs/agenti/dati-cartella.md, docs/agenti/test.md, docs/agenti/rust-desktop.md.

## Obiettivo
Preparare i moduli `.wasm` dei lettori, **uno per famiglia di formato** (`aperti`, `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`). Ogni modulo si carica solo quando arriva un file della sua famiglia, apre i file e ne scrive l'mzML in OPFS. Il motore poi lo legge come ogni mzML. Il motore non cambia di un byte. Questo pacchetto NON aggiunge formati: costruisce la base comune, il caricamento a richiesta, l'interfaccia comune, la scrittura in OPFS e i tetti. Lo prova con un lettore che esiste già (Agilent `.D` GC/MS).

## Da fare (in ordine)
1. `crates/mzlab-core/Cargo.toml`: `[features] estesi = []`, vuota, con il commento «varianti dei lettori servite solo dai moduli per famiglia, mai nel motore». Verifica con `cargo tree -e features -p mzlab-wasm` che `mzlab-wasm` non la accenda. Verifica anche che `pages.yml`, `release.yml` e `sito-rust.yml` costruiscano `-p mzlab-wasm` da solo, così l'unificazione delle feature del workspace non la accende. Leggi i file senza modificarli.
2. `crates/mzlab-lettori/` (libreria; `mzlab-core = { path = "../mzlab-core", features = ["estesi"] }`):
   - `[features]` vuote `aperti`, `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`, e `tutte` = l'elenco intero (per il desktop e per `cargo test`).
   - `src/lib.rs`:
     - `pub fn registro() -> Vec<&'static VoceLettore>`: per ogni famiglia accesa, i suoi `VOCE`. `agilent` e `sciex` ripetono quelli di `mzlab_core::lettori::LETTORI` (`agilent::lettore::VOCE`, `wiff::lettore::VOCE`) più le loro varianti; `thermo` ripete `rawlettore::VOCE` finché DEV-01 non lo sposta.
     - `pub fn apri(fs, limiti)` che usa `lettori::riconosci_in`.
     - `pub fn famiglia() -> &'static str`.
   - `src/srm.rs`: l'adattatore `SrmDaScansioni<L>`.
     - Avvolge un `Box<dyn Lettore>`.
     - Per le scansioni con filtro SRM/MRM (livello 2, precursore, punti = prodotti) aggiunge un cromatogramma `ChromKind::Srm` per transizione.
     - Id `SRM SIC Q1=<q1> Q3=<q3>` come `crates/mzlab-core/src/wiff/run.rs` (stesso formato letto da `mzmlhead::srm_id`).
     - Tolleranza Q3 in una costante.
     - Test con un `Lettore` finto scritto nel test.
3. `crates/mzlab-lettori-wasm/` (ponte, `crate-type = ["cdylib"]`, stessa versione di `wasm-bindgen` del `Cargo.lock`, come `crates/mzlab-lib-wasm`). Le sue feature inoltrano quelle di `mzlab-lettori`. **Interfaccia comune**, la stessa per ogni famiglia:
   - `famiglia()`, `estensioni() -> JSON`.
   - `probe(nomi, dimensioni, read) -> JSON {formato, campioni:[{id,name,scans}], motivi:[...]}`.
   - `to_mzml(nomi, dimensioni, read, c, sink, progress)`: stesso contratto di `native_to_mzml` (`crates/mzlab-wasm/src/nativi.rs:275`). Lettura a offset con la callback `read`, scrittura a pezzi di ~64 KB, `progress(done,total)`.
   - Interfaccia di sessione, per il lettore esterno del motore di DEV-01: `apri(nomi, dimensioni, read) -> handle`, `indice(h, c)`, `scansione(h, c, k)` (m/z e intensità come `Float64Array`, più l'intestazione in JSON), `cromatogramma(h, c, id)`, `intestazione(h, c, k)`, `metodo(h, c)`, `chiudi(h)`. Ogni risposta è JSON o array tipizzati e gli errori sono `{error, error_key, params}` come `error_json`. Prova nativa nel crate (`#[cfg(test)]`, `FileSet` in memoria).
   - Copia il tipo `Membri` di `nativi.rs`: non dipendere da `mzlab-wasm`.
   - Aggiungi i due crate ai `members` del `Cargo.toml` di radice.
4. `tools/build_lettori_wasm.sh <site/static>`:
   - Una tabella `famiglia tetto` (i tetti delle regole comuni) e, per ognuna, `cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-lettori-wasm --no-default-features --features <famiglia>`.
   - Poi `wasm-bindgen --target web --no-typescript --out-dir <site/static>/wasm-lettori/<famiglia>` (la stessa uscita di cargo cambia a ogni giro: copiala subito) e `tools/wasm_opt.sh ... <tetto> "lettori <famiglia>"`.
   - Alla fine scrive `wasm-lettori/manifest.json` `{famiglia: {sha256, byte}}`, come `tools/build_feat_wasm.sh`.
   - Una famiglia senza lettori (oggi tutte tranne `agilent`, `sciex`, `thermo`) si salta con un messaggio: nasce con il suo pacchetto.
   - Chiama lo script dall'ultima riga di `tools/build_feat_wasm.sh`, che `pages.yml:68`, `release.yml` e `sito-rust.yml` già eseguono: NESSUN workflow cambia.
5. `tests/test_lettori_wasm.py` (pytest):
   - La tabella dei tetti nello script è quella delle regole comuni e nessun valore sale.
   - `crates/mzlab-wasm/Cargo.toml` non nomina `mzlab-lettori` né `estesi`.
   - Ogni famiglia della tabella ha la sua feature in `crates/mzlab-lettori/Cargo.toml`.
   - Se `site/static/wasm-lettori/<famiglia>/mzlab_lettori_wasm_bg.wasm` esiste, la sua dimensione sta sotto il tetto.
6. Pagina:
   - `mzlab/web/lettori-famiglie.js` (`// @ts-check`, globale `LETTORI_FAM`): la tabella fissa famiglia → `{ext:[...], membri:[...], errori:[...]}`. Gli errori sono le chiavi del motore che mandano al modulo: `err.file.agilent` → `agilent`, `err.file.wiff` → `sciex`, `err.file.numpress` → `aperti`. Un test Rust in `mzlab-lettori` legge il file e controlla che le estensioni di ogni `VOCE` siano nella riga della sua famiglia.
   - `mzlab/web/lettori-worker.js` (worker modulo):
     - Alla prima richiesta di una famiglia fa `import("./wasm-lettori/<famiglia>/mzlab_lettori_wasm.js")`.
     - Controlla lo SHA-256 del manifesto come `TP_Mine/files/feat-worker.js`.
     - Tiene i moduli già caricati.
     - Messaggi `probe`, `toMzml`, `cancel` con la famiglia.
   - `mzlab/web/lettori-modulo.js` (`// @ts-check`, globale `LETTORI_MOD`): `famigliaDi(files, erroreMotore?)`, `probe(fam, members)`, `toMzml(fam, members, c, dest, onProgress, signal)`. Il worker nasce solo alla prima richiesta.
   - Aggiungi i due file alla lista `L` di `mzlab/web/carica.js` e al progetto giusto di `tools/tipi/` (0 errori). La CSP di `index.html` ha già `wasm-unsafe-eval` e `worker-src 'self' blob:`: non cambiarla.
7. Regola R in `upload0` (`mzlab/web/shell.js:216`), dopo `files=await rawToMzml(...)` (riga ~230): `files=await moduloToMzml(files,box,rep)`.
   - Raggruppa i file per famiglia con `LETTORI_FAM` (estensione o membro).
   - Riprova con il modulo della famiglia un campione che il motore rifiuta con un errore della tabella.
   - Scrive con `rawSink` di `mzlab/web/rust-opfs.js`.
   - Righe «In conversione» con `CONVROWS` (`mzlab/web/conversione.js`), nome con `convName`. «Scarica .mzML» passa da `CONV` senza riconvertire; «Annulla» ferma.
   - `fileKind`/`UNOPEN_RE` (`shell.js:169-170`) non rifiutano più le estensioni di una famiglia che ha un modulo nel manifesto.
   - Chiavi nuove: `err.file.lettoriLoad` (modulo non caricabile, con la famiglia nei `params`), `conv.row.mod` se serve una fase nuova.
8. Prova del percorso senza formati nuovi:
   - Il parametro di pagina `?lettori=modulo` manda al modulo `agilent` anche un `.D` GC/MS.
   - `tests_e2e/e2e_lettori_modulo.py`, con la cartella sintetica di `tools/agilent_sintetico.py`, controlla:
     - lo stesso TIC del percorso nativo (la stessa richiesta del TIC che fa `tests_e2e/e2e_lettori.py`);
     - «Scarica .mzML» identico byte per byte a quello del motore;
     - un solo modulo scaricato (le richieste di rete non contengono `wasm-lettori/sciex`);
     - `window.MZLAB_LR_MISS` vuoto.
   - Registra l'e2e in `tools/verifica.py`: dizionario `NEEDS` («sito», riga ~42) e mappa file → e2e (riga ~316).
9. Desktop (`crates/mzlab-desktop`): dipende da `mzlab-lettori` con la feature `tutte`. Il motore nativo apre con `mzlab_lettori::apri` ciò che `lettori::apri` rifiuta. Se il lavoro non è piccolo, scrivilo come «da fare» al Coordinatore.
10. Documenti:
    - nuovo file d'area `docs/agenti/lettori-modulo.md` (famiglie, crate, interfaccia comune, regola R, tetti, prove, «come si aggiunge una famiglia»), con la sua riga nella tabella §5 di `AGENTS.md`;
    - `docs/agenti/estendere.md`, «Aggiungere un formato»: un formato nuovo nasce nella cartella della sua famiglia di `mzlab-lettori`;
    - `docs/agenti/lettura-nativa.md`: una riga per la regola R.

## Atteso
- CI verde e `e2e_lettori_modulo` verde.
- SHA-256 del motore invariato.
- `site/static/wasm-lettori/{agilent,sciex,thermo}/` costruiti da `build_feat_wasm.sh`.
- «pages» riuscito dopo il merge, con le righe «lettori <famiglia> .wasm (optimised): N bytes» nel log. Scrivi i valori in `docs/agenti/misure.md`.

## File reali
Nessuno.

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`). Coordinatore 9 = `session_018GRRMEJ9JUr7Utr7ujQiXs`: a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: PR, CI, pages, cosa resta). Se sei bloccato o devi scegliere tra due comportamenti: PR aperta con la spiegazione e domanda al Coordinatore, niente merge.
- AGENTS.md è nel tuo contesto: rispettalo. Leggi SOLO: la sezione 4 di `docs/studi/FORMATI_NATIVI_MS.md` (un modulo `.wasm` per famiglia) (`grep -n '^## 4' docs/studi/FORMATI_NATIVI_MS.md`, poi lettura a intervalli), `docs/agenti/estendere.md`, `docs/agenti/lettura-nativa.md`, `docs/agenti/metodo.md` e i file d'area elencati sotto. Per trovare codice: `python3 tools/indice.py` e `grep -n`, oppure il subagente `cercatore`.
- Non toccare: `.github/workflows/`, `mzlab/web/teoria/`, il tetto del motore (1 703 936 B in `pages.yml`) e i tetti dei moduli (`tools/build_lettori_wasm.sh` e `tests/test_lettori_wasm.py`: `aperti` 393 216 B, `thermo` 458 752 B, `agilent` 393 216 B, `sciex` 327 680 B, `waters` 327 680 B, `shimadzu` 327 680 B, `bruker` 524 288 B). **Il `.wasm` del motore non deve cambiare** (329 byte di margine). Prova: `cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm` prima e dopo, stesso SHA-256 di `target/wasm32-unknown-unknown/release/mzlab_wasm.wasm` (eccezione: DEV-01 e i pacchetti che toccano il motore, DEV-09, 10, 11, che misurano e scrivono il peso).
- Dove va il codice: i lettori di una famiglia in `crates/mzlab-lettori/src/<famiglia>/`, dietro la feature Cargo con il nome della famiglia (`aperti`, `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`), con la sua riga in `mzlab/web/lettori-famiglie.js`; le varianti dei lettori che stanno già in `mzlab-core` dietro `#[cfg(feature = "estesi")]` (spenta per `mzlab-wasm`, accesa dai moduli). Una chat di famiglia tocca solo la sua cartella, la sua riga e il suo file d'area: le famiglie lavorano in parallelo. Rust puro, nessun crate nuovo salvo quelli che il prompt nomina. Codice e commenti in inglese; ogni testo visibile è una chiave in `mzlab/web/lang/it.js` e `en.js` (`tools/controlla_i18n.py`).
- Contratto `Lettore` (`crates/mzlab-core/src/lettori.rs`): `riconosci` guarda i byte (l'estensione da sola non basta), mai `read_to_end` oltre 8 MB, ogni numero letto dal file passa da `Limiti::voci_ok`/`profondita_ok` prima di allocare o ricorrere, errori come chiavi `err.file.*` (mai panico), ciò che il file non ha è assente.
- **Metodo e metadati** (richiesta di Federico): ogni lettore implementa `metodo(c)` = documento `metodo/1` (`docs/agenti/metodo.md`, schema `mzlab/web/schemi/metodo-1.json`) e `testata_mzml(c)` con `Header::run_params` che porta `("mzLab metodo/1", documento)` (come `rawlettore.rs::testata`), più strumento, seriale, software e versione, `startTimeStamp`, SHA-1 dei file. Sezioni: `strumento` (produttore, modello, seriale, firmware/software), `origine` (file, SHA-1, byte, `file_creato`), `campione` (nome, vial, volume, posizione in sequenza), `operatore` e computer marcati `identita` (mai copiati nell'mzML), `ms` (sorgente, polarità, esperimenti con parametri: energie di collisione, isolamento, risoluzione, transizioni Q1/Q3/CE/dwell, liste di inclusione, finestre DIA), `lc` (gradiente, flusso, forno), `rivelatori`, calibrazione/lock mass. Ciò che non decodifichi va in `campi`/`grezzo` o in `avvisi` (`meta.nonDecodificato`), e aggiungi la voce in `docs/agenti/grezzi-non-estratti.md` (regola «nessuna perdita»).
- Riferimenti liberi (Apache-2.0: `openwraw`, `openaraw`, `opensxraw`, `opentimstdf`, `openszraw` su docs.rs; ProteoWizard) solo per capire il formato: il codice si scrive da zero e si cita il riferimento nel commento del modulo. Mai decompilare il software del produttore.
- Dati veri: solo dal repository privato `Freeesta/mzlab-dati` (`MZLAB_DATI=<percorso>`); MAI nel repository pubblico; i test che li chiedono saltano senza (`eprintln!("SKIP ...")`). Nessun cognome di `nomi_vietati.txt` (repository privato) in codice, test, commit o PR. Se i file veri mancano: implementa e prova su file costruiti nel test, scrivi «Non verificato (nessun file vero)» nel file d'area, PR aperta, chiedi i file al Coordinatore.
- Prove obbligatorie: file costruito nel test (scritto da un codificatore nel test Rust), casi ostili in `crates/mzlab-lettori/tests/robustezza_<famiglia>.rs` o `crates/mzlab-core/tests/robustezza_*.rs` (banco `tests/robustezza/mod.rs`), parità con l'mzML di msconvert sui file veri, `cargo test -p mzlab-lettori --features <famiglia>` e `cargo test -p mzlab-core`, la costruzione del modulo con `tools/build_lettori_wasm.sh` (sotto il tetto), `python3 tools/verifica.py --cambiati` (in cloud: `docs/agenti/ambiente-cloud.md`), un e2e nel browser se tocchi la pagina (un front end non provato nel browser è rotto).
- Flusso: ramo `claude/<argomento>` da `origin/main`, PR, merge con merge commit quando la CI della PR è verde, poi controlla che il workflow «pages» del commit su `main` sia riuscito. Documenti = stato attuale: una riga nuova nel file d'area per ogni funzione nuova; aggiorna la riga del formato in `docs/studi/FORMATI_NATIVI_MS.md` (§1 e §2) da «A» a ciò che è vero ora.
- Se il contesto supera 200k token a fine PR: blocco di consegna al Coordinatore (AGENTS.md §2) e fermati.
