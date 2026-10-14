# DEV-02 — Lettore mzXML

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-02 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Lettore mzXML.
File d'area da leggere oltre a quelli comuni: docs/agenti/lettori-modulo.md.

## Obiettivo
Leggere i file mzXML (ISB, versioni 2.x e 3.x) con il modulo lettori: spettri, livelli, precursori, polarità, cromatogrammi TIC/BPC calcolati, metadati.

## Da fare
1. `crates/mzlab-lettori/src/mzxml.rs` + `VOCE` (`id: "mzxml"`, estensioni `.mzxml`) in `LETTORI_EXTRA`. `riconosci`: `<mzXML` nei primi 4 KB (`Si`).
2. Indice: se c'è `<indexOffset>` in coda (leggi gli ultimi 4 KB), usa `<index name="scan"><offset id=...>`; altrimenti una passata a offset che trova ogni `<scan `. Le scansioni annidate (mzXML 2: MS2 dentro MS1) contano con `Limiti::profondita_ok`.
3. Per scansione: `num`, `msLevel`, `polarity` (+/−), `retentionTime` (durata ISO `PT…S`, in minuti), `peaksCount`, `filterLine`, `basePeakMz`, `totIonCurrent`, `centroided`, `lowMz/highMz`, `scanType` (Full, SIM, SRM, CRM, Q1, Q3, MRM), `collisionEnergy`; `<precursorMz precursorCharge= precursorIntensity= activationMethod= windowWideness=>`; `<peaks precision="32|64" byteOrder="network" contentType="m/z-int" compressionType="zlib|none" compressedLen=>` (base64, big-endian, coppie interleaved). Usa la base64 e lo zlib che `mzlab-core` già usa (`base64`, `flate2`), con il limite di gonfiamento di `mzml.rs::decode_array`.
4. `scanType="SRM"/"MRM"`: avvolgi con `SrmDaScansioni` (DEV-0) per avere i cromatogrammi SRM.
5. `metodo(c)` e `testata_mzml`: `msInstrument` (`msManufacturer`, `msModel`, `msIonisation`, `msMassAnalyzer`, `msDetector`, `software`), `parentFile` (`fileName`, `fileType`, `fileSha1`), `dataProcessing` (`centroided`, software), `startTime`/`endTime`, `operator` (`identita`). Ciò che resta va in `campi`.
6. Test: mzXML costruiti nel test (32 e 64 bit, zlib e no, con e senza indice, annidato e piatto, SRM); casi ostili (offset fuori file, `peaksCount` enorme, annidamento profondo, base64 rotta); parità con l'mzML di msconvert dello stesso file (`MZLAB_DATI`): m/z ≤ 1e-6, intensità relative ≤ 1e-6, RT, livello, precursore, polarità. E2e: un mzXML sintetico apre nella vista Dati con TIC e spettro.

## Atteso
`.mzxml` apre; riga «mzXML» di `FORMATI_NATIVI_MS.md` aggiornata.

## File reali
2 mzXML: un full scan LR e un DDA HR, ciascuno con l'mzML di msconvert dello stesso file grezzo.

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
