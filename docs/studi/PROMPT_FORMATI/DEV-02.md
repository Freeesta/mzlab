# DEV-02 — Thermo `.raw`: SRM dei TSQ, lock mass, PDA/UV (modulo `thermo`)

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-02 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Thermo `.raw`: SRM dei TSQ, lock mass, PDA/UV (modulo `thermo`).
File d'area da leggere oltre a quelli comuni: docs/agenti/lettori-modulo.md, docs/agenti/metodo.md (sezione `.raw`), docs/agenti/dati-3200.md (tracce analogiche), `vendor/opentfraw/MODIFICHE-MZLAB.md`.

## Obiettivo
Dopo DEV-01 il lettore Thermo vive solo nel modulo `thermo` (`crates/mzlab-lettori`, feature `thermo`), letto dal motore attraverso il lettore esterno. Restano tre lacune del lettore:
- un `.raw` di un TSQ (Quantum, Endura, Quantiva, Altis, Quantis) in SRM non ha tracce SRM: `LettoreRaw::cromatogrammi` dà solo il TIC (`crates/mzlab-core/src/rawlettore.rs:222-248`), quindi il pannello MRM e mzFinder MRM (`native_srm`) non vedono nulla;
- la lock mass non è nel metodo;
- le tracce PDA/UV non si leggono (controller `Pda`/`Uv` noti in `vendor/opentfraw/src/reader.rs:382-387`).

Tutto il codice è nel modulo o sotto la feature `thermo` di `mzlab-core`, che il motore non accende più: il SHA-256 del motore resta quello di DEV-01.

## Da fare
1. SRM:
   - avvolgi il lettore `thermo-raw` con `SrmDaScansioni` (DEV-00) quando le scansioni hanno `ScanType::Srm`/`Crm` (`vendor/opentfraw/src/types.rs:49-58`, `scan_event.rs:70`);
   - Q1 = precursore della scansione, Q3 = centro di ogni intervallo di prodotto. Verifica sul file vero se i punti sono centroidi o intervalli e scrivi la regola in `docs/agenti/lettori-modulo.md`;
   - CE dal filtro (`@cid20.00`) o dall'evento;
   - id come msconvert (`SRM SIC Q1=… Q3=…`) e polarità per transizione dal filtro.
2. `metodo/1` del TSQ (`crates/mzlab-core/src/metodo_raw/script.rs`): la tabella delle transizioni del testo del metodo (composto, Q1, Q3, CE, RF lens/tube lens, dwell/tempo di ciclo, finestra di RT) va in `ms.periodi[].esperimenti[].transizioni`. Se il testo non ha una tabella riconoscibile: avviso `meta.nonDecodificato` con `dove`.
3. Lock mass (Orbitrap):
   - i campi del trailer `opentfraw.lock_mass_correction_ppm` e `number_of_lock_masses` (`vendor/opentfraw/src/extra.rs:122-125`) e la lista delle masse di riferimento del metodo vanno in `metodo/1` (`ms.calibrazione`: masse, valore costante o intervallo della correzione);
   - nessuna correzione dei dati.
4. PDA/UV:
   - Controlla prima come il motore legge un mzML con UV: `crates/mzlab-core/src/mzml.rs:270-285` (`"TWC"` e `c.pda` → `ChromKind::Pda`) e cosa fa con uno spettro `MS:1000804` senza livello MS.
   - Se lo scambierebbe per uno spettro di massa, il lettore dà SOLO cromatogrammi (TWC e una traccia per lunghezza d'onda o per segnale); lo spettro UV 3D resta un `campo`/avviso. Scrivi la scelta in `docs/agenti/lettori-modulo.md`: vale per tutte le famiglie.
   - Lettura dei controller PDA/UV in `vendor/opentfraw`: aggiungi la modifica a `MODIFICHE-MZLAB.md` (Apache-2.0).
   - `metodo/1`: `rivelatori` con lunghezze d'onda, larghezza di banda, riferimento, frequenza di campionamento, lampada.
5. Test:
   - `SrmDaScansioni` con un lettore finto che porta filtri TSQ veri (copiati come stringhe);
   - parità sul file vero contro i cromatogrammi SRM e UV dell'mzML di msconvert (`MZLAB_DATI`, salta senza): stesso numero di transizioni, Q1/Q3 ≤ 1e-3, tempi e intensità ≤ 1e-6 relative;
   - e2e solo con un file vero (salta senza);
   - il modulo resta sotto 458 752 B.

## Atteso
- Un `.raw` TSQ apre con il pannello MRM pieno e `native_srm` riceve le tracce.
- Le tracce PDA appaiono tra le tracce analogiche.
- La riga «Thermo» della matrice è aggiornata.

## File reali
- 1 `.raw` di un TSQ in SRM (meglio 2: positivo e negativo);
- 1 `.raw` Orbitrap acquisito con lock mass;
- 1 `.raw` con PDA.

Ognuno con il suo mzML di msconvert (con le tracce UV).

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
