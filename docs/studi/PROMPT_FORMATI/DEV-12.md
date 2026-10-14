# DEV-12 — Tipi di scansione: precursor ion, neutral loss, EPI, MS3

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---».

---

Pacchetto DEV-12 dello studio `docs/studi/FORMATI_NATIVI_MS.md`: Tipi di scansione: precursor ion, neutral loss, EPI, MS3.
File d'area da leggere oltre a quelli comuni: docs/agenti/dati.md (informazioni del file e parti di un file misto), docs/agenti/lingue.md.

## Obiettivo
La vista Dati classifica un file in `iteminfo.rs::acq` (`crates/mzlab-core/src/iteminfo.rs:278-325`: MS1, SIM, DDA, DIA, AIF, PRM, MSn) ma non riconosce precursor ion scan, constant neutral loss/gain, EPI: oggi sono «MS2» o «full scan». Riconoscerli dal file (mai dal nome). Dipende da DEV-11 (tocca il motore) e da DEV-7 (gli mzML del modulo portano i termini).

## Da fare
1. `crates/mzlab-core/src/scantable.rs` (e `scantable/rows.rs`): leggi il termine del tipo di spettro dell'mzML (`MS:1000341` precursor ion spectrum, `MS:1000326` constant neutral loss spectrum, `MS:1000325` constant neutral gain spectrum, `MS:1000789` enhanced multiply charged spectrum, `MS:1000790` time-delayed fragmentation spectrum; controlla ogni accession sul `psi-ms.obo`) e il filtro Thermo (`pr`, `cnl`/`cng` se presenti) in un campo nuovo della scansione; il `.mzidx` porta il campo se serve (`docs/agenti/mzidx.md`: compatibilità con gli indici già scritti, versione solo se indispensabile).
2. `iteminfo.rs::acq`: voci `PREC`, `NL`, `NG`, `EPI` con priorità sopra MS2; `ms2_exps` e le parti di un file misto (`iteminfo.rs:58-142`) le separano.
3. Pagina: etichette nuove in `lang/it.js` e `lang/en.js` per le parti; l'elenco dei file mostra il tipo. Il pannello MS2 di una precursor ion scan mostra i PRECURSORI dello ione prodotto fisso: se il pannello assume il contrario, scrivi la domanda al Coordinatore prima di cambiarlo.
4. Il motore cresce: misura e scrivi il peso nuovo in `docs/agenti/rust.md` (deve restare sotto il tetto con il margine di DEV-11).
5. Test: mzML costruiti nel test per ogni tipo; parità `tools/info_riferimento.py` se la rotta `info` cambia (Python resta riferimento: aggiorna il riferimento solo per il comportamento nuovo, scritto prima in Rust); e2e: un mzML precursor ion sintetico mostra la parte giusta.

## Atteso
I quattro tipi riconosciuti; nessun file esistente cambia classificazione (golden invariati); riga «Precursor ion / neutral loss» della matrice aggiornata.

## File reali
1 mzML (o `.wiff`) per tipo dal QTRAP: precursor ion, neutral loss, EPI, MS3.

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
