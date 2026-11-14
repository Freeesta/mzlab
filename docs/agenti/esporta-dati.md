# Esporta tutti i miei dati (inventario di ciò che l'app salva nel browser + scrittore ZIP)

Tutto il lavoro dell'utente sta nel SUO browser; l'esportazione lo raccoglie in un solo `mzlab-dati-<data>.zip` scaricato in locale (nulla esce dal computer). Nomi di storage: `docs/agenti/dati.md` («Inventario chiavi»), `mzlab/web/nomi.js`.

## Scrittore ZIP (Rust)
- `crates/mzlab-core/src/zip.rs`: `zip_stored(&[(nome, byte)])` scrive un ZIP «stored» (nessuna compressione, nessuna dipendenza, niente ZIP64: al più 65535 voci e 4 GiB), nomi UTF-8 (bit 11), data DOS fissa 1980-01-01 (archivio deterministico), CRC-32 IEEE. `zip_packed(packed)` lo costruisce da un buffer unico: per voce `u16` lunghezza nome, `u32` lunghezza dati (little endian), nome, dati.
- Nel motore (`crates/mzlab-wasm/src/zip.rs`): `zip(packed) -> Vec<u8>` (una sola lettura; il worker passa il buffer restituito, una copia nuova della lunghezza esatta, senza ricopiarlo).
- Lettura: `unzip_stored(zip)` rilegge un ZIP «stored» (CRC controllato, nomi non sicuri e voci che condividono byte rifiutati: vuoti, `/` iniziale, `\`, NUL, `..`, due voci sugli stessi byte (`zip: entries overlap`: un archivio «stored» tiene ogni byte una volta, quindi l'espansione non supera il file); vale anche per archivi di altri scrittori, ma non legge voci compresse né ZIP64); `unzip_packed` lo restituisce nel buffer di `unpack`. Nel motore: `unzip(zip) -> Vec<u8>`. Prove nei test Rust `zip/tests.rs` (rilegge anche `py.zip` scritto da Python).
- Prove: `cargo test -p mzlab-core zip::` (CRC noto, archivio vuoto, impronta del file) e `tests/test_zip_stored.py` (il test Rust con `MZLAB_ZIP_OUT=<cartella>` scrive 0 file, un file vuoto, 4 file con 70 000 byte e nome non ASCII; Python li rilegge con `zipfile` byte per byte e con `unzip -t`).

## Pulsante e raccolta
- **Pulsante**: nessuno. La sezione «Dati e privacy» delle Impostazioni è stata tolta dall'interfaccia: `esporta-dati.js`, `importa-dati.js` e il worker restano come codice senza entrata (nessun altro file li carica) e senza prova e2e; le chiavi `export.*` e `import.*` rimaste in `lang/*/settings.js` sono quelle che usano i moduli.
- `mzlab/web/esporta-dati.js`: `collect()` (localStorage, IndexedDB `mzlab_lab` negli store `notebook`, `kv`, `liste_utente`, `molecole` come JSON con i binari in base64, OPFS `librerie/`), `manifest()`, `pack()`, `run()`, `start()`. `esporta-dati-worker.js` chiama `zip`/`unzip` del motore. Il file è `mzlab-dati-<AAAA-MM-GG>.zip` con `manifesto.json` (formato `mzlab-dati/1`, versione, data, `files` con byte e significato, `excluded` con motivo), `localStorage.json`, `indexeddb/<store>.json`, `opfs/librerie/…`.

## Importa i miei dati
- `importa-dati.js` (`begin(d, file)`) legge uno ZIP di questo formato. Il worker `esporta-dati-worker.js` (op `unzip`) fa leggere lo ZIP a Rust; `plan()` accetta solo `manifesto.json` con formato `mzlab-dati/1` e tiene una chiave di localStorage solo se è di mzLab e non è segreta, di esecuzione o di sviluppo (stesse regole dell'esportazione: un file manomesso non può scrivere altro).
- Anteprima `#uip-import-box` (`role=group`): numero di voci e quante differiscono, record per archivio IndexedDB, file di libreria; il focus va su «Ripristina» (`#uip-import-go`), «Annulla» non scrive nulla. Solo alla conferma `apply()` scrive localStorage, IndexedDB (tramite `lab()`) e le librerie in OPFS (op `opfs`; i file in uso sono contati in `import.partial`), poi la pagina si ricarica. Un file non valido dà `import.bad` senza anteprima. I segreti locali non vengono toccati.

## Regole di raccolta
- **Si prende** tutto ciò che crea l'utente: preferenze e viste (localStorage `mzlab.*`, `mzlab-theme`, `mzlab-labels`; i vecchi `qqq.*` solo se non c'è il corrispondente `mzlab.*`), taccuino, liste utente, molecole, librerie spettrali, sessione. I file mzML/raw e gli indici (`idx/`, `tiles/`) NON si includono se non sono già salvati dall'app come dato dell'utente: sono ricostruibili dai file originali.
- **Esclusi, elencati nel manifesto con il motivo**: segreti (ogni chiave il cui nome contiene `token`, `key` o `secret`; la chiave dell'assistente non è mai salvata, vive solo nell'iframe `llm/trasporto.html`); stato di esecuzione (`mzlab.run`, `mzlab.avvii`, `mzlab.migrato`, `mzlab.memProbe`, `mzlab.memBudget*`, `mzlab.memLevel`, `mzlab.kprefetch`, sessionStorage); cache dell'app (Cache API `mzlab-app-*`, `mzlab-big-*`, service worker); flag di sviluppo e amministratore (`mzlab.admin`, `mzlab.dev.*`).

## Inventario
| Dove | Chiave / nome | Significato | Formato |
|---|---|---|---|
| localStorage | `mzlab.prefs` | preferenze generali | JSON oggetto |
| localStorage | `mzlab.lang`, `mzlab-theme`, `mzlab-labels` | lingua, tema, etichette dei grafici | stringa |
| localStorage | `mzlab.a11y` | aiuti di lettura e impostazioni di accessibilità | JSON |
| localStorage | `mzlab.sessione` | file e pannelli aperti (sessione) | JSON |
| localStorage | `mzlab.picchi`, `mzlab.calc`, `mzlab.mappa`, `mzlab.ordine`, `mzlab.pca`, `mzlab.pfas`, `mzlab.profilo`, `mzlab.fair`, `mzlab.flgrp`, `mzlab.tunit`, `mzlab.kmd`, `mzlab.qaqc`, `mzlab.riferimento`, `mzlab.bianco`, `mzlab.comp`, `mzlab.infg.frag` | scelte delle viste di Dati | JSON o stringa |
| localStorage | `mzlab.barra.lato`, `mzlab.barra.lr`, `mzlab.barra.hr`, `mzlab.barra.hint`, `mzlab.banco.info` | layout della barra laterale e avvisi già visti | stringa / JSON |
| localStorage | `mzlab.lib.soglie`, `mzlab.lib.cartella`, `mzlab.liste.attive`, `mzlab.contaminanti`, `.esclusi`, `.on` | librerie, liste e contaminanti scelti | JSON |
| localStorage | `mzlab.disegno.font`, `.riquadri`, `.strumenti`, `mzlab.pal.recent` | pagina Disegno: font, riquadri, strumenti, colori recenti | JSON |
| localStorage | `mzlab.tour.lr`, `mzlab.upd.last`, `mzlab.upd.seen`, `mzlab.rel.last`, `mzlab.rel.seen`, `mzlab.reset.note`, `mzlab.logo.gold` | tour, avvisi di aggiornamento già visti | stringa / JSON |
| localStorage | `mzlab.llm`, `mzlab.llm.consenso` | assistente: impostazioni e consensi per fornitore (senza chiavi) | JSON |
| localStorage | `mzlab.attributio.reviewer`, `mzlab.tabui.*`, `mzlab.pipeline.draft` | admin: revisore e decisioni (non esportati dall'interfaccia degli studenti) | JSON |
| IndexedDB `mzlab_lab` | `notebook`, `kv` | taccuino; coppie chiave-valore | oggetti |
| IndexedDB `mzlab_lab` | `liste_utente`, `molecole`, `manifest` | liste dell'utente, archivio molecole, elenco dei file in OPFS | oggetti |
| IndexedDB `mzlab_lab` | `files` | file salvati dal vecchio motore (solo ripiego) | blob |
| IndexedDB `mzlab-libs`, `mzlab-libdir` | `lib`, `h` | librerie spettrali e cartella puntata senza OPFS (ripiego) | blob / handle |
| OPFS | `librerie/` | librerie spettrali salvate (`<id>.mzlib`) | binario |
| OPFS | `mzlab/` (radice del motore), `idx/`, `tiles/`, `conv/`, `export/` | file importati, indici, mappe e conversioni: ricostruibili | binario |
| Cache API | `mzlab-app-*`, `mzlab-big-*`, `mzlab-libs` | copia offline dell'app | risposte HTTP |
| sessionStorage | `mzlab.run`, `mzlab.telefono*`, `mzlab.unopen`, `mzlab.convopen`, `mzlab.tour.step` | stato di questa esecuzione | stringa |

Nota: le chiavi e le cartelle si leggono per NOME dal browser (come `reset.js`, nessuna lista fissa): la tabella spiega le famiglie, la raccolta prende tutto ciò che inizia per `mzlab`/`qqq` meno gli esclusi.

## Esportazioni a pezzi (`export-pezzi.js`, `export-worker.js`)
- Tutte le esportazioni di testo passano da `dl(name, text, type, prov)` (`explore.js`); `text` è una stringa, un `Blob`/`Uint8Array` o un iterabile (anche asincrono) di pezzi (stringhe, `Uint8Array`, `Blob`).
- Una stringa o un `Blob` sotto 8 MB e un iterabile che finisce sotto 8 MB si salvano con `saveBlob` come un `Blob` intero; oltre, `EXPORT_PEZZI.save` (`export-pezzi.js`, modulo ES caricato con `carica()` al primo export grande o a iterabile, fuori dall'elenco d'avvio) scrive i pezzi con `export-worker.js` in OPFS `mzlab/export/<id>` (`createSyncAccessHandle`, pezzi di circa 1 MB, un messaggio di conferma per pezzo) e scarica il `File` letto da lì. Stessi byte: le stringhe sono UTF-8 e una coppia surrogata mai spezzata fra due messaggi.
- Senza OPFS o `Worker` i pezzi si raccolgono in memoria e si salva un `Blob` (`save` torna `false`). Errore di scrittura (spazio, archivio): avviso `export.stream.fail`, nessun file.
- Provenienza (`provenienza/1`): CSV/TSV portano la riga `# mzLab …` come prima; su un iterabile la riga si antepone al primo pezzo, identica; `prov === false` non la scrive.
- Pulizia: il file in `export/` si toglie 10 minuti dopo l'avvio del download (`drop`); al primo caricamento del modulo `clean` toglie i file più vecchi di 1 ora. L'URL del download resta valido per gli stessi 10 minuti.
- Chi produce molto testo passa un generatore: MGF/MSP di tutti gli MS2 (`piecesMgf`, `piecesMsp` in `workflow.js`, stessi byte di `formatMgf`/`formatMsp`).
- Prova: `tests_e2e/e2e_esporta_pezzi.py` (byte per byte su sorgenti piccole, 9 MB con coppia surrogata tagliata, 210 MB in pezzi con SHA-256).
- mzFinder (`TP_Mine/js/`): le esportazioni passano dallo stesso `dl()` (`window.dl`): `G.dl` in `28_`, `30_`, `38_`, `42_` e l'aiuto `dl` di `19_tpmine_results.js` (usato anche da `21_tpmine_hr.js`, con `prov === false`: nessuna riga di provenienza). Una stringa grande (CSV completo, MGF, SIRIUS) oltre 8 MB va in OPFS; lo ZIP del rapporto (`zipStore`) e le stringhe del motore restano in memoria finché il motore non le consegna a pezzi.
