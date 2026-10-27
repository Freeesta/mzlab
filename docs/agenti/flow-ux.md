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
[Flow] (Canvas|List) [Templates▾] [+ Add step] (Simple|Advanced) ↶ ↷ ⤢   ✓ Ready  [⚠ 0] [Auto-run] [▶ Run] [>_ Terminal] [⋯]
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
| Open | tolto dalla barra: il piano si apre dalla List view; «Download plan» nel menu ⋯ |
| Live, Pause, Run now | interruttore piccolo «Auto-run» (= Live) accanto a «▶ Run»; «Pause» compare solo mentre Auto-run ha lavori attivi; «Run now» resta com'è (solo quando serve) |
| Run on CLI, ▾ | un pulsante «>_ Terminal» (pannello del §2) |
| Ready to run | «✓ Ready» / «⚠ 2 to fix» |

- Da 17 controlli su due righe a 12 su una (a 1280 px); sotto 900 px la barra va a capo senza scorrimento orizzontale.
- Inspector: parte dal 50 % della larghezza (`--insp: 50%`, limiti 240 px–70 %), il separatore resta; con ≤ 5 carte la disposizione si compatta (W 230, H 84).
- Grafo MSn: Raw files → Fragmentation graph → {Export, View graph}. «Export» è un gruppo Simple dei due passi d'esportazione (Advanced li mostra separati); «View graph» è una carta di vista, non un passo del piano (la CLI non cambia): appare accanto a ogni passo che ha una vista registrata e si accende quando il risultato esiste.
- Descrizioni: sotto il campo resta solo l'etichetta (≤ 8 parole, unità tra parentesi); la frase va in un «ⓘ» che apre/chiude una nota vicino al campo (clic, Invio, Esc; `aria-describedby` sul campo): niente `title` come unico veicolo (tastiera e tocco, WCAG 1.4.13). Una frase sola, ≤ 20 parole.

## 4. Decisioni

**Prese** (Federico, via COO): nomi `spectra.msn_graph` «Fragmentation graph», `msn.library_export` «Save as library», `msn.table_export` «Save as tables», gruppo «Export», carta «View graph» (id invariati); «Open» tolto dalla barra (il piano si apre dalla List view); «Live» → interruttore piccolo «Auto-run»; Canvas/List interruttore piccolo nella barra; inspector da metà pagina; Export e View a destra del grafo; terminale fatto in casa (niente xterm.js né pty) con «Copied» visibile, riga modificabile e «Run here».

**Regola dei testi** (Federico, per tutta l'interfaccia): pochi testi; niente paragrafi esplicativi dove l'interfaccia si capisce da sola; tooltip SOLO per dire cosa fa un campo o cosa inserirvi, mai opinioni, avvisi o cautele; un testo che non dice cosa fa o cosa inserire si toglie (con la sua chiave i18n).

**Prese in seguito**: Auto-run acceso di norma come oggi; «View graph» è una carta fuori dal piano (CLI e JSON invariati); B tocca solo l'interfaccia: testi brevi in `x-label`/`x-tip` dei descrittori, `title`/`description` (aiuto della CLI) invariati; tooltip di Auto-run «Run again when an input or setting changes».

## 5. Prompt dei lavori

Un file per lavoro in `docs/agenti/prompt-flow-ux/` (formato a 7 punti, ramo e dipendenze in testa, regole comuni e regola dei testi incluse):

| Lavoro | File | Ramo | Dipende da |
|---|---|---|---|
| A — il grafo si apre dopo la corsa | `A.md` | `claude/flow-msn-risultati` | — |
| B — etichette corte, descrizioni nei tooltip | `B.md` | `claude/flow-tooltip` | — |
| C — Raw files: tabella, tipo, esplora, mzML | `C.md` | `claude/flow-raw-files` | FLOW-INFUSIONE-RAW unita |
| D — nomi umani, Export e View graph | `D.md` | `claude/flow-nomi-viste` | A |
| E — barra compatta, inspector a metà | `E.md` | `claude/flow-barra` | — |
| F — pannello Terminal | `F.md` | `claude/flow-terminal` | E |

A, B, C ed E partono insieme; D dopo A; F dopo E. I file sorgente sono disgiunti; i test e2e comuni (`e2e_flow.py`, `e2e_flow_msn.py`) si toccano solo nelle righe del proprio lavoro.
