# DEV-11 — Dieta del motore (spazio per DEV-12, 13, 14)

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-11 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Dieta del motore (spazio per DEV-12, 13, 14).
File d'area da leggere oltre a quelli comuni: docs/agenti/rust.md, docs/agenti/memoria.md, docs/agenti/misure.md.

## Obiettivo
Il `.wasm` del motore pesa 1 703 607 B su 1 703 936 B (329 B di margine; log del job `build` di `pages.yml`). DEV-12, 13 e 14 devono toccare il motore. Liberare almeno 16 384 B senza cambiare alcun comportamento. Il tetto non si tocca.

## Da fare
1. Misura: costruisci il motore come `pages.yml:53-61` (`cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`, `wasm-bindgen`, `tools/wasm_opt.sh`); `cargo install twiggy` e `twiggy top -n 60` / `twiggy dominators` sul `.wasm` prima di `wasm-opt` per vedere chi pesa. Scrivi le prime 30 voci in `docs/agenti/misure.md` (sezione nuova, solo l'ultima misura).
2. Candidati da verificare (non presumere: misura ciascuno): codice di `vendor/opentfraw/src/mzml.rs` (1 180 righe; il motore usa solo `extract_spectrum`, `scan_metadata`, `run_terms`: `rawlettore.rs:8`, `:366`) e di `raw::to_mzml` se nessuna esportazione wasm lo chiama più (`crates/mzlab-wasm/src/nativi.rs:296` usa `lettori_mzml::scrivi`); formattazione con `format!` ripetuta nelle rotte JSON (`json.rs` ha già `Out`); tabelle di stringhe duplicate; rami usati solo da Python o dai test (spostali sotto `#[cfg(test)]` o in `tools/`, mai togliere una rotta che la pagina chiama: `grep -n` in `mzlab/web/`).
3. Ogni riduzione = un commit; dopo ognuna `cargo test --release -p mzlab-core -p mzlab-wasm`, le parità d'oro (`crates/mzlab-core/tests/golden.rs`, `python3 tools/verifica.py --senza-e2e`), poi alla fine `python3 tools/verifica.py --cambiati` e la CI della PR con `sito-rust.yml` (e2e del motore).
4. Scrivi in `docs/agenti/rust.md` il peso nuovo e la regola «il margine liberato è per DEV-12/13/14 dello studio formati».

## Atteso
Motore ≤ 1 687 552 B (almeno 16 KB liberi), tutti i test e le parità invariati, nessuna rotta tolta.

## File reali
Nessuno.

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`). Coordinatore 9 = `session_018GRRMEJ9JUr7Utr7ujQiXs`: a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: PR, CI, pages, cosa resta). Se sei bloccato o devi scegliere tra due comportamenti: PR aperta con la spiegazione e domanda al Coordinatore, niente merge.
- AGENTS.md è nel tuo contesto: rispettalo. Leggi SOLO: la sezione 4 di `docs/studi/FORMATI_NATIVI_MS.md` (`grep -n '^## 4' docs/studi/FORMATI_NATIVI_MS.md`, poi lettura a intervalli), `docs/agenti/estendere.md`, `docs/agenti/lettura-nativa.md`, `docs/agenti/metodo.md` e i file d'area elencati sotto. Per trovare codice: `python3 tools/indice.py` e `grep -n`, oppure il subagente `cercatore`.
- Non toccare: `.github/workflows/`, `mzlab/web/teoria/`, il tetto del motore (1 703 936 B in `pages.yml`) e i tetti del modulo lettori (`tools/build_lettori_wasm.sh`, `tests/test_lettori_wasm.py`: 1 048 576 B `lettori`, 655 360 B `lettori-tdf`). **Il `.wasm` del motore non deve cambiare**: ha 329 byte di margine. Prova: `cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm` prima e dopo, stesso SHA-256 di `target/wasm32-unknown-unknown/release/mzlab_wasm.wasm` (eccezione: solo il pacchetto DEV-11 e quelli che dipendono da lui).
- Dove va il codice: lettori nuovi in `crates/mzlab-lettori/src/` (registro `LETTORI_EXTRA`); varianti dei lettori che stanno già in `mzlab-core` dietro `#[cfg(feature = "estesi")]` (la feature è spenta per `mzlab-wasm`, accesa da `mzlab-lettori`). Rust puro, nessun crate nuovo salvo quelli che il prompt nomina. Codice e commenti in inglese; ogni testo visibile è una chiave in `mzlab/web/lang/it.js` e `en.js` (`tools/controlla_i18n.py`).
- Contratto `Lettore` (`crates/mzlab-core/src/lettori.rs`): `riconosci` guarda i byte (l'estensione da sola non basta), mai `read_to_end` oltre 8 MB, ogni numero letto dal file passa da `Limiti::voci_ok`/`profondita_ok` prima di allocare o ricorrere, errori come chiavi `err.file.*` (mai panico), ciò che il file non ha è assente.
- **Metodo e metadati** (richiesta di Federico): ogni lettore implementa `metodo(c)` = documento `metodo/1` (`docs/agenti/metodo.md`, schema `mzlab/web/schemi/metodo-1.json`) e `testata_mzml(c)` con `Header::run_params` che porta `("mzLab metodo/1", documento)` (come `rawlettore.rs::testata`), più strumento, seriale, software e versione, `startTimeStamp`, SHA-1 dei file. Sezioni: `strumento` (produttore, modello, seriale, firmware/software), `origine` (file, SHA-1, byte, `file_creato`), `campione` (nome, vial, volume, posizione in sequenza), `operatore` e computer marcati `identita` (mai copiati nell'mzML), `ms` (sorgente, polarità, esperimenti con parametri: energie di collisione, isolamento, risoluzione, transizioni Q1/Q3/CE/dwell, liste di inclusione, finestre DIA), `lc` (gradiente, flusso, forno), `rivelatori`, calibrazione/lock mass. Ciò che non decodifichi va in `campi`/`grezzo` o in `avvisi` (`meta.nonDecodificato`), e aggiungi la voce in `docs/agenti/grezzi-non-estratti.md` (regola «nessuna perdita»).
- Riferimenti liberi (Apache-2.0: `openwraw`, `openaraw`, `opensxraw`, `opentimstdf`, `openszraw` su docs.rs; ProteoWizard) solo per capire il formato: il codice si scrive da zero e si cita il riferimento nel commento del modulo. Mai decompilare il software del produttore.
- Dati veri: solo dal repository privato `Freeesta/mzlab-dati` (`MZLAB_DATI=<percorso>`); MAI nel repository pubblico; i test che li chiedono saltano senza (`eprintln!("SKIP ...")`). Nessun cognome di `nomi_vietati.txt` (repository privato) in codice, test, commit o PR. Se i file veri mancano: implementa e prova su file costruiti nel test, scrivi «Non verificato (nessun file vero)» nel file d'area, PR aperta, chiedi i file al Coordinatore.
- Prove obbligatorie: file costruito nel test (scritto da un codificatore nel test Rust), casi ostili in `crates/mzlab-lettori/tests/robustezza.rs` o `crates/mzlab-core/tests/robustezza_*.rs` (banco `tests/robustezza/mod.rs`), parità con l'mzML di msconvert sui file veri, `cargo test -p mzlab-lettori -p mzlab-core`, `python3 tools/verifica.py --cambiati` (in cloud: `docs/agenti/ambiente-cloud.md`), un e2e nel browser se tocchi la pagina (un front end non provato nel browser è rotto).
- Flusso: ramo `claude/<argomento>` da `origin/main`, PR, merge con merge commit quando la CI della PR è verde, poi controlla che il workflow «pages» del commit su `main` sia riuscito. Documenti = stato attuale: una riga nuova nel file d'area per ogni funzione nuova; aggiorna la riga del formato in `docs/studi/FORMATI_NATIVI_MS.md` (§1 e §2) da «A» a ciò che è vero ora.
- Se il contesto supera 200k token a fine PR: blocco di consegna al Coordinatore (AGENTS.md §2) e fermati.
