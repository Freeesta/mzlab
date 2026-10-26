# Esporta tutti i miei dati (inventario di ciò che l'app salva nel browser + scrittore ZIP)

Tutto il lavoro dell'utente sta nel SUO browser; l'esportazione lo raccoglie in un solo `mzlab-dati-<data>.zip` scaricato in locale (nulla esce dal computer). Nomi di storage: `docs/agenti/dati.md` («Inventario chiavi»), `mzlab/web/nomi.js`.

## Scrittore ZIP (Rust)
- `crates/mzlab-core/src/zip.rs`: `zip_stored(&[(nome, byte)])` scrive un ZIP «stored» (nessuna compressione, nessuna dipendenza, niente ZIP64: al più 65535 voci e 4 GiB), nomi UTF-8 (bit 11), data DOS fissa 1980-01-01 (archivio deterministico), CRC-32 IEEE. `zip_packed(packed)` lo costruisce da un buffer unico: per voce `u16` lunghezza nome, `u32` lunghezza dati (little endian), nome, dati.
- Nel motore (`crates/mzlab-wasm/src/zip.rs`): `zip_len(packed) -> u32` e `zip_write(packed, out) -> u32` (la pagina chiede la misura, prepara `out`, poi scrive: nessuna funzione torna un `Vec<u8>`).
- Prove: `cargo test -p mzlab-core zip::` (CRC noto, archivio vuoto, impronta del file) e `tests/test_zip_stored.py` (il test Rust con `MZLAB_ZIP_OUT=<cartella>` scrive 0 file, un file vuoto, 4 file con 70 000 byte e nome non ASCII; Python li rilegge con `zipfile` byte per byte e con `unzip -t`).

## Pulsante e raccolta
- **Pulsante**: Impostazioni, gruppo «Dati e privacy» (`uip-g-priv`, chiuso di base): «Esporta tutti i miei dati» / «Export all my data» (`#uip-export`, chiavi `export.*` in `lang/*/settings.js`), con riga di stato `#uip-export-st` (`role="status"`, `aria-live="polite"`). `settings.js` carica `esporta-dati.js` al primo clic.
- `mzlab/web/esporta-dati.js`: `collect()` (localStorage, IndexedDB `mzlab_lab` negli store `notebook`, `kv`, `liste_utente`, `molecole` come JSON con i binari in base64, OPFS `librerie/`), `manifest()`, `pack()`, `run()`, `start()`. `esporta-dati-worker.js` chiama `zip_len`/`zip_write` del motore. Il file è `mzlab-dati-<AAAA-MM-GG>.zip` con `manifesto.json` (formato `mzlab-dati/1`, versione, data, `files` con byte e significato, `excluded` con motivo), `localStorage.json`, `indexeddb/<store>.json`, `opfs/librerie/…`.
- Prova: `tests_e2e/e2e_esporta_dati.py` (giro `--fumo` su Chromium Linux): impostazioni e due segreti piantati, pulsante dalla tastiera, ZIP letto con `zipfile`, manifesto coerente, nessun segreto nei file, axe nel pannello, bersaglio ≥ 24 px.

## Regole di raccolta
- **Si prende** tutto ciò che crea l'utente: preferenze e viste (localStorage `mzlab.*`, `mzlab-theme`, `mzlab-labels`; i vecchi `qqq.*` solo se non c'è il corrispondente `mzlab.*`), taccuino, liste utente, molecole, librerie spettrali, sessione. I file mzML/raw e gli indici (`idx/`, `tiles/`) NON si includono se non sono già salvati dall'app come dato dell'utente: sono ricostruibili dai file originali.
- **Esclusi, elencati nel manifesto con il motivo**: segreti (token sigillati del Libro `mzlab.libro.token`, `mzlab.libro.teoria.token`; ogni chiave il cui nome contiene `token`, `key` o `secret`; la chiave dell'assistente non è mai salvata, vive solo nell'iframe `llm/trasporto.html`); stato di esecuzione (`mzlab.run`, `mzlab.avvii`, `mzlab.migrato`, `mzlab.memProbe`, `mzlab.memBudget*`, `mzlab.memLevel`, `mzlab.kprefetch`, sessionStorage); cache dell'app (Cache API `mzlab-app-*`, `mzlab-big-*`, service worker); flag di sviluppo e amministratore (`mzlab.admin`, `mzlab.dev.*`).

## Inventario
| Dove | Chiave / nome | Significato | Formato |
|---|---|---|---|
| localStorage | `mzlab.prefs` | preferenze generali | JSON oggetto |
| localStorage | `mzlab.lang`, `mzlab-theme`, `mzlab-labels` | lingua, tema, etichette dei grafici | stringa |
| localStorage | `mzlab.a11y`, `mzlab.set.a11y` | aiuti di lettura e impostazioni di accessibilità | JSON |
| localStorage | `mzlab.sessione` | file e pannelli aperti (sessione) | JSON |
| localStorage | `mzlab.picchi`, `mzlab.mappa`, `mzlab.ordine`, `mzlab.pca`, `mzlab.pfas`, `mzlab.profilo`, `mzlab.fair`, `mzlab.flgrp`, `mzlab.tunit`, `mzlab.kmd`, `mzlab.qaqc`, `mzlab.riferimento`, `mzlab.gradiente.ultimo`, `mzlab.bianco`, `mzlab.comp`, `mzlab.infg.frag` | scelte delle viste di Dati | JSON o stringa |
| localStorage | `mzlab.barra.lato`, `mzlab.barra.lr`, `mzlab.barra.hr`, `mzlab.barra.hint`, `mzlab.banco.info` | layout della barra laterale e avvisi già visti | stringa / JSON |
| localStorage | `mzlab.lib.soglie`, `mzlab.lib.cartella`, `mzlab.liste.attive`, `mzlab.contaminanti`, `.esclusi`, `.on` | librerie, liste e contaminanti scelti | JSON |
| localStorage | `mzlab.disegno.font`, `.riquadri`, `.strumenti`, `mzlab.pal.recent` | pagina Disegno: font, riquadri, strumenti, colori recenti | JSON |
| localStorage | `mzlab.tour.lr`, `mzlab.upd.last`, `mzlab.upd.seen`, `mzlab.rel.last`, `mzlab.rel.seen`, `mzlab.reset.note`, `mzlab.logo.gold`, `mzlab.teoria.esteso`, `mzlab.teoria.itnote`, `mzlab.pratica` | tour, avvisi di aggiornamento già visti, Teoria e Pratica | stringa / JSON |
| localStorage | `mzlab.llm`, `mzlab.llm.consenso` | assistente: impostazioni e consensi per fornitore (senza chiavi) | JSON |
| localStorage | `mzlab.libro.teoria.draft.<capitolo>` | bozza non inviata di un capitolo (mai il token) | JSON |
| localStorage | `mzlab.attributio.reviewer`, `mzlab.tabui.*`, `mzlab.pipeline.draft` | admin: revisore e decisioni (non esportati dall'interfaccia degli studenti) | JSON |
| IndexedDB `mzlab_lab` | `notebook`, `kv` | taccuino; coppie chiave-valore | oggetti |
| IndexedDB `mzlab_lab` | `liste_utente`, `molecole`, `manifest` | liste dell'utente, archivio molecole, elenco dei file in OPFS | oggetti |
| IndexedDB `mzlab_lab` | `files` | file salvati dal vecchio motore (solo ripiego) | blob |
| IndexedDB `mzlab-libs`, `mzlab-libdir` | `lib`, `h` | librerie spettrali e cartella puntata senza OPFS (ripiego) | blob / handle |
| OPFS | `librerie/` | librerie spettrali salvate (`<id>.mzlib`) | binario |
| OPFS | `mzlab/` (radice del motore), `idx/`, `tiles/`, `conv/`, `export/` | file importati, indici, mappe e conversioni: ricostruibili | binario |
| Cache API | `mzlab-app-*`, `mzlab-big-*`, `mzlab-libs` | copia offline dell'app | risposte HTTP |
| sessionStorage | `mzlab.run`, `mzlab.telefono*`, `mzlab.unopen`, `mzlab.convopen`, `mzlab.teoria.y/.side`, `mzlab.tour.step` | stato di questa esecuzione | stringa |

Nota: le chiavi e le cartelle si leggono per NOME dal browser (come `reset.js`, nessuna lista fissa): la tabella spiega le famiglie, la raccolta prende tutto ciò che inizia per `mzlab`/`qqq` meno gli esclusi.
