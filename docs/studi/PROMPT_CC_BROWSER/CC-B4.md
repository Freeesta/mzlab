# CC-B4 — Avvio automatico dopo i TP, in un worker a bassa priorità, con stato visibile

Prompt pronto per una chat DEV: copia tutto ciò che segue la riga «---». Parte dopo il merge di CC-B2 e CC-B3; in parallelo con CC-B5 e CC-B6.

---

Pacchetto CC-B4 dello studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (leggi §5 «Prestazioni» e «Dati», §7 punti 4-6, §8). File d'area: `docs/agenti/reattivita.md`, `docs/agenti/mzfeat-wasm.md` (worker e caricamento dei `.wasm` di mzFinder), `docs/agenti/memoria.md`, `docs/agenti/rust-desktop.md` (motore nativo).

## Obiettivo
Quando Tabui ha finito di calcolare i TP e il genitore ha uno SMILES, parte da solo il calcolo di reattività del genitore in un worker separato; i TP hanno sempre la precedenza; il pannello di CC-B3 mostra lo stato e poi i risultati.

## Da fare
1. **Worker e lazy-loading**: `TP_Mine/files/chem-worker.js`: carica il `.wasm` `chem` (manifesto come `grafo`) solo alla prima richiesta, chiama `reactivity_json`, risponde `{ok, json}` o `{error, error_key, params}`. Collocare i file dell'interfaccia in `TP_Mine/lazy/` (con caricamento tramite `TPLAZY.load`) invece di caricamento fisso in `TP_Mine/js/` per rispettare il tetto di avvio di 1.55 MB. Un solo calcolo alla volta; una richiesta nuova annulla quella in coda (terminare il worker se è a metà e ricrearlo).
2. **Quando parte**: dopo l'evento o la promessa di fine del calcolo dei TP in LR (`20_tpmine.js`) e in HR (`21_tpmine_hr.js`) (trova i punti con il `cercatore`); solo con `uipAdminOn()` e SMILES valido; mai durante un calcolo dei TP (se l'utente rilancia i TP, il calcolo di reattività si ferma e riparte dopo); con `requestIdleCallback` o un ritardo breve per non contendere la CPU.
3. **Cache** nella sessione (stesso SMILES canonico + stessa `version()` del motore = nessun nuovo calcolo); se serve una cache durevole, segui le regole dei dati utente di `docs/agenti/dati.md` senza nuovi nomi di database (chiedi al Coordinatore prima).
4. **Stato** nel pannello: «waiting for TPs», «computing… (step N of 4: geometry, neutral, cation, anion)» se il motore dà i passi, «done in X s», «failed» con il motivo e il pulsante «Retry»; un pulsante «Recompute» con opzioni in lingua inglese (es. "Solvent: Water / None", "Conformers: 8").
5. **App desktop**: stessa sequenza, ma con la funzione nativa (rotta di CC-B2) al posto del worker.

## Prove
- `tests_e2e/e2e_reattivita_auto.py`: con `qqq.admin=1` e un file d'esempio di `mzlab/web/esempi/` con genitore (o un genitore inserito nel test), dopo i TP lo stato passa da «waiting» a «done» e la tabella ha i valori; rilanciare i TP durante il calcolo lo ferma; senza `qqq.admin` il `.wasm` `chem` non viene scaricato (controllo delle richieste di rete).
- Misura: il tempo di calcolo dei TP con worker di reattività attivo in background non deve aumentare di oltre il 5% (o > 200 ms) rispetto al tempo senza calcolo (studio §7.6), scritta nel documento d'area; `window.MZLAB_LR_MISS` vuoto.

## Atteso
Calcolo automatico, non invadente, con stato chiaro; nessun effetto per chi non è amministratore.

## Regole comuni (valgono per tutta la chat)
- Sei una chat DEV di mzLab (repository `Freeesta/mzlab`). Il titolo della chat inizia con «mzLab» (per esempio «mzLab CC-B1 geometria 3D in Rust»). Il Coordinatore è la sessione che ti ha mandato questo prompt (il suo id è nel messaggio di avvio): a fine lavoro UN messaggio con `send_message` (al massimo 5 righe: PR, CI, pages, peso dei `.wasm` toccati, cosa resta). Se manca l'id, scrivi lo stesso riassunto nella descrizione della PR.
- AGENTS.md è nel tuo contesto: rispettalo. Leggi SOLO lo studio `docs/studi/CC_MOTORE_NEL_BROWSER.md` (§ indicati), il file d'area `docs/agenti/reattivita.md` (lo crea CC-B1) e i file elencati nel prompt. Per trovare codice: `python3 tools/indice.py` poi `grep -n`, oppure il subagente `cercatore`. Letture a intervalli, niente file interi grandi.
- **Il programma propone, il chimico decide**: ogni valore di reattività si chiama «Prev.» (previsione); nessuna funzione assegna un livello di Schymanski; nulla riordina, esclude o toglie le classi di posizioni di `posiz/1`.
- **Visibilità**: solo con `uipAdminOn()` (`mzlab/web/settings.js`), in Tabui (mzFinder) e Attributio; niente nella vista Dati, nel Disegno degli studenti, in Teoria o nella relazione. Un e2e controlla che senza `qqq.admin` non ci siano pannello né download del `.wasm` `chem`.
- **Rust per ogni calcolo** (`crates/mzlab-chem`); JS solo per l'interfaccia; Python solo per strumenti e test (`tools/`), mai nel sito. Codice e commenti in inglese. I testi di Tabui sono in inglese senza chiavi `lang/` (eccezione di `docs/agenti/mzfinder.md`).
- **Codice ripreso da sci-form** (MIT, github.com/jigonzalez930209/sci-form, versione 0.15.2): solo i file che servono, copiati nel crate (mai come dipendenza crates.io), ognuno con in testa provenienza (versione e percorso d'origine) e «MIT, see LICENSE-sci-form»; `crates/mzlab-chem/LICENSE-sci-form` con il testo della licenza. Ogni pezzo ripreso entra solo con una prova di parità che lo copre. Unica dipendenza nuova ammessa: `nalgebra` (Apache-2.0, `default-features = false` se basta).
- **Tetti**: il `.wasm` `chem` ha un tetto proprio in `tools/build_feat_wasm.sh` = 1,3 × misura dopo `wasm-opt -Oz`, con la misura nel commento accanto e nel documento d'area; obiettivo ≤ 1 300 000 B (studio §7). Non toccare il tetto comune degli altri `.wasm`, `.github/workflows/` né `.claude/settings.json`.
- Non toccare: `mzlab/web/teoria/`, il motore (`crates/mzlab-core`, `crates/mzlab-wasm`), i `.wasm` diversi da quelli del prompt, i pacchetti AT/MZ di Attributio, i nomi dei dati utente (`qqq_lab`, `qqq.*`).
- **Documenti**: le righe del pacchetto vanno in `docs/agenti/reattivita.md` (≤ 25 KB, righe ≤ 600 caratteri, stato attuale senza «ora/prima» né numeri di PR); la riga di AGENTS.md §5 la aggiunge solo CC-B1. Aggiorna la riga del tuo pacchetto nella tabella di §9 dello studio (stato, misure).
- Composti di prova: solo strutture pubbliche (studio §6); MAI dati del repository privato copiati nel pubblico; nessun cognome di `nomi_vietati.txt` (`python3 -m pytest -q tests/test_esempi_anonimi.py`).
- Funzioni ≤ 80 righe (il codice ripreso si spezza se serve); test in file a sé (`crates/mzlab-chem/tests/*.rs`); rustfmt solo sui file che tocchi (mai `cargo fmt` del repository); `python3 tools/controlla.py` verde; un file JS nuovo con `// @ts-check` in `tools/tipi/pagina.json` e 0 errori; ogni grafico nuovo con la sua tabella `TDATI`, colori da variabili CSS (Okabe-Ito), tastiera e `aria-label`, bersagli ≥ 24×24 px, zoom e rotazione senza trascinamento.
- Prove: `cargo test --release -p mzlab-chem` (e dei crate toccati), `python3 tools/verifica.py --cambiati` (in cloud: `docs/agenti/ambiente-cloud.md`; prima degli e2e di mzFinder `python3 tools/build_site.py`, poi cancella `site/`), un e2e nel browser se tocchi la pagina (un front end non provato nel browser è rotto), `window.MZLAB_LR_MISS` vuoto.
- Flusso: ramo `claude/<argomento>` da `origin/main`, PR non bozza, merge con merge commit quando la CI della PR è verde, poi controlla che il workflow «pages» del commit su `main` sia riuscito. Un criterio dello studio §7 non raggiunto o una scelta tra due comportamenti: PR aperta con la spiegazione e domanda al Coordinatore, niente merge.
- Se il contesto supera 200k token a fine PR: blocco di consegna al Coordinatore (AGENTS.md §2) e fermati.
