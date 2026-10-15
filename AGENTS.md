# AGENTS.md - mzLab (repository Freeesta/mzlab, pacchetto mzlab)

Questo file è già nel tuo contesto (Claude Code lo carica da solo): non rileggerlo. Leggi il file d'area del tuo lavoro (§5). Per trovare un file o una funzione: `python3 tools/indice.py` poi `grep -n '<nome>' docs/agenti/mappa.md`, o il subagente `cercatore`.

**Non leggere** (bloccati da `.claude/settings.json`; grep resta permesso): `mzlab/web/vendor/`, `mzlab/web/teoria/pratica/ei-dati.js`, `mzlab/web/elements.js`, `mzlab/web/teoria/glossario-dati.js`, `mzlab/web/esempi/`, `.git/`, `.verifica/` (anche `cache_massbank`), `docs/agenti/misure-diario.md`, `tests_e2e/shots/`, `__pycache__/`, `.pytest_cache/`, `*.egg-info/`, `site/`.

## 1. Il progetto
- **mzLab**: programma DIDATTICO per l'esperienza 3 del laboratorio di analisi degli inquinanti. Gli studenti analizzano dati LC-MS di un triplo quadrupolo SCIEX 3200 QTRAP (risoluzione unitaria, ESI+) per trovare i prodotti di trasformazione (TP) di un inquinante degradato su TiO2. Legge anche file HR (Orbitrap, Q-TOF) e DDA. Nome visibile SOLO in `mzlab/web/appname.js`.
- **Distribuzione**: SOLO il sito https://freeesta.github.io/mzlab/ (GitHub Pages, `pages.yml` a ogni push su `main`). Tutto gira nel browser; i file degli studenti non lasciano il computer.
- **Principi da non violare**:
  1. **Il programma NON dà le risposte** nel mondo LR (niente «ecco i TP», niente rette di taratura o tabelle della relazione: gli studenti le fanno in Excel). Il mondo HR è uno strumento di ricerca: librerie, formule e analoghi come candidati con punteggio e prove, mai come verità. Eccezione: mzFinder (nascosto, solo per Federico).
  2. Leggero: il programma servito è Rust (`crates/`: `.wasm` nel sito, nativo nell'app desktop) + HTML/JS semplice, senza bundler né dipendenze pesanti. Python ≥ 3.11 + numpy è solo RIFERIMENTO dei test di parità (`tools/golden.py`, `tools/*_riferimento.py`) e strumento (`tools/`): mai nel sito (fa eccezione solo mzFinder, nascosto, che tiene il suo Pyodide in `static/mzfinder/`) e nessuna funzione nuova nasce in Python.
  3. Interfaccia in italiano e inglese; Teoria e Pratica solo in italiano; codice e commenti in inglese. Sempre «m/z», «RT», «XIC», «TIC», «MRM», «MS2».
  4. A risoluzione unitaria un m/z è un candidato, non un'identificazione (spiegato in Teoria, senza avvisi ripetuti nell'interfaccia).
  5. **Teoria e Pratica**: niente traduzioni, aggiunte o tagli senza un'istruzione esplicita di Federico; l'accessibilità (presentazione) è ammessa.
  6. **Accessibilità**: colori solo da variabili CSS, `prefers-reduced-motion`, tastiera e `aria-label`, grafici leggibili dai daltonici (Okabe-Ito); **ogni grafico nuovo nasce con la sua tabella dei dati** (`TDATI`, `docs/agenti/dati.md`), con bersagli di almeno 24×24 px (40 px al tocco) e zoom/spostamento senza trascinamento (WCAG 2.2 AA; `e2e_axe_grafici`).
- **Dati del laboratorio**: MAI nel repository (pubblico); stanno nel repository PRIVATO `Freeesta/mzlab-dati` (`mzML/`, `dam/`, `HRMS/`; prompt in `scratchpad/`). Eccezione: i file anonimi di `mzlab/web/esempi/`. I cognomi elencati in `nomi_vietati.txt` del repository privato non compaiono mai nel repository pubblico (interfaccia, Teoria, esempi, test, commit, PR; `tests/test_esempi_anonimi.py` lo controlla); nessun composto è vietato.

## 2. Regole di lavoro
- **Consegna = sito pubblicato**: il lavoro è finito quando è in `main` e il workflow «pages» di quel commit è riuscito.
- **Flusso**: ramo `claude/<argomento>` da `origin/main` → PR → unione con merge commit (mai rebase né push forzato). Se la PR è già unita, ricrea il ramo da `origin/main`. Prima di unire: `git merge origin/main`, risolvi i conflitti tenendo il lavoro di tutti, verifica di nuovo. Se scegliere fa perdere un comportamento: PR aperta e domanda al Coordinatore.
- **Perimetro**: ogni chat tocca solo ciò che dice il suo prompt; a fine lavoro UN messaggio al Coordinatore (max 5 righe: PR, CI, pages, cosa resta). Rosso o decisione da prendere: PR aperta con la spiegazione, niente merge.
- **Commit**: titolo breve in italiano + 1-3 righe + attribuzione dell'ambiente. Mai nomi di modelli.
- **Documenti**: AGENTS.md e `docs/agenti/` descrivono solo lo stato attuale. Quando cambi un comportamento, RISCRIVI la riga che lo descrive (niente «ora…», «prima era…», numeri di PR, date); una funzione nuova = una riga nel file della sua area; una riga di documento ≤ 600 caratteri, un file d'area ≤ 25 KB (oltre si divide per argomento).
- **Consumo di token**: ogni turno rilegge tutto il contesto, quindi una chat lunga costa più di due chat corte. Una sessione per pacchetto; leggi solo il file d'area che ti serve; `grep -n` e letture a intervalli; niente screenshot nel contesto salvo un giudizio visivo; raggruppa le modifiche e prova una volta alla fine.
  **Consegna**: alla fine della PR in corso, se il contesto supera 200k token (sempre sopra 300k), manda al Coordinatore il blocco di consegna (≤15 righe: prompt, ramo, PR e CI, fatto, da fare con il prossimo passo esatto, decisioni, trappole file:riga, comandi di prova con atteso, cosa non rileggere) e fermati.
- **Subagenti Haiku** (`.claude/agents/`): le mani, non la testa; compiti meccanici con ingresso esatto, lanciati insieme in un solo messaggio se indipendenti: `verificatore`, `controllo-github`, `cercatore` (`file:riga`), `inventario-testi`, `sostituzioni` (vecchio → nuovo), `archivista` (`git mv`), `estrattore` (passaggi su un argomento), `stato-sessioni`. Non usare il predefinito Explore (gira sul modello principale): usa `cercatore`. Controlla sempre il risultato; due errori = lo fa la chat.
  Mai a un subagente: decisioni, codice nuovo, testi di Teoria, asserzioni dei test, commit, push, merge.
- **Agente locale** (Mac di Federico, `~/mzLab/`): può fare commit e push su `main`; non cancella file, li sposta in `~/mzLab/_cestino/<data>_<motivo>/`.
- Un front end non provato nel browser è rotto. Niente `pkill -f` con parole presenti nel comando: ferma i server con il PID.

## 3. Verifica e test
- **Verifica**: `python3 tools/verifica.py` (~9 min con 4 CPU; `--cambiati` = solo gli e2e dei file cambiati rispetto a `origin/main`; log in `.verifica/log/`, letti dal `verificatore`). Prima del merge: `--cambiati` + la CI della PR (`test.yml`, 4 job obbligatori di «main protetto»); ogni notte `notte.yml`. Livelli, mappa (`python3 tools/indice.py`, non è nel repository), soglie (`tools/controlla.py`) e `noti_rossi.txt`: `docs/agenti/test.md`.
- **Dati**: il repository privato si trova da solo (accanto, sopra, nella home o con `MZLAB_DATI=<percorso>`); senza, `tools/dati_sintetici.py` crea file simili e i test che chiedono dati veri si saltano (tabella `NEEDS`). Test con Pyodide (`e2e13`, `e2e_tpmine*`): prima `python3 tools/build_site.py` (scrive `site/`, da cancellare dopo).
- **Struttura e test nuovi**: `tests/` = pytest, `tests_e2e/` = script Playwright (uno per argomento, `lib.py` → `Run`); regole e aiuti in `docs/agenti/test.md`.

## 4. Architettura
- **Python = riferimento**: `mzlab/api.py` (`dispatch`: tutte le rotte `/api/...`, errori sempre JSON `{error, error_key, params}`) sopra `App` (`mzlab/app.py`: file aperti, sessione, taccuino). Lo usa `tests_e2e/pyref.py` (il riferimento nel processo del test). Una rotta nuova nasce in Rust (`crates/mzlab-core`) con la sua parità; `api.py` si tocca solo per correggere bug trovati dalla parità.
- **Nel sito**: `tools/build_site.py` copia `mzlab/web` in `site/static`; `browser.js` passa `fetch("api/...")` al motore Rust/WASM (`rust-bridge.js`, `rust-worker.js`, `lr.js`), che risponde a tutte le rotte (file, sessione, taccuino, cromatogrammi, spettri, MRM, formule, mappe, HR; mzML e `.mzidx` in OPFS).
  Un solo motore e nessun ripiego: se il worker muore due volte in un minuto la pagina chiede di ricaricare (`error.engineFallback`); una richiesta che il motore non sa servire risponde `err.route.unavailable` e finisce in `window.MZLAB_LR_MISS` (i test la vogliono vuota). Pyodide (`tpmine-worker.js`) resta solo per mzFinder. Dettagli in `docs/agenti/dati.md` e `docs/agenti/rust-rotte.md`.
- **App desktop** (`crates/mzlab-desktop`, Tauri v2): la stessa pagina web con un motore nativo Rust dietro `mzlab://`; dettagli in `docs/agenti/rust-desktop.md`.
- **Python** (`mzlab/`): `reader/` (mzML senza librerie, profilo, `.dam`, `mzidx.py`: formato in `docs/agenti/mzidx.md`), `explore.py`, `peaks.py`, `project.py`, `chem/` (masse, formule, addotti, albero MSn), `ionfamily.py` (senza verdetti), `demo.py` (file per file: la mappa). `tools/converti.py`: `.raw` con ThermoRawFileParser, `.wiff`/`.d` con MSConvert in Docker.
- **Indirizzi**: `#/dati` e `#/disegno` nell'app; `#/teoria`, `#/teoria/N` e `#/pratica` portano alla Teoria autonoma `static/teoria/` (senza motore; `docs/agenti/teoria.md`).
- **Pagina** (`mzlab/web/`): script classici in `index.html` nell'ordine lì scritto, che condividono il campo globale (un nome `function`/`const` doppio in due file rompe la pagina); globali `E` (file e pannelli), `S` (stato), `NB` (taccuino); moduli ES (`m:` nel caricatore) per i file nuovi e le foglie che nessuno chiama per nome; tipi: JSDoc + `tsc` (`tools/tipi/`, job `tipi`): un file nuovo ha `// @ts-check`, sta in un progetto di `tools/tipi/` e ha 0 errori, la base `base_errori.json` scende soltanto; ogni pulsante con `data-ic` sta in `QICON`. Dettagli in `docs/agenti/dati.md`.
- **Due lingue** (dettagli in `docs/agenti/lingue.md`): ogni testo nuovo nasce come chiave in `lang/it.js` e `lang/en.js`, mai scritto a mano nel codice; `tools/controlla_i18n.py` rifiuta italiano fuori dai cataloghi, chiavi mancanti o orfane.
- **Sicurezza**: CSP nel meta di `index.html` (niente `unsafe-eval`, `connect-src 'self'`); testi dai file da `EH()`; Ketcher in iframe `sandbox`.
- **Dati utente nel browser, nomi da NON cambiare mai**: IndexedDB `"qqq_lab"`, localStorage `qqq.*` e `qqq-labels`, cache `qqq-app-*`, `qqq-big-*`, `mzlab-libs`.
- **Smartphone** (`telefono.js`): su telefono (`?telefono=1|0` forza) non si scaricano Dati, Disegno, Pratica né motore: arriva `telefono-scheda.js`. Un file di script nuovo di Dati va aggiunto nella lista `L` di `carica.js` (`docs/agenti/dati.md`).

## 5. File d'area (leggi solo quello del tuo lavoro)
| Se tocchi… | Leggi |
|---|---|
| vista Dati in bassa risoluzione: caricamento, schede, TIC/XIC/spettro, integrazione | `docs/agenti/dati.md` |
| primo caricamento di più file (si apre solo il primo), campi della tabella di carico | `docs/agenti/dati-carico.md` |
| barra laterale della vista Dati, strumenti, suggerimenti, tour | `docs/agenti/dati-barra.md` |
| azioni dei pannelli (registro `azioni/1`, menu del tasto destro, Maiusc+F10) | `docs/agenti/azioni.md` |
| animazione di caricamento (`base`) e analizzatori animati spenti (`an-*.js`: quadrupolo, TOF, Orbitrap, ESI, trappola, FT-ICR, QqQ, Paul, settore, IMS, MALDI) | `docs/agenti/analizzatori.md` |
| header (schermo intero, logo, (i)), area e tabella di caricamento | `docs/agenti/intestazione.md` |
| carica una cartella, cartella puntata e ricordata (elenco in grigio, «Altri», desktop) | `docs/agenti/dati-cartella.md` |
| mappa RT×m/z, mirino, navigazione, visore 3D | `docs/agenti/dati-mappa.md` |
| velocità della mappa: misure dei gesti (`tools/bench_mappa.py`), diagnosi e pacchetti P1-P10 | `docs/agenti/mappa-fluida.md` |
| liste, correzione, flusso di lavoro, QA/QC, statistica, rapporto | `docs/agenti/dati-strumenti.md` |
| fatti sui dati del 3200, gradiente, tracce analogiche, tabelle dei dati, tema | `docs/agenti/dati-3200.md` |
| mondo HR: Banco, formule, isotopi, librerie, albero MSn | `docs/agenti/alta-risoluzione.md` |
| statistica in HR: tabella delle feature, bianco, differenza dal riferimento, PCA | `docs/agenti/alta-risoluzione-statistica.md` |
| formato `.mzlib`, parser delle librerie, indici e punteggi (`crates/mzlab-lib`) | `docs/agenti/librerie.md` |
| Disegno (Ketcher) | `docs/agenti/disegno.md` |
| Teoria e Pratica (`mzlab/web/teoria/`) | `docs/agenti/teoria.md` (e `teoria/LEGGIMI.md` se scrivi capitoli) |
| testi, `lang/`, `help.js`, inglese | `docs/agenti/lingue.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` (tabelle untargeted e identità delle feature: `mzfinder-untargeted.md`; validazione delle feature: `mzfinder-validazione.md`; tabella dei prodotti HR: `mzfinder-tabella.md`; indice dei sospetti `.mzsus`: `sospetti.md`) |
| mzFinder nel browser (`crates/mzlab-feat-wasm`, `TP_Mine/files/feat-worker.js`, pannello Pipeline `TP_Mine/js/32_pipeline.js`, stima dei parametri `crates/mzlab-feat/src/params.rs`) | `docs/agenti/mzfeat-wasm.md` |
| lettore `.wiff` + `.wiff.scan` (`crates/mzlab-core/src/wiff/`) | `docs/agenti/wiff.md` (altri strumenti SCIEX, sMRM, PIS/NL, DAD, feature `estesi`: `wiff-varianti.md`) |
| lettore cartelle Agilent `.D` (`crates/mzlab-core/src/agilent/`) | `docs/agenti/agilent.md` |
| benchmark di mzFinder contro altri strumenti (`tools/bench/`, tabella `bench/1`, metriche P1-P5, statistica) | `docs/agenti/mzfinder-benchmark.md` |
| pipeline delle feature a bassa risoluzione (full scan, asse in Da, profilo `lr`: `recipe_lr.rs`, `axis.rs`) | `docs/agenti/mzfeat-lr.md` |
| ingresso di mzFinder: `.mzidx` per ogni file scelto o da IndexedDB (`TP_Mine/files/finder-idx.js`), lista di trasformazioni dell'utente in Rust | `docs/agenti/mzfinder-ingresso.md` |
| pipeline delle feature HR in Rust (`crates/mzlab-feat`) | `docs/agenti/mzfeat.md` (prove e misure: `mzfeat-prove.md`; tabella, id e campioni: `mzfeat-tabella.md`; vista dei prodotti `tp/1`: `mzfeat-tp.md`) |
| Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di rilevamento alla mzmine: `crates/mzlab-feat/src/attributio/`, `untargeted/`, `.wasm` `attributio` e `untargeted`) | `docs/agenti/attributio.md` |
| ruoli dei file e tempi dei campioni (`campioni/2`, `mzlab-core/src/campioni/`: proposte dai nomi, avvisi, migrazione, indici per il percorso TP) | `docs/agenti/campioni.md` |
| contratto di memoria dei moduli WASM (`mzlab-core::budget`, uscite a offset, lastre, flusso delle prove) | `docs/agenti/memoria.md` |
| formato `.mzidx` | `docs/agenti/mzidx.md` |
| cosa i file grezzi contengono e non estraiamo ancora (regola «nessuna perdita») | `docs/agenti/grezzi-non-estratti.md` |
| lettura nativa di `.raw`, `.wiff`, `.D` nel sito (campioni, OPFS, fasi di avanzamento, mzML su richiesta) | `docs/agenti/lettura-nativa.md` |
| motore Rust (`crates/`, dati d'oro `tools/golden.py`) | `docs/agenti/rust.md` (+ `rust-rotte.md`, `rust-desktop.md`) |
| misure dei motori (ultima misura per sezione) | `docs/agenti/misure.md` |
| struttura dei test, test nuovi | `docs/agenti/test.md` |
| workflow GitHub condizionali (concurrency, pages, job saltati) | `docs/agenti/ci-veloce.md` |
| contratto dei lettori dei produttori (`lettori.rs`, trait `Lettore`, registro, `Limiti`), aggiungere un formato | `docs/agenti/estendere.md` |
| modulo «aperti» (mzML con numpress, mzXML: `crates/mzlab-lettori/src/aperti/`, `numpress.rs`) | `docs/agenti/lettori-aperti.md` |
| modulo «andi» (netCDF/ANDI `.cdf`: `crates/mzlab-lettori/src/andi/`) | `docs/agenti/lettori-andi.md` |
| moduli lettori, uno per famiglia (`crates/mzlab-lettori*`, `.wasm` a parte in `wasm-lettori/<famiglia>/`, feature `estesi`, `LETTORI_FAM`, regola R in `shell.js`, `LETTORI_MOD`, `?lettori=modulo`, tetti) | `docs/agenti/lettori-modulo.md` |
| rilascio: tag `vX.Y.Z`, `release.yml`, `CHANGELOG.md`, versione unica, SBOM | `docs/agenti/rilascio.md` |
| lanciare la verifica nel container cloud (Chromium, `--setup`, test che costruiscono il sito) | `docs/agenti/ambiente-cloud.md` |

## 6. Trappole generali
- Service worker, anteprime dei link, `title` su iframe, `ask()`: in fondo a `docs/agenti/dati.md`.
- Un `verifica.py` mentre si modificano i file contamina il risultato: un giro alla volta su un worktree fermo.
- Il proxy GitHub del cloud può ignorare la cancellazione di rami remoti: li cancella Federico.
- **Tour guidato**: ogni modifica dell'interfaccia deve lasciare validi `tour.js`, `tests_e2e/e2e_tour.py` e l'invito «Start the tour»: ogni passo deve trovare il suo bersaglio visibile.

## Compact instructions
Nel riassunto tieni: pacchetto e PR (numero, ramo, stato CI), punti fatti e da fare del prompt, decisioni prese e perché, errori aperti con il comando che li riproduce, file toccati. Togli: contenuto dei file letti, log, output dei test passati.
