# AGENTS.md - mzLab (repository Freeesta/mzlab, pacchetto mzlab)

Questo file è già nel tuo contesto (Claude Code lo carica da solo): non rileggerlo. Leggi il file d'area del tuo lavoro (§5). Per trovare un file o una funzione: `python3 tools/indice.py` poi `grep -n '<nome>' docs/agenti/mappa.md`, o il subagente `cercatore`.

**Non leggere** (bloccati da `.claude/settings.json`; grep resta permesso): `mzlab/web/vendor/`, `mzlab/web/teoria/pratica/ei-dati.js`, `mzlab/web/elements.js`, `mzlab/web/teoria/glossario-dati.js`, `mzlab/web/tpmine.enc`, `mzlab/web/esempi/`, `.git/`, `.verifica/` (anche `cache_massbank`), `docs/agenti/misure-diario.md`, `tests_e2e/shots/`, `__pycache__/`, `.pytest_cache/`, `*.egg-info/`, `site/`.

## 1. Il progetto
- **mzLab**: programma DIDATTICO per l'esperienza 3 del laboratorio di analisi degli inquinanti. Gli studenti analizzano dati LC-MS di un triplo quadrupolo SCIEX 3200 QTRAP (risoluzione unitaria, ESI+) per trovare i prodotti di trasformazione (TP) di un inquinante degradato su TiO2. Legge anche file HR (Orbitrap, Q-TOF) e DDA. Nome visibile SOLO in `mzlab/web/appname.js`.
- **Distribuzione**: SOLO il sito https://freeesta.github.io/mzlab/ (GitHub Pages, `pages.yml` a ogni push su `main`). Tutto gira nel browser; i file degli studenti non lasciano il computer.
- **Principi da non violare**:
  1. **Il programma NON dà le risposte** nel mondo LR (niente «ecco i TP», niente rette di taratura o tabelle della relazione: gli studenti le fanno in Excel). Il mondo HR è uno strumento di ricerca: librerie, formule e analoghi come candidati con punteggio e prove, mai come verità. Eccezione: mzFinder (nascosto, solo per Federico).
  2. Leggero: il calcolo pesante vive in Rust (`crates/`: `.wasm` nel sito, nativo nell'app desktop); Python ≥ 3.11 + numpy (Pyodide) serve solo ciò che Rust non fa (HR, Banco, mzFinder, mappe HR, `.dam`) ed è il RIFERIMENTO dei test di parità (`tools/golden.py`); HTML/JS semplice, nessun bundler né dipendenza pesante.
  3. Interfaccia in italiano e inglese; Teoria e Pratica solo in italiano; codice e commenti in inglese. Sempre «m/z», «RT», «XIC», «TIC», «MRM», «MS2».
  4. A risoluzione unitaria un m/z è un candidato, non un'identificazione (spiegato in Teoria, senza avvisi ripetuti nell'interfaccia).
  5. **Teoria e Pratica**: niente traduzioni, aggiunte o tagli senza un'istruzione esplicita di Federico; l'accessibilità (presentazione) è ammessa.
  6. **Accessibilità**: colori solo da variabili CSS, `prefers-reduced-motion`, tastiera e `aria-label`, grafici leggibili dai daltonici (Okabe-Ito); **ogni grafico nuovo nasce con la sua tabella dei dati** (`TDATI`, `docs/agenti/dati.md`), con bersagli di almeno 24×24 px (40 px al tocco) e zoom/spostamento senza trascinamento (WCAG 2.2 AA; `e2e_axe_grafici`).
- **Dati del laboratorio**: MAI nel repository (pubblico); stanno nel repository PRIVATO `Freeesta/mzlab-dati` (`mzML/`, `dam/`, `HRMS/`; prompt in `scratchpad/`). Eccezione: i file anonimi di `mzlab/web/esempi/`. Il nome del composto dell'esperienza, dei suoi TP e degli inquinanti dei metodi non compare mai nel repository pubblico (interfaccia, Teoria, esempi, test, commit, PR; i test lo controllano).

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
- **Un comando**: `python3 tools/verifica.py` (sintassi JS, testi, pytest, `prova_hr`, tutti gli e2e; ~9 min con 4 CPU). Opzioni: `--cambiati` (solo gli e2e dei file cambiati rispetto a `origin/main`), `--ultimi-falliti`, `--solo e2e6,e2e18`, `--fumo`. Log in `.verifica/log/<nome>.log`: solo per i FAIL, letti dal `verificatore`.
- **Tre livelli**: (1) durante il lavoro e prima del merge `--cambiati` + la CI della PR (4 job obbligatori di «main protetto»); (2) CI di ogni PR (`test.yml`); (3) ogni notte su main (`notte.yml`) e a fine di un pacchetto grande.
- **Mappa e soglie**: `docs/agenti/mappa.md` NON è nel repository: si genera con `python3 tools/indice.py` (~1 s; all'inizio della sessione e dopo aver aggiunto o spostato file; `--controlla` verifica i gruppi: una cartella nuova va in `GROUPS`). `python3 tools/controlla.py` avvisa di file/funzioni/righe troppo lunghi e FALLISCE solo se peggiorano rispetto a `tools/soglie_base.json`; la base si restringe con `--aggiorna`, mai si allarga. `tools/noti_rossi.txt`: e2e rossi noti (non bloccanti).
- **Dati**: il repository privato si trova da solo (accanto, sopra, nella home o con `MZLAB_DATI=<percorso>`); senza, `tools/dati_sintetici.py` crea file simili e i test che chiedono dati veri si saltano (tabella `NEEDS`). Test con Pyodide (`e2e13`, `e2e_tpmine*`): prima `python3 tools/build_site.py` (scrive `site/`, da cancellare dopo).
- **Struttura e test nuovi**: `tests/` = pytest, `tests_e2e/` = script Playwright (uno per argomento, `lib.py` → `Run`); regole e aiuti in `docs/agenti/test.md`.

## 4. Architettura
- **Un codice Python per due «server»**: `mzlab/api.py` (`dispatch`: tutte le rotte `/api/...`, errori sempre JSON `{error, error_key, params}`) sopra `App` (`mzlab/app.py`: file aperti, sessione, taccuino). Lo usano `tools/dev_server.py` (solo test e sviluppo) e `mzlab/browser.py` (nel sito, dentro Pyodide). Un endpoint nuovo si aggiunge solo in `api.py`, con un test.
- **Nel sito**: `tools/build_site.py` copia `mzlab/web` in `site/static` e scarica Pyodide; `browser.js` passa `fetch("api/...")` al motore Rust/WASM (`rust-bridge.js`, `rust-worker.js`, `lr.js`; predefinito, `?motore=python` e `?lr=0` tornano al percorso classico), che per i file a bassa risoluzione risponde a file, sessione, taccuino, TIC, XIC, spettri, MRM e formule senza Pyodide (mzML e `.mzidx` in OPFS).
  Pyodide (`browser-worker.js`: Python + `mzlab.zip`) parte una volta, con un avviso, alla prima richiesta che solo Python serve (HR, Banco, mzFinder, mappe HR, `.dam`). Dettagli in `docs/agenti/dati.md` e `docs/agenti/rust-rotte.md`.
- **App desktop** (`crates/mzlab-desktop`, Tauri v2): la stessa pagina web con un motore nativo Rust dietro `mzlab://`; dettagli in `docs/agenti/rust-desktop.md`.
- **Python** (`mzlab/`): `reader/` (mzML senza librerie, profilo, `.dam`, `mzidx.py`: formato in `docs/agenti/mzidx.md`), `explore.py`, `peaks.py`, `project.py`, `chem/` (masse, formule, addotti, albero MSn), `ionfamily.py` (senza verdetti), `demo.py` (file per file: la mappa). `tools/converti.py`: `.raw` con ThermoRawFileParser, `.wiff`/`.d` con MSConvert in Docker.
- **Indirizzi**: `#/dati` e `#/disegno` nell'app; `#/teoria`, `#/teoria/N` e `#/pratica` portano alla Teoria autonoma `static/teoria/` (senza motore; `docs/agenti/teoria.md`).
- **Pagina** (`mzlab/web/`): script classici in `index.html` nell'ordine lì scritto, che condividono il campo globale (un nome `function`/`const` doppio in due file rompe la pagina); globali `E` (file e pannelli), `S` (stato), `NB` (taccuino); moduli ES (`m:` nel caricatore) per i file nuovi e le foglie che nessuno chiama per nome; tipi: `// @ts-check` + `tsconfig.json` (job `tipi`); ogni pulsante con `data-ic` sta in `QICON`. Dettagli in `docs/agenti/dati.md`.
- **Due lingue** (dettagli in `docs/agenti/lingue.md`): ogni testo nuovo nasce come chiave in `lang/it.js` e `lang/en.js`, mai scritto a mano nel codice; `tools/controlla_i18n.py` rifiuta italiano fuori dai cataloghi, chiavi mancanti o orfane.
- **Sicurezza**: CSP nel meta di `index.html` (niente `unsafe-eval`, `connect-src 'self'`); testi dai file da `EH()`; Ketcher in iframe `sandbox`.
- **Dati utente nel browser, nomi da NON cambiare mai**: IndexedDB `"qqq_lab"`, localStorage `qqq.*` e `qqq-labels`, cache `qqq-app-*`, `qqq-big-*`, `mzlab-libs`.
- **Smartphone** (`telefono.js`): su telefono (`?telefono=1|0` forza) non si scaricano Dati, Disegno, Pratica né motore: arriva `telefono-scheda.js`. Un file di script nuovo di Dati va aggiunto nella lista `L` del caricatore in `index.html` (`docs/agenti/dati.md`).

## 5. File d'area (leggi solo quello del tuo lavoro)
| Se tocchi… | Leggi |
|---|---|
| vista Dati in bassa risoluzione: caricamento, schede, TIC/XIC/spettro, integrazione, barra laterale, tour | `docs/agenti/dati.md` |
| header (schermo intero, logo, (i)), area e tabella di caricamento | `docs/agenti/intestazione.md` |
| mappa RT×m/z, mirino, navigazione, visore 3D | `docs/agenti/dati-mappa.md` |
| liste, correzione, flusso di lavoro, QA/QC, statistica, rapporto | `docs/agenti/dati-strumenti.md` |
| fatti sui dati del 3200, gradiente, tracce analogiche, tabelle dei dati, tema | `docs/agenti/dati-3200.md` |
| mondo HR: Banco, formule, isotopi, librerie, albero MSn | `docs/agenti/alta-risoluzione.md` |
| statistica in HR: tabella delle feature, bianco, differenza dal riferimento, PCA | `docs/agenti/alta-risoluzione-statistica.md` |
| formato `.mzlib`, parser delle librerie, indici e punteggi (`crates/mzlab-lib`) | `docs/agenti/librerie.md` |
| Disegno (Ketcher) | `docs/agenti/disegno.md` |
| Teoria e Pratica (`mzlab/web/teoria/`) | `docs/agenti/teoria.md` (e `teoria/LEGGIMI.md` se scrivi capitoli) |
| testi, `lang/`, `help.js`, inglese | `docs/agenti/lingue.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` (tabelle untargeted e identità delle feature: `mzfinder-untargeted.md`; validazione delle feature: `mzfinder-validazione.md`; indice dei sospetti `.mzsus`: `sospetti.md`) |
| mzFinder nel browser (`crates/mzlab-feat-wasm`, `TP_Mine/files/feat-worker.js`, pannello Pipeline `TP_Mine/js/32_pipeline.js`, stima dei parametri `crates/mzlab-feat/src/params.rs`) | `docs/agenti/mzfeat-wasm.md` |
| benchmark di mzFinder contro altri strumenti (`tools/bench/`, tabella `bench/1`, metriche P1-P5, statistica) | `docs/agenti/mzfinder-benchmark.md` |
| motore Rust (`crates/`, dati d'oro `tools/golden.py`) | `docs/agenti/rust.md` (+ `rust-rotte.md`, `rust-desktop.md`) |
| misure dei motori (ultima misura per sezione) | `docs/agenti/misure.md` |
| struttura dei test, test nuovi | `docs/agenti/test.md` |
| lanciare la verifica nel container cloud (Chromium, `--setup`, test che costruiscono il sito) | `docs/agenti/ambiente-cloud.md` |

## 6. Trappole generali
- Service worker, anteprime dei link, `title` su iframe, `ask()`: in fondo a `docs/agenti/dati.md`.
- Un `verifica.py` mentre si modificano i file contamina il risultato: un giro alla volta su un worktree fermo.
- Il proxy GitHub del cloud può ignorare la cancellazione di rami remoti: li cancella Federico.
- **Tour guidato**: ogni modifica dell'interfaccia deve lasciare validi `tour.js`, `tests_e2e/e2e_tour.py` e l'invito «Start the tour»: ogni passo deve trovare il suo bersaglio visibile.

## Compact instructions
Nel riassunto tieni: pacchetto e PR (numero, ramo, stato CI), punti fatti e da fare del prompt, decisioni prese e perché, errori aperti con il comando che li riproduce, file toccati. Togli: contenuto dei file letti, log, output dei test passati.
