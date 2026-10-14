# DEV-08 — UV/PDA dei file dei produttori

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-08 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: UV/PDA dei file dei produttori.
File d'area da leggere oltre a quelli comuni: docs/agenti/lettori-modulo.md, docs/agenti/dati-3200.md (tracce analogiche), docs/agenti/grezzi-non-estratti.md.

## Obiettivo
Le tracce UV/PDA stanno nei file ma nessun lettore le dà: Thermo (controller `Pda`/`Uv` noti in `vendor/opentfraw/src/reader.rs:382-387`, non letti), SCIEX (`DAD*`, `DADRealTimeData` del `.wiff`, «non decodificato» in `metodo.md`), Agilent (`*.ch`, `*.uv`, `DAD1*.ch` nella cartella `.D`). Il modulo le scrive nell'mzML (regola R, motivo `"pda"`).

## Da fare
1. Controlla prima come il motore legge un mzML con UV: `crates/mzlab-core/src/mzml.rs:270-285` (`"TWC"` e `c.pda` → `ChromKind::Pda`) e cosa fa con uno spettro `MS:1000804` (electromagnetic radiation spectrum) senza livello MS. Se il motore lo scambierebbe per uno spettro di massa, il modulo scrive SOLO cromatogrammi (TWC e una traccia per lunghezza d'onda o per segnale), e lo spettro UV 3D resta un `campo`/avviso: scrivi la scelta in `docs/agenti/lettori-modulo.md`.
2. Thermo: lettura dei controller PDA/UV in `vendor/opentfraw` (aggiungi la modifica a `vendor/opentfraw/MODIFICHE-MZLAB.md`, licenza Apache-2.0), sotto `estesi` se tocca codice usato dal motore; `probe` → `"pda"` quando c'è un controller PDA/UV.
3. SCIEX: decodifica di `DAD*` del `.wiff` (OLE, `ole.rs`) sotto `estesi`; `probe` → `"pda"`.
4. Agilent: `*.ch` (segnali a una lunghezza d'onda, formato ChemStation 130/179/181) e `*.uv` (spettri 3D) della cartella `.D`, sotto `estesi` in `agilent/`; riferimento libero: il progetto `rainbow` (licenza da verificare) solo come guida.
5. `metodo/1`: `rivelatori` con lunghezze d'onda, larghezza di banda, riferimento, frequenza di campionamento, lampada.
6. Test: per ogni produttore un file costruito nel test dove si può (Agilent `*.ch`), parità con le tracce UV dell'mzML di msconvert sui file veri (`MZLAB_DATI`): stessi tempi, valori ≤ 1e-6 relativi; e2e: la traccia UV appare tra le tracce analogiche della vista Dati.

## Atteso
Le tracce UV/PDA dei file dei tre produttori appaiono; motore invariato; riga «UV/PDA» della matrice aggiornata.

## File reali
1 `.raw` con PDA, 1 `.wiff` con DAD, 1 `.D` LC con DAD (segnali `*.ch` e spettri `*.uv`); ognuno con l'mzML di msconvert che contiene le tracce UV.

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
