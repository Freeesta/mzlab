# DEV-13 — Mobilità ionica nell'mzML

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-13 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Mobilità ionica nell'mzML.
File d'area da leggere oltre a quelli comuni: docs/agenti/mzidx.md, docs/agenti/dati.md, docs/agenti/alta-risoluzione.md.

## Obiettivo
`mzml.rs` non legge gli array di mobilità ionica. Primo passo minimo: leggerli e mostrarli dove non serve un asse nuovo. Dipende da DEV-11 (motore) e da DEV-10 (gli mzML dei TDF).

## Da fare
1. `crates/mzlab-core/src/mzml.rs` (`decode_array`, riconoscimento degli array, righe ~405-415): array di mobilità (`MS:1002476` ion mobility drift time array, `MS:1002815` inverse reduced ion mobility array, e quelli per scansione `MS:1002476`/`MS:1002815` come cvParam della scansione; controlla gli accession sul `psi-ms.obo`). Una scansione con mobilità per scansione (un valore) o per punto (array).
2. Informazioni del file: acquisizione `IMS` in `iteminfo.rs::acq` e intervallo di mobilità nella risposta `info`.
3. `/api/scan` (`hrscans.rs`, una scansione come il file la ha): colonna della mobilità se c'è. La mappa e l'XIC filtrato per mobilità NON fanno parte di questo pacchetto: scrivi una proposta al Coordinatore (formato `.mzidx` con la colonna, vista RT × 1/K0).
4. Test: mzML costruito nel test con le due forme; casi ostili (array di lunghezza diversa dai punti); peso del motore misurato.

## Atteso
Un mzML con mobilità apre, dice «IMS» e mostra la mobilità nella scansione; nessun file senza mobilità cambia; righe IMS della matrice aggiornate.

## File reali
1 mzML con mobilità (msconvert di un TDF senza `--combineIonMobilitySpectra`, o di un Waters HDMSe).

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
