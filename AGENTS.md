# AGENTS.md - mzLab (repository Freeesta/mzlab, pacchetto mzlab)

Nucleo per tutti gli agenti: solo lo STATO ATTUALE (la storia è in git). Leggi questo file e il file d'area del tuo lavoro (§5); poi `grep -n` sul codice, mai file interi sopra le 500 righe.

**Non leggere** (bloccati anche da `.claude/settings.json`; grep resta permesso): `mzlab/web/vendor/`, `mzlab/web/teoria/pratica/ei-dati.js`, `mzlab/web/elements.js`, `mzlab/web/teoria/glossario-dati.js`, `mzlab/web/esempi/`, `.git/`, `.verifica/`, `tests_e2e/shots/`, `__pycache__/`, `.pytest_cache/`, `*.egg-info/`, `site/`.

## 1. Il progetto
- **mzLab**: programma DIDATTICO per l'esperienza 3 del laboratorio di analisi degli inquinanti (UniTO; titolare Federico Cristaudo). Gli studenti analizzano dati LC-MS di un triplo quadrupolo SCIEX 3200 QTRAP (risoluzione unitaria, ESI+) per trovare i prodotti di trasformazione (TP) di un inquinante degradato per fotocatalisi su TiO2. Legge anche file HR (Orbitrap, Q-TOF) e DDA. Nome visibile scritto SOLO in `mzlab/web/appname.js`.
- **Distribuzione**: SOLO il sito https://freeesta.github.io/mzlab/ (GitHub Pages, `pages.yml` a ogni push su `main`). Tutto gira nel browser (Pyodide); i file degli studenti non lasciano il computer. Nessun tracciamento né statistica.
- **Principi da non violare**:
  1. **Il programma NON dà le risposte** nel mondo LR (niente «ecco i TP», niente rette di taratura o tabelle della relazione: gli studenti le fanno in Excel). Il mondo HR è uno strumento di ricerca: librerie, formule e analoghi come candidati con punteggio e prove, mai come verità. Eccezione: mzFinder (nascosto, solo per Federico).
  2. Leggero: Python ≥ 3.11 + numpy; HTML/JS semplice, nessun bundler, nessuna dipendenza pesante; JS senza build; Rust → .wasm solo nella CI (mai committato).
  3. Interfaccia in italiano e inglese; Teoria e Pratica solo in italiano; codice e commenti in inglese. Sempre «m/z», «RT», «XIC», «TIC», «MRM», «MS2» (la pagina mostra MS<sup>2</sup>).
  4. A risoluzione unitaria un m/z è un candidato, non un'identificazione (spiegato in Teoria, senza avvisi ripetuti nell'interfaccia).
  5. **Teoria e Pratica**: niente traduzioni, aggiunte o tagli senza un'istruzione esplicita di Federico; l'accessibilità (presentazione) è ammessa.
  6. **Accessibilità**: colori solo da variabili CSS, `prefers-reduced-motion`, tastiera e `aria-label`, grafici leggibili dai daltonici (Okabe-Ito).
- **Dati del laboratorio**: MAI nel repository (pubblico); stanno nel repository PRIVATO `Freeesta/mzlab-dati` (`mzML/`, `dam/`, `HRMS/`; prompt dei lavori in `scratchpad/`). Eccezione: i file anonimi di `mzlab/web/esempi/` (5 Full Scan, 2 MS2, 4 standard MRM, 1 ritaglio Orbitrap DDA autorizzato da Federico). Il nome del composto dell'esperienza, dei suoi TP e degli inquinanti dei metodi non compare mai nel repository pubblico (interfaccia, Teoria, giochi, esempi, test, commit, PR; i test lo controllano).

## 2. Regole di lavoro
- **Consegna = sito pubblicato**: il lavoro è finito quando è in `main` e il workflow «pages» di quel commit è riuscito (a Federico basta Cmd/Ctrl+Shift+R).
- **Flusso**: ramo `claude/<argomento>` da `origin/main` → PR → unione con merge commit (mai rebase né push forzato; il ramo si cancella da solo). Se la PR del ramo è già unita, ricrea il ramo da `origin/main`. Prima di unire: `git fetch`, `git merge origin/main`, risolvi tu i conflitti tenendo il lavoro di tutti, verifica di nuovo. Se scegliere fa perdere un comportamento: PR aperta e una domanda al Coordinatore.
- **Perimetro**: ogni chat tocca solo ciò che dice il suo prompt; a fine lavoro UN messaggio al Coordinatore (max 5 righe: PR, CI, pages, cosa resta). Qualcosa rosso o una decisione da prendere: PR aperta con la spiegazione, niente merge.
- **Commit**: titolo breve in italiano + 1-3 righe, più le righe di attribuzione dell'ambiente. Mai nomi di modelli.
- **Documenti**: AGENTS.md e `docs/agenti/` descrivono solo lo stato attuale. Quando cambi un comportamento, RISCRIVI la riga che lo descrive (niente «ora…», «prima era…», numeri di PR, date); una funzione nuova = una riga nel file della sua area.
- **Consumo di token**: una sessione per pacchetto di lavoro (mai «continua anche con…» in una chat lunga); leggi solo il file d'area che ti serve; `grep -n` e letture a intervalli; niente screenshot nel contesto salvo un giudizio visivo (quelli per la PR si allegano senza leggerli); raggruppa le modifiche e prova una volta alla fine.
- **Subagenti Haiku** (`.claude/agents/`): le mani, non la testa; compiti meccanici con ingresso esatto e risultato controllabile, lanciati insieme in un solo messaggio quando sono indipendenti. `verificatore` (esegue la verifica, riporta solo i FAIL), `controllo-github` (PR, CI, pages, rami; per un job rosso la riga decisiva), `cercatore` (`file:riga` di una funzione, chiave, selettore o testo), `inventario-testi` (tabella dei testi visibili di un file o di una scheda), `sostituzioni` (elenco esatto vecchio → nuovo con le occorrenze attese), `archivista` (spostamenti con `git mv`). Controlla sempre il risultato; due errori = lo fa la chat. Mai a un subagente: decisioni, codice nuovo, testi di Teoria, asserzioni dei test, commit, push, merge, messaggi.
- **Agente locale** (Mac di Federico, `~/mzLab/mzlab` e `~/mzLab/mzlab-dati`): può fare commit e push su `main`; non cancella file, li sposta in `~/mzLab/_cestino/<data>_<motivo>/`.
- Un front end non provato nel browser è rotto: niente «funziona» senza prove. Niente `pkill -f` con parole presenti nel comando (uccide la propria shell): ferma i server con il PID.

## 3. Verifica e test
- **Un comando**: `python3 tools/verifica.py` (sintassi JS, controllo dei testi, pytest, `prova_hr`, tutti gli e2e trovati con il glob `tests_e2e/e2e*.py`; ~9 min con 4 CPU). Opzioni: `--cambiati` (solo gli e2e dei file cambiati rispetto a `origin/main`, tabella `AREE`; un file del nucleo come `explore.js` o `index.html` li chiede tutti, i soli documenti nessuno), `--ultimi-falliti`, `--rapida`, `--solo e2e6,e2e18`, `--senza-e2e`, `--paralleli N`, `--browser firefox|webkit`, `--fumo` (`SMOKE`). Log in `.verifica/log/<nome>.log`: solo per i FAIL, letti dal `verificatore`.
- **Tre livelli**: (1) durante il lavoro e prima del merge `--cambiati` + la CI della PR; (2) CI di ogni PR (`test.yml`: pytest Linux 3.11/3.14, sintassi JS, `--fumo` Chromium; i suoi 4 job sono i controlli obbligatori di «main protetto»; un push nuovo ferma il giro vecchio); (3) ogni notte su main e a mano a fine di un pacchetto grande (`notte.yml`: pytest Mac e Windows, `--fumo` Firefox/WebKit/Chromium Windows, verifica completa Chromium Linux).
- **Dati**: il repository privato si trova da solo (accanto, in una cartella sopra, nella home o con `MZLAB_DATI=<percorso>`); senza, `tools/dati_sintetici.py` crea file simili e i test che chiedono dati veri, un `.dam` o il sito costruito si saltano (tabella `NEEDS`). Test del sito con Pyodide (`e2e13`, `e2e_tpmine*`): prima `python3 tools/build_site.py` (scrive `site/`, da cancellare dopo).
- **Struttura**: `tests/` = pytest (anche funzioni JS pure con node; `I18N` da `tests/node_i18n.js`); `tests_e2e/` = script Playwright, uno per argomento; aiuti (`NOT_TESTS`): `lib.py` (`Run`: server di sviluppo + browser in `it-IT`, porta e cartella proprie), `lib_hr.py`, `synth.py`, `make_examples.py`, `lat_arrows.py`.
- **Test nuovi**: `r.port` (mai una porta nell'URL); dopo un'azione che chiede dati `ready(pg)` invece di pause fisse; un controllo già presente non si ripete (`grep -n 'step("' tests_e2e/*.py`); il test va in `AREE` se copre file precisi e in `NEEDS` se gli serve qualcosa; la CSP vieta `eval` (a `wait_for_function` passa espressioni, non funzioni lunghe); un test che dipende dall'ora tollera il cambio di minuto; su Windows l'output di node si legge con `encoding="utf-8"`.

## 4. Architettura
- **Un codice Python per due «server»**: `mzlab/api.py` (`dispatch`: tutte le rotte `/api/...`, errori sempre JSON `{error, error_key, params}`) sopra `App` (`mzlab/app.py`: file aperti, sessione, taccuino). Lo usano `tools/dev_server.py` (solo test e sviluppo) e `mzlab/browser.py` (nel sito, dentro Pyodide). Un endpoint nuovo si aggiunge solo in `api.py`, con un test. `/api/scanbin` restituisce blocchi binari (usati da `scroll.js` solo per i file HR).
- **Nel sito**: `tools/build_site.py` copia `mzlab/web` in `site/static`, scarica Pyodide, scrive titolo e tag Open Graph e il service worker `sw.js` (app: rete prima; Pyodide e vendor: cache prima). `browser.js` passa `fetch("api/...")` a `browser-worker.js` (Pyodide, numpy, `mzlab.zip`; file e taccuino in IndexedDB; file > 50 MB come Blob in `/big/N`, oltre 300 MB non salvati). Con `?motore=rust` TIC, XIC e scansioni singole le risponde un motore Rust/WASM (`docs/agenti/rust.md`). Caricamento: subito solo il motore (~10 MB), poi da liberi ciò che sta sotto 1 MB (Teoria, OpenChemLib), Ketcher al passaggio su «Disegno», esempi solo col pulsante.
- **Python** (`mzlab/`): `reader/` (mzML senza librerie, profilo, `.dam`), `explore.py` (`Session`, `Item`), `peaks.py`, `project.py` (tipo, tempo e concentrazione dai nomi, riconoscimento in italiano), `chem/` (`elements.py` masse, formule, addotti, arrotondamento half-up; composizione, contaminanti, albero MSn), `ionfamily.py` (senza verdetti), `demo.py`. `tools/converti.py`: `.raw` con ThermoRawFileParser, `.wiff`/`.d` con MSConvert in Docker (nel sito si mostra la ricetta).
- **Pagina** (`mzlab/web/`): script classici in `index.html` nell'ordine lì scritto, che condividono il campo globale (una `function` o `const` con lo stesso nome in due file rompe la pagina); globali `E` = file e pannelli, `S` = stato, `NB` = taccuino. `draw.js` è un modulo ES; `xlsx.js` scrive .xlsx senza librerie; `perf.js` = `?perf`. `sw.js`, `appname.js`, icone e anteprima sono generati da `tools/genera_icone.py` e `tools/genera_anteprima.py`; ogni pulsante con `data-ic` deve stare in `QICON` (`icons-modi.js`).
- **Due lingue** (dettagli in `docs/agenti/lingue.md`): ogni testo nuovo nasce come chiave in `lang/it.js` e `lang/en.js`, mai scritto a mano nel codice; `python3 tools/controlla_i18n.py` (nella verifica e nella CI) rifiuta italiano fuori dai cataloghi, chiavi mancanti o orfane.
- **Sicurezza**: CSP nel meta di `index.html` (niente `unsafe-eval`, `connect-src 'self'`); ogni testo dai file passa da `EH()`; Ketcher in iframe `sandbox`.
- **Dati utente nel browser, nomi da NON cambiare mai**: IndexedDB `"qqq_lab"`, localStorage `qqq.*` e `qqq-labels`, cache `qqq-app-*`, `qqq-big-*`, `mzlab-libs`.
- **Smartphone** (`telefono.js`; stessa regola in `teoria.js`): user agent da telefono o touch con lato corto < 500 px (`?telefono=1|0` forza): header con logo, Teoria e «i», avviso; nessun motore, nessun Disegno; i giochi (tranne le domande dell'orale) rimandano al computer.

## 5. File d'area (leggi solo quello del tuo lavoro)
| Se tocchi… | Leggi |
|---|---|
| vista Dati in bassa risoluzione: caricamento, schede, TIC/XIC/spettro, integrazione, mappa, barra laterale, liste, tour | `docs/agenti/dati.md` |
| mondo HR: Banco, formule, isotopi, librerie, albero MSn | `docs/agenti/alta-risoluzione.md` |
| formato `.mzlib`, parser delle librerie, indici e punteggi (`crates/mzlab-lib`) | `docs/agenti/librerie.md` |
| Disegno (Ketcher) | `docs/agenti/disegno.md` |
| Teoria e Pratica (`mzlab/web/teoria/`) | `docs/agenti/teoria.md` (e `teoria/LEGGIMI.md` se scrivi capitoli) |
| testi, `lang/`, `help.js`, inglese | `docs/agenti/lingue.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` |
| motore Rust (`crates/`, dati d'oro `tools/golden.py`) | `docs/agenti/rust.md` |

## 6. Trappole generali
- Il service worker tiene in cache l'app: dopo un deploy prova con Cmd/Ctrl+Shift+R. Le anteprime dei link restano in cache presso le app di messaggistica (`?v=2` sul link; con un'immagine nuova cambia anche il `?v=` di `og:image` in `build_site.py`).
- Un `title` su un iframe diventa un suggerimento su tutto il contenuto: usa `aria-label`. `ask()` restituisce `null` anche per il testo vuoto: per «valore predefinito» usa una parola («auto»).
- Un `verifica.py` lanciato mentre si modificano i file contamina il risultato: un giro alla volta su un worktree fermo.
- Il proxy GitHub del cloud può ignorare la cancellazione di rami remoti: Federico li cancella da https://github.com/Freeesta/mzlab/branches.
