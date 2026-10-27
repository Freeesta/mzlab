# Flow: usabilità del modello «MSn infusione» (studio e lavori)

Studio della prova di Federico in Flow web (modello «Fragmentation tree of a standard», un'infusione Orbitrap MS1–MS8): stato del codice, terminale, nuovo layout, decisioni aperte e prompt dei lavori. Testi del Flow solo in inglese; logica in Rust, JS colla.

## 1. Stato reale (punti 1–8)

- **Grafo non visibile (1)**: la corsa web MSn (`flow-web-msn.js`, `one()`) scrive `<istanza>.json` (msngrafo/1), `.msp`, `.csv` in OPFS e li restituisce in `res.files`; `flow-web-ui.js` (`F.webRunUi`) ne fa solo link di scaricamento nella riga del passo.
- (1, segue) `F.webAfterRun` (`flow-run-end.js`) mette in `S.results` solo la tabella `series.mzfeat`; la sessione live (`flow-live-jobs.js`, `msn`) tiene i testi in `live.files` e nessuno li porta in `S.results`. La vista esiste già (`F.openMsnGraph`, `flow-views-msn.js` + `flow-msn-graph.js`, aperta da `flow-report.js` per `formato: "msngrafo/1"`) ma non è collegata: `e2e_flow_msn.py:118` la apre a mano.
- **Descrizioni dei campi (2)**: vengono dal descrittore `step/1` in Rust (`description` di ogni parametro, `doc` dell'algoritmo); `flow-form.js:46` e `:57` le scrivono come `span.fl-ds` sotto ogni campo. Un solo punto da cambiare per tutti i moduli; le stime hanno già un «ⓘ» con `title` (`flow-form.js:42`).
- **Raw files (3)**: il nodo è un ingresso del piano (`plan.inputs`, `F.INPUT_NAMES.raw`); la riga «N files in Data: …» è `flow-inspect.js:19`, i pulsanti Add files/Add a folder `flow-import.js:86-90` (chiamano i caricatori di Data). I file vengono da `E.files`/`ST.files` (Data); niente tipo di acquisizione, niente esportazione mzML.
- (3, segue) In Data esistono già: `showMethod` (`eventi.js:442`, rotta `api/method`: tipo, polarità, strumento, finestra, transizioni), la barra informativa del banco (`banco.js:349`: `hdr` intestazione della scansione, `lst` lista delle scansioni, `fil` file e strumento, `alb` albero dei precursori `infusione-albero.js`, `gra` grafo `infusione-grafo.js`). Il wasm `mzlab-lettori-wasm` ha `to_mzml` (`lib.rs:238`); il passo Rust `convert.mzml` c'è solo nativo.
- **Disposizione (4)**: `F.canvasModel` (`flow-fix.js:118`) disegna un arco solo dai riferimenti `inst.in`; i passi MSn prendono l'infusione «aperta in Data» (`F.webMsn.infusion`) senza un `file:` nel piano, quindi Raw files resta scollegato e nella stessa colonna. In Simple esistono già i gruppi (`F.BASE`: più passi in una carta), oggi solo per la base.
- **Nomi (5)**: `F.nameOf` (`flow-names.js:84`) ricade sull'id quando manca in `F.NAMES`; mancano `spectra.msn_graph`, `msn.library_export`, `msn.table_export` (e `chrom.uv_peaks`, `features.table`, `quant.calibrate`, `spectra.ms2`, `table.drift`, `table.kmd`, `table.pca`, `table.stats`, `tp.dig_lr`). Anche la riga di stato live usa `nameOf`, per questo mostra gli id.
- **Inspector (6)**: `flow-shell.js` `--insp` 340 px (240–900, separatore trascinabile e da tastiera); la disposizione `F.layout` ha passi fissi W 280, H 96.
- **Run on CLI (7)**: `flow-run-ui.js` copia lo script di `api/flow/command` (Rust `mzlab_core::flow::command`: here-document con tutto il piano + `mzlab-cli flow run plan.json --in "$MZLAB_IN" --out …`); la freccia ▾ apre una `textarea` modificabile con la cartella. Conferma «Copied» solo nella riga di stato (toast). L'esecutore CLI accetta già `--set ISTANZA.NOME=VALORE` (`mzlab-cli/src/flow_run.rs:68`, `flow_apply.rs:58 apply_set`).
- **Barra (8)**: «Open» apre un PIANO `mzflow/2` (`#fl-file` della List view), non file di dati. «Live» (`flow-live-bar.js`, acceso di norma) fa partire da soli i passi a ogni modifica del piano: è il motivo per cui i nodi erano già «done»; «Pause» ferma i lavori nuovi, «Run now» appare solo se un passo chiede più memoria. Canvas/List/Plan sono una riga a sé (`flx-views`) sopra la barra.
- **Condivisione con Data**: `flow-msn-graph.js` carica già `infusione-grafo-svg.js` e `infusione-grafo-css.js` dell'app (stesso SVG, `msngrafo-vista.js` disegna albero e radiale senza globali): il «View» del Flow non riscrive nulla, collega. Mancano nel Flow il pannello dello ione (`infusione-grafo-nodo.js`) e il passaggio albero/radiale.
- Il «null» dopo «done in 8.7 s» è di FLOW-INFUSIONE-RAW (caricamento `.raw` ed errori del nodo Raw files): fuori da questi lavori.

## 2. Terminale (punto 7): raccomandazione

- **(i) Browser**: il core è wasm ma `mzlab-cli` no (dipende da file e processi); il motore Flow del browser è la sessione live (`flow-live*.js`) con lo scheduler Rust. Un terminale «vero» che esegue `mzlab-cli flow run` nel browser vorrebbe dire portare in wasm l'esecutore della CLI: non serve, la corsa web fa già gli stessi passi con gli stessi `.wasm`.
- (i, costo) xterm.js (`@xterm/xterm` 5.x, MIT): circa 250–300 KB minificato più CSS (stima, ≈70 KB gzip), compatibile con la CSP; serve solo se c'è un flusso ANSI da un processo vero. Nuova dipendenza: decide Federico; qui non serve.
- **(ii) App desktop**: un pty reale vuole `portable-pty` (MIT) o `tauri-plugin-shell` + xterm.js: due-tre dipendenze nuove, una shell arbitraria dentro la pagina (capacità Tauri, CSP) e differenze Mac/Windows. Fattibile, sproporzionato: l'app esegue già il piano nel processo (`flow_run.rs`, `mzlab_cli::flow::run_plan_notify`).
- **(iii) Raccomandazione — pannello «Terminal» fatto in casa, reale sul motore**: `<pre>` dell'uscita + UNA riga di comando modificabile (`input` monospaziato, non `contenteditable`, per tastiera e lettori di schermo), «Copy» (copia subito all'apertura, «Copied ✓» visibile nel pannello), «Run here», «Reset», «Show script».
- (iii, segue) La riga mostrata è corta: `mzlab-cli flow run plan.json --in <cartella> --set graph.ppm=5 …` (un `--set` per parametro diverso dal default), lo script completo resta dietro «Show script». «Run here» manda la riga a una rotta Rust nuova (`api/flow/line`: analizza la riga con lo stesso `apply_set` della CLI spostato in `mzlab-core`, restituisce il piano o l'errore `err.cli.*`) e poi la esegue con l'esecutore della pagina (browser: `F.runners.web`; app: `flow/run`); l'uscita sono gli eventi del passo, una riga ciascuno.
- (iii, costo) zero dipendenze; ~1 file JS (≤250 righe) + ~150 righe Rust con test di parità (riga → piano uguale a quello di `mzlab-cli flow run --set`). È «reale» nel senso che conta: stessi comandi, stesso analizzatore, stesso motore; `--in/--out` nel browser sono ignorati con una riga che lo dice (i file sono quelli di Data).

## 3. Layout proposto

Schizzo (prova HTML e foto Playwright fatte fuori dal repository):

```
[Flow] (Canvas|List) [Templates▾] [+ Add step] (Simple|Advanced) ↶ ↷ ⤢   ✓ Ready  [⚠ 0] [▶ Run|▾] [>_ Terminal] [⋯]
┌──────────── canvas (≈50 %) ───────────────────────────┐┃┌──────── inspector (≈50 %) ────────────┐
│ ┌Raw files──────┐   ┌Fragmentation graph┐   ┌Export─────────┐ │ Fragmentation graph                   │
│ │1 file·Orbitrap│──▶│timolol·[M+H]+     │─┬▶│Library · Table│ │ Builds the MSn tree of one infusion.  │
│ │Infusion·MS1–8 │   │✓ done 4.4 s       │ │ │✓ 2 files ⤓    │ │ Molecule      [timolol        ] ⓘ     │
│ └───────────────┘   └───────────────────┘ │ └───────────────┘ │ Root ion      [[M+H]+ 317.1642] ⓘ     │
│                                           └▶┌View graph─────┐ │ Sub-formula window (ppm) [5]    ⓘ     │
│                                             │▸ Open explorer│ │ ▸ More (6)      ▸ Results             │
└───────────────────────────────────────────────────────────────┘└───────────────────────────────────────┘
```

| Oggi (barra) | Proposta |
|---|---|
| Canvas / List view / Plan (JSON), riga a sé | interruttore piccolo `Canvas|List` nella barra; «Plan (JSON)» nel menu ⋯ (solo Advanced) |
| Flow, Templates, Add analysis | restano («Add analysis» → «+ Add step») |
| Simple / Advanced | resta |
| Undo, Redo, Fit | solo icone con `aria-label` e `title` (bersagli ≥ 24 px, 40 al tocco) |
| Checklist | icona ⚠ con il numero dei problemi, accanto allo stato |
| Open | nel menu ⋯ come «Open plan…» (insieme a «Download plan») |
| Live, Pause, Run now | Run diventa pulsante diviso `▶ Run|▾`: nel ▾ la casella «Auto-run on changes» (= Live); «Pause» compare solo mentre lavori live sono attivi; «Run now» resta com'è (solo quando serve) |
| Run on CLI, ▾ | un pulsante «>_ Terminal» (pannello del §2) |
| Ready to run | «✓ Ready» / «⚠ 2 to fix» |

- Da 17 controlli su due righe a 12 su una (a 1280 px); sotto 900 px la barra va a capo senza scorrimento orizzontale.
- Inspector: parte dal 50 % della larghezza (`--insp: 50%`, limiti 240 px–70 %), il separatore resta; con ≤ 5 carte la disposizione si compatta (W 230, H 84).
- Grafo MSn: Raw files → Fragmentation graph → {Export, View graph}. «Export» è un gruppo Simple dei due passi d'esportazione (Advanced li mostra separati); «View graph» è una carta di vista, non un passo del piano (la CLI non cambia): appare accanto a ogni passo che ha una vista registrata e si accende quando il risultato esiste.
- Descrizioni: sotto il campo resta solo l'etichetta (≤ 8 parole, unità tra parentesi); la frase va in un «ⓘ» che apre/chiude una nota vicino al campo (clic, Invio, Esc; `aria-describedby` sul campo): niente `title` come unico veicolo (tastiera e tocco, WCAG 1.4.13). Una frase sola, ≤ 20 parole.

## 4. Decisioni di Federico

1. **Nomi** (gli id interni restano): `spectra.msn_graph` «Fragmentation graph» (alt. «MSn tree», «Build fragmentation tree»); `msn.library_export` «Save as library» (alt. «Library export (MSP)»); `msn.table_export` «Save as tables» (alt. «Table export (CSV)»); gruppo «Export» (alt. «Save results»); carta «View graph» (alt. «Explore graph»). Gli altri nove senza nome li propone DEV-D dal titolo del descrittore.
2. **Live**: dentro Run ▾ come «Auto-run on changes», acceso di norma (consigliato) / spento di norma / tolto.
3. **Open**: nel menu ⋯ (consigliato) / tolto (il piano si apre anche trascinando il file).
4. **View** come carta di vista fuori dal piano (consigliato) / passo vero `view.msn_graph` (cambia CLI e JSON).
5. **Terminale**: pannello fatto in casa (consigliato, nessuna dipendenza) / xterm.js / pty reale nell'app.
6. **Titoli Rust**: accorciare i `title` dei descrittori oltre 8 parole (tocca la CLI `--help`) oppure accorciarli solo nell'interfaccia.
7. **Inspector** al 50 % sempre o solo per piani con ≤ 5 carte.

## 5. Prompt dei lavori

Ordine: A → B → C → D → E → F. A, B, C ed E non toccano gli stessi file e possono andare in parallelo; D dopo A; F dopo E. C dopo l'unione di FLOW-INFUSIONE-RAW (stesso nodo Raw files). I file sorgente sono disgiunti; i test e2e comuni (`e2e_flow.py`, `e2e_flow_msn.py`) si toccano solo nelle righe del proprio lavoro.

### DEV-A — Il grafo si apre dopo la corsa
- **Ruolo**: DEV Sonnet, Flow web (amministratore, solo inglese). Leggi `docs/agenti/flow-web.md` e questo file §1.
- **Contesto**: la corsa MSn (Run e Live) è «done» ma il grafo msngrafo/1 non si apre: i testi restano in `res.files` (link) e `live.files`; `F.openMsnGraph` esiste ed è scollegato.
- **Perimetro**: `flow/lazy/flow-run-end.js`, `flow-web-ui.js`, `flow-live-jobs.js` (solo il lavoro `msn`), `flow-views-msn.js`, `tests_e2e/e2e_flow_msn.py`. Non toccare `flow-shell.js`, `flow-names.js`, `flow-fix.js`, il nodo Raw files.
- **Lavoro**: (1) `F.webMsnAfterRun(S, files)` in `flow-run-end.js`: ogni testo MSn entra in `S.results[istanza]` come elemento `{type, label, text}` (`.json` → vista del grafo, `.msp` → library, `.csv` → table), sostituendo quello con la stessa etichetta; (2) chiamalo da `F.webRunUi` e quando un lavoro `msn` del live finisce; (3) a fine corsa si apre da solo il grafo (come la tabella delle feature), col fuoco sul titolo della vista; (4) registra la vista del grafo per il suo tipo in `flow-views-msn.js` se `flow-report.js` non basta.
- **Accettazione**: dopo Run o Live il pannello del nodo del grafo ha «Results» con il grafo apribile e i due file scaricabili; il grafo si apre da solo; nessun `window.MZLAB_LR_MISS`.
- **Prove**: `python3 tests_e2e/e2e_flow_msn.py` (togli l'apertura a mano della riga 118, verifica `.ig-wrap svg` dopo Run e dopo una modifica in Live), `tsc` del progetto `tools/tipi/`, `python3 tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-msn-risultati`, una riga in `docs/agenti/flow-web.md`; messaggio al COO (≤5 righe, SHA a CI verde).

### DEV-B — Descrizioni nei suggerimenti
- **Ruolo**: DEV Sonnet, modulo dei parametri del Flow. Leggi `docs/agenti/flow-grafica.md` (moduli) e §3 di questo file.
- **Contesto**: sotto ogni campo c'è una frase (`flow-form.js:46`, `:57`); Federico vuole etichette brevi e la frase nel suggerimento, per tutti i moduli.
- **Perimetro**: `flow/lazy/flow-form.js`, `flow-msn-molecule.js` (solo le sue note), CSS di `flow-ui.js` (`.fl-ds`), un test pytest nuovo, i test e2e che cercano `.fl-ds`. Non toccare i descrittori Rust né `flow-inspect.js`.
- **Lavoro**: (1) aiuto `F.tip(id, text)`: pulsante «ⓘ» 24×24 (40 al tocco) con `aria-label` «About <etichetta>», `aria-expanded`, che apre una nota `role="note"` sotto il campo, legata con `aria-describedby`; chiusura con Esc, nuovo clic o fuoco altrove; (2) `description` e `doc` dell'algoritmo passano da `span.fl-ds` al suggerimento (nessuna frase visibile di norma).
- **Lavoro (segue)**: (3) l'«ⓘ» della stima usa lo stesso aiuto; (4) regola di scrittura nel file d'area: etichetta ≤ 8 parole, suggerimento una frase ≤ 20 parole; un test che elenca i titoli dei descrittori oltre 8 parole con una lista nota che può solo scendere.
- **Accettazione**: nessuna `.fl-ds` visibile senza clic; ogni campo con descrizione ha il suo «ⓘ» raggiungibile da tastiera; `e2e_axe_grafici` e axe del Flow senza nuove violazioni.
- **Prove**: `python3 tests_e2e/e2e_flow.py`, `e2e_flow_liste.py`, `e2e_flow_import.py`, pytest nuovo, `tsc`, `tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-tooltip`; riga in `flow-grafica.md`; messaggio al COO.

### DEV-C — Raw files: tabella, tipo di acquisizione, esplora, mzML
- **Ruolo**: DEV Sonnet, nodo Raw files del Flow. Leggi `docs/agenti/flow-grafica.md`, `docs/agenti/dati.md` (barra del banco) e §1 qui.
- **Contesto**: il nodo mostra «N files in Data: …» (`flow-inspect.js:19`); Data ha già `api/method`, la barra del banco (`banco.js:349`: `hdr`, `lst`, `fil`, `alb`) e `to_mzml` nel wasm dei lettori. FLOW-INFUSIONE-RAW possiede il caricamento `.raw` e i messaggi d'errore del nodo: parti dopo la sua unione e non toccarli.
- **Perimetro**: file nuovo `flow/lazy/flow-raw-files.js` (in `LATE` di `flow-tool.js`), la riga 19 di `flow-inspect.js`, un'esportazione minima in `banco.js` (funzione che disegna un pannello `fil|lst|alb` per il file `k` in un contenitore dato, senza copiare codice), Rust solo se il tipo di acquisizione non è già calcolato.
- **Lavoro**: (1) tabella ordinata (nome, tipo: LC-MS / infusione / DDA / DIA / MSn / SRM-MRM, polarità, strumento, livelli MS, dimensione) da `ST.files` + `api/method`; se la classificazione non esiste in Rust, aggiungila in `mzlab-core` come campo della rotta esistente, con test di parità; (2) per riga un solo pulsante «Explore» che apre sotto la tabella le schede File and instrument / Scans / Precursor tree riusando la funzione di `banco.js`; (3) «Export mzML»: un `.mzML` aperto si scarica così com'è; un formato del fornitore passa da `to_mzml` nel worker dei lettori.
- **Accettazione**: con l'infusione Orbitrap la riga dice «Infusion · MSn (MS1–MS8) · positive · Orbitrap Fusion»; l'esplorazione mostra gli stessi dati di Data; il mzML scaricato si riapre in Data.
- **Prove**: e2e nuovo `tests_e2e/e2e_flow_rawfiles.py` (file sintetici di `tools/dati_sintetici.py`), parità Rust se aggiunta, `tsc`, `tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-raw-files`; righe in `flow-grafica.md` e `dati.md`; messaggio al COO.

### DEV-D — Nomi umani, Export e View, collegamento con Data
- **Ruolo**: DEV Sonnet, carte del Flow. Leggi `flow-grafica.md` e §1, §3, §4 qui; parti dopo l'unione di DEV-A.
- **Contesto**: `F.nameOf` mostra gli id dei passi senza nome; Raw files non è collegato ai passi MSn; Export e View non esistono come carte.
- **Perimetro**: `flow/lazy/flow-names.js`, `flow-fix.js` (`canvasModel`, `layout`), `flow-canvas.js` (solo la carta di vista), `flow-msn-graph.js`, i test e2e del Flow che leggono i titoli delle carte. Non toccare `flow-shell.js` né `flow-run-*.js`.
- **Lavoro**: (1) `F.NAMES` per i 12 passi senza nome (nomi del §4 dopo la scelta di Federico; mai la parola «formula» da sola); (2) gruppi Simple anche fuori dalla base (`F.GROUPS`: «Export» = `msn.library_export` + `msn.table_export`).
- **Lavoro (segue)**: (3) arco da Raw files ai passi che leggono l'infusione di Data anche senza `file:` nel piano; (4) carta di vista «View graph» (fuori dal piano) accanto a un passo con vista registrata, spenta finché non c'è risultato, che apre `F.openMsnGraph` grande; (5) nella vista del grafo: albero/radiale e pannello dello ione dai moduli di Data (`msngrafo-vista.js`, `infusione-grafo-nodo.js`) e «Open in Data» che apre la scheda del grafo sullo stesso file: nessuna copia di codice; (6) disposizione compatta con ≤ 5 carte.
- **Accettazione**: il modello MSn si disegna Raw files → Fragmentation graph → {Export, View graph}; nessun id visibile nelle carte né nella riga di stato; Advanced mostra i passi separati; il piano `mzflow/2` esportato è identico a prima.
- **Prove**: `e2e_flow.py`, `e2e_flow_msn.py`, `e2e_flow_web.py`, `flow_rule_tests.rs` invariato, `tsc`, `tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-nomi-viste`; righe in `flow-grafica.md`; messaggio al COO.

### DEV-E — Barra compatta e inspector a metà
- **Ruolo**: DEV Sonnet, guscio del Flow. Leggi `flow-grafica.md`, `flow-live.md` e §3, §4 qui.
- **Contesto**: la barra ha 17 controlli su due righe; «Open» apre un piano; «Live» è l'esecuzione automatica.
- **Perimetro**: `flow/lazy/flow-shell.js`, `flow-shell-css.js`, `flow-live-bar.js`, `flow/js/flow-tool.js` (scheletro della barra), i test e2e che usano `data-tab`, `#flx-live-on`, «Open», «Run on CLI». Non toccare `flow-run-ui.js` (è di DEV-F): il pulsante «>_ Terminal» chiama `actions().toggle` come la freccia di oggi e tiene l'id `flx-cli-show`.
- **Lavoro**: (1) `Canvas|List` piccolo nella barra, «Plan (JSON)» e «Open plan…» / «Download plan» nel menu ⋯; (2) Undo/Redo/Fit solo icone; Checklist come ⚠ con il numero; (3) Run diviso con ▾: «Auto-run on changes» (stato e id del Live di oggi), «Pause» visibile solo con lavori live attivi; (4) un solo «>_ Terminal» al posto di «Run on CLI» e ▾; (5) `--insp` iniziale 50 % (240 px–70 %), il valore resta per la sessione.
- **Accettazione**: una riga a 1280 px, nessuno scorrimento orizzontale a 360 px, ogni bersaglio ≥ 24 px (40 al tocco), tastiera completa, nessun testo perso per i lettori di schermo.
- **Prove**: `e2e_flow.py`, `e2e_flow_live.py`, `e2e_flow_annulla.py`, `e2e_flow_web.py`, `e2e_axe_grafici`, `tsc`, `tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-barra`; righe in `flow-grafica.md` e `flow-live.md`; messaggio al COO.

### DEV-F — Pannello Terminal
- **Ruolo**: DEV Sonnet, Rust + colla JS. Leggi `docs/agenti/rust-flow.md`, `cli.md`, §2 qui; parti dopo l'unione di DEV-E.
- **Contesto**: `api/flow/command` dà uno script lungo; la CLI accetta `--set`; `apply_set` sta in `mzlab-cli/src/flow_apply.rs`.
- **Perimetro**: `crates/mzlab-core/src/flow/` (`command.rs` + modulo nuovo della riga), `crates/mzlab-cli/src/flow_apply.rs` (diventa un rimando al core), il ponte wasm e la rotta dell'app desktop della sola `flow/line`, `flow/lazy/flow-run-ui.js`, file nuovo `flow/lazy/flow-terminal.js`, test. Non toccare `flow-shell.js`.
- **Lavoro**: (1) in Rust la riga corta (`mzlab-cli flow run plan.json --in <cartella> --set …`, un `--set` per parametro diverso dal default) e la rotta `flow/line` (riga + piano → piano con i `--set` applicati o errore `err.cli.*`), con lo stesso `apply_set` della CLI; (2) pannello: uscita in `<pre aria-live="polite">`, riga in `input` monospaziato, copia all'apertura con «Copied ✓» nel pannello, «Run here» (browser: `F.runners.web`; app: `flow/run`), «Reset», «Show script»; uscita = una riga per evento di passo con i nomi umani.
- **Lavoro (segue)**: (3) `--in/--out` nel browser: riga che dice che si usano i file di Data.
- **Accettazione**: modificare `--set graph.ppm=5` e premere «Run here» dà lo stesso piano che `mzlab-cli flow run --set graph.ppm=5`; un `--set` sbagliato mostra l'errore della CLI; tutto da tastiera.
- **Prove**: test Rust di parità riga↔CLI, `cargo test -p mzlab-core -p mzlab-cli`, e2e nuovo `tests_e2e/e2e_flow_terminal.py`, `tsc`, `tools/verifica.py --cambiati`.
- **Consegna**: PR `claude/flow-terminal`; righe in `rust-flow.md`, `cli.md`, `flow-grafica.md`; messaggio al COO.
