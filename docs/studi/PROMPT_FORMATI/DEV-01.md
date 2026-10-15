# DEV-01 — Thermo fuori dal motore: modulo `thermo` e lettore esterno

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-01 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Thermo fuori dal motore: modulo `thermo` e lettore esterno.
File d'area da leggere oltre a quelli comuni: docs/agenti/lettori-modulo.md, docs/agenti/lettura-nativa.md, docs/agenti/rust.md, docs/agenti/rust-desktop.md, docs/agenti/memoria.md, `vendor/opentfraw/MODIFICHE-MZLAB.md`.

## Obiettivo
Il lettore Thermo occupa circa 156 KB del `.wasm` del motore. Misura dello studio, senza `wasm-bindgen`: con `rawlettore::VOCE` in `LETTORI` il motore pesa 2 024 460 B, senza 1 868 164 B (`docs/studi/FORMATI_NATIVI_MS.md` §0). Va spostato nel modulo `thermo`, così si libera il margine per DEV-09, 10 e 11.

Il `.raw` deve restare letto **senza mzML intermedio** (`docs/agenti/lettura-nativa.md`): indice v2 con le posizioni, rotte, `native_srm` e «Scarica .mzML» come oggi. Per questo il motore riceve un **lettore esterno**: un `Lettore` che chiama in modo sincrono il modulo `thermo`, caricato nello stesso worker. Il SHA-256 del motore CAMBIA (eccezione delle regole comuni) e il motore deve scendere di almeno 120 000 B.

## Da fare
1. Prima della modifica, misura il motore come `pages.yml:53-61`: `cargo build`, `wasm-bindgen`, `tools/wasm_opt.sh`. Scrivi il valore.
2. `crates/mzlab-core`:
   - Feature `thermo`, accesa in `default`. Sotto `#[cfg(feature = "thermo")]` vanno `raw`, `rawlettore`, `metodo_raw::{script, ...}` e la dipendenza `opentfraw` (`optional = true`), con la riga `crate::rawlettore::VOCE` di `LETTORI` (`lettori.rs:221`) e `mzidx::build_raw` (`mzidx.rs:676`).
   - `metodo_raw::campi` resta sempre compilato: lo usano `agilent/doc.rs:8` e `agilent/quant.rs:9`. Se serve, spostalo in un modulo neutro (`metodo/campi.rs`) e lascia un `pub use`.
   - `crates/mzlab-wasm/Cargo.toml`: `mzlab-core = { path = ..., default-features = false }`.
   - Verifica: `cargo tree -e features -p mzlab-wasm` non mostra `thermo`; `cargo test -p mzlab-core` (con `default`) compila ancora `raw_parity.rs`, `raw_lettore.rs` e `raw_profile_parity.rs`; i test Rust lanciati dalla CI (`test.yml` e `tools/verifica.py`: leggili, non cambiare il workflow) compilano ancora i test Thermo.
3. Modulo `thermo` (`crates/mzlab-lettori`, feature `thermo`): registra `mzlab_core::rawlettore::VOCE`, con `mzlab-core` e la feature `thermo` accesa. Il modulo implementa l'interfaccia di sessione di DEV-00: `apri`, `indice`, `scansione`, `cromatogramma`, `intestazione`, `metodo`, `chiudi`.
4. `crates/mzlab-wasm/src/esterno.rs`: `LettoreEsterno`, che implementa `Lettore` sopra un oggetto JS di funzioni sincrone (`{indice, scansione, intestazione, cromatogrammi, cromatogramma, metodo, testata}`), come `JsMember` di `nativi.rs` chiama `read`.
   - Le risposte JSON si leggono con `serde_json`, già nel motore.
   - m/z e intensità arrivano come `Float64Array` e si copiano con `copy_to`.
   - `posizione` delle voci dell'indice viaggia nel JSON, perché l'indice v2 la scrive (`docs/agenti/mzidx.md`).
   - Ogni errore del modulo diventa `Error` con la stessa chiave.
   - `Nativi::open` (`nativi.rs:98`): quando `lettori::apri` dà `err.file.notAccepted` e la pagina ha registrato un lettore esterno per il dataset (`Engine.native_external(ds, obj)`, nuova), si apre `LettoreEsterno`. `native_samples`, `build_mzidx_native`, `native_srm` e `native_to_mzml` passano dallo stesso `reader`, senza un ramo apposta.
   - Misura il peso aggiunto: deve stare sotto 12 KB.
5. Pagina (worker):
   - `mzlab/web/rust-nativo.js`, prima di `attach` di un dataset `.raw`:
     - fa `import("./wasm-lettori/thermo/mzlab_lettori_wasm.js")` una sola volta per worker, con il controllo SHA-256 del manifesto;
     - apre la sessione sul modulo con le stesse funzioni `read` dei membri;
     - passa al motore l'oggetto di funzioni.
   - Fai lo stesso nei worker di `mzlab/web/idxpool.js`, che costruiscono gli indici in parallelo.
   - Il modulo si scarica solo al primo `.raw`: prova che un `.wiff` non lo chiede.
   - Errore di caricamento: `err.file.wiffLoad` con `famiglia: "thermo"` (chiave esistente, riusata da DEV-00).
   - `nativo.js::raws` e `shell.js::rawToMzml` restano uguali.
6. Desktop: `mzlab-desktop` usa `mzlab-core` con `default` (Thermo nel motore nativo): nessun lettore esterno. Verifica con la sua prova.
7. Memoria (`docs/agenti/memoria.md`): il modulo `thermo` ha un suo heap. Misura il picco su un `.raw` vero (indice + 20 scansioni) e scrivilo in `docs/agenti/misure.md`.
8. Prove:
   - `cargo test -p mzlab-core`, `-p mzlab-lettori --features thermo` e `-p mzlab-wasm`.
   - Un test Rust di `LettoreEsterno`: nessun crate usa `wasm-bindgen-test`, quindi separa la conversione delle risposte dalle chiamate JS e provala in nativo con risposte finte.
   - Gli e2e che aprono un `.raw`, da trovare con `grep -ln '\.raw' tests_e2e/`: oggi `e2e_rust`, `e2e_converti`, `e2e_mappa3d`, `e2e_inglese_caricamento`; quelli con file veri saltano senza `MZLAB_DATI`.
   - Un e2e nuovo `tests_e2e/e2e_thermo_modulo.py` con un `.raw` del repository privato (salta senza): TIC, una scansione e «Scarica .mzML» uguali a quelli di `main`, `window.MZLAB_LR_MISS` vuoto.
   - `python3 tools/verifica.py --cambiati` e la CI della PR con `sito-rust.yml`.
9. Documenti:
   - `docs/agenti/rust.md`: peso nuovo del motore e la regola «il margine liberato è per DEV-09/10/11 dello studio formati».
   - `docs/agenti/lettura-nativa.md`: il `.raw` passa dal lettore esterno.
   - `docs/agenti/lettori-modulo.md`: lettore esterno e interfaccia di sessione.
   - §1 di `FORMATI_NATIVI_MS.md`: riga Thermo.

## Atteso
- Il motore scende di almeno 120 000 B, misurati prima e dopo nel log di `pages`.
- Ogni `.raw` si apre come prima, senza mzML intermedio.
- Il modulo `thermo` sta sotto 458 752 B e si scarica solo con un `.raw`.
- Tutte le parità Thermo sono invariate.

## File reali
I `.raw` già nel repository privato (`HRMS/`, con i loro mzML). Nessun file nuovo.

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
