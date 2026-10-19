# Studio «Flow, la grafica»: come si vede e come si usa la tela dei moduli

Obiettivo (Federico, 19/10/2026): una sezione «Flow» con moduli trascinabili e collegati uno dopo l'altro (lettura dei grezzi → mzML → picchi e cromatogrammi → workflow TP/screening → disegno → teoria), sul web o nell'app desktop. Questo studio decide **solo la grafica**: come si disegna, come si tocca, come i moduli si parlano A VISTA. Base: `main` a `2e64041`.

Parte da due studi paralleli e non li ripete:
- `docs/studi/WORKFLOW_MODULARI.md` (PR #1095): `passo/1` (porte tipizzate, parametri in JSON Schema), `mzflow/2` (il piano), `prov/1`, e soprattutto `workflow/COMUNICAZIONE.md` (regola di compatibilità, conversione proposta, ordine, cache, messaggi tela ↔ motore). **Qui c'è solo il modo di mostrarli.**
- `docs/studi/STRUTTURA_DATO.md` (PR #1098): il record `mol/1` della porta `structure`.

File dello studio (`docs/studi/flow_grafica/`, sonde: non vanno nel sito, `tools/indice.py` salta `docs/studi/`):
- `prototipo.html` + `flow-proto-model.js` (regole: 153 righe) + `flow-proto.js` (tela: 405 righe) + `flow-proto.css` (74 righe): catena di moduli finti con porte tipizzate, JS semplice, nessuna dipendenza;
- `misura.py`: 9 prove di accettazione + misure di fluidità con Playwright (§8).

**Segni**: **V** = verificato (letto in una fonte citata o misurato qui con il comando scritto); **I** = ipotesi o ricordo non riletto.

## 0. Verdetto
1. **Scriviamo la tela in casa**, in JS semplice: DOM per i nodi, un solo SVG per i fili. Il prototipo fa tutto il necessario in ~560 righe (13,6 KB compressi con gzip) e resta fluido con 200 nodi anche con la CPU rallentata 4 volte (V, §8). Nessuna libreria esistente rispetta insieme «niente bundler», accessibilità da tastiera e licenza permissiva (§2).
2. **Le regole non stanno nella tela**: compatibilità, conversioni, ordine, cache li decide il motore (`plan.check` in Rust, COMUNICAZIONE.md §5). La tela colora quello che il motore risponde. Nel prototipo le regole sono in un file a parte proprio per mostrare il confine.
3. **Ogni tipo di dato = colore + forma + lettera**: nessuna informazione sta solo nel colore (Okabe-Ito ha 8 colori, i tipi sono 15).
4. **Mentre trascini, ogni ingresso dice se va bene**: verde «compatibile», arancio «⇄ serve una conversione», grigio «no» con il motivo; il passo mancante si **propone** con un clic, mai inserito da solo.
5. **Tutto si fa anche senza trascinare** (clic-clic, tastiera, una vista «Elenco dei passi» come tabella dei dati del grafo), come chiedono WCAG 2.5.7 e la regola `TDATI` del progetto.
6. **Dove vive**: in Tabui (`TP_Mine/`), quindi solo in inglese e senza chiavi `lang/` (decisione da confermare, §11).

## 1. Cosa fanno i migliori, e cosa prendiamo
| Programma | Idea grafica | La prendiamo? | Fonte |
|---|---|---|---|
| KNIME | sotto ogni nodo un «semaforo»: rosso non configurato, giallo pronto, verde eseguito, croce errore | sì, ma con **forme** oltre al colore (cerchio vuoto, tratteggiato, pieno, quadrato) | KNIME user guide (V); dettaglio degli stati da un corso UGR (V, fonte secondaria) |
| KNIME | porte di forma diversa per dati, modelli, variabili | sì: forma per famiglia di tipo | KNIME user guide «Node Ports» (I: pagina non letta) |
| Unreal Blueprints | colore della porta per tipo; tirare un filo nel vuoto apre l'elenco dei soli nodi compatibili | sì, tutte e due (nel prototipo: rilascio nel vuoto → palette filtrata) | Epic, «Nodes in Unreal Engine» (V) |
| Unreal Blueprints | nodo di conversione cablato in mezzo (`AutowireConversionNode`) | sì, ma **proposto** e confermato con un clic | API UE 4.26 (V); quando lo inserisce da solo: non trovato (I) |
| Galaxy | il controllo «questa uscita va in questo ingresso?» sta nel codice dei terminali (`ConnectionAcceptable`); conversioni implicite tra formati, che non si concatenano (a→b→c no) | sì il controllo per porta; noi concateniamo (ricerca in ampiezza, COMUNICAZIONE.md §2) | commit e mailing list Galaxy (V) |
| Orange | i «canali» hanno nome e tipo; se due widget si possono collegare in più modi, una finestra chiede quale | sì: se più porte vanno bene, si chiede | orange-widget-base, `Input`/`Output` (V); la finestra stessa: ricordo (I) |
| n8n | ogni nodo mostra i dati di ingresso e di uscita dell'ultimo giro; dati «fissati» per provare senza ricalcolare | sì l'anteprima; il «fissa» = la nostra cache per chiave | ricordo (I) |
| Nuke | miniatura dell'immagine sul nodo («postage stamp») | sì per spettri e XIC, piccole e solo su richiesta | ricordo (I) |
| Houdini | nodi «sporchi» da ricalcolare quando cambia un parametro a monte | sì: stato «needs run» che scende ai discendenti | ricordo (I) |
| Node-RED | palette a sinistra, deploy con un pulsante, nodi raggruppati in sotto-flussi | sì palette e gruppi; il «deploy» è il nostro «Run» | ricordo (I) |
| ComfyUI | tela su `<canvas>` (LiteGraph): veloce con molti nodi, ma nodi e porte non sono elementi della pagina | **no** per noi: un lettore di schermo non li vede | README di `@comfyorg/litegraph` (V) |
| React Flow (xyflow) | nodi e fili raggiungibili con Tab, Invio seleziona, frecce spostano, Esc deseleziona; testi per lettori di schermo configurabili (`ariaLabelConfig`), annuncio dello spostamento in una regione live | sì, **tutto il modello da tastiera** (idea, non codice) | reactflow.dev, «Accessibility» e «AriaLabelConfig» (V) |

## 2. Librerie: usare o scrivere?
| Libreria | Licenza | Cosa serve | Verdetto |
|---|---|---|---|
| React Flow / Svelte Flow (xyflow) | MIT (I: file non riletto) | React o Svelte + bundler | no: framework e bundler vietati (Principio 2) |
| Rete.js v2 | MIT (plugin V; nucleo da riverificare nel LICENSE) | pacchetti TypeScript + un plugin di disegno (React/Vue/Angular/Svelte o Lit) | no: bundler e un framework di disegno |
| LiteGraph.js (e fork di ComfyUI) | MIT (V per l'originale; fork: da leggere) | nessuna dipendenza, disegna su canvas; copia di ComfyUI ~506 kB non compressa (V) | no: canvas = niente tastiera né lettori di schermo; pesante |
| Drawflow | MIT (V) | JS semplice, nessuna dipendenza (V) | idea sì, codice no: ultimo push 19/10/2024 (V), niente porte tipizzate né accessibilità |
| Baklava.js, Litegraph.ts, JointJS | Baklava MIT, JointJS MPL-2.0 (I) | Vue / TS / MPL non è nella lista permissiva di `deny.toml` (I) | no |

**Perché in casa** (V dal prototipo): il cuore (nodi DOM, fili SVG, trascinamento, zoom, pizzico) sono ~250 righe; il resto è nostro comunque (porte tipizzate, conversioni proposte, anteprime, stati, mzflow/2). Una libreria aggiungerebbe peso, un bundler e un modello di accessibilità da correggere. Le idee si prendono (§1), il codice no.

## 3. Come si parlano i moduli A VISTA
**3.1 La porta.** Pallino con colore Okabe-Ito (variabile CSS `--t-<tipo>`, chiaro e scuro), **forma** e nome della porta; `aria-label` completo («Input index, Scan index, one per file, of Detect features»). Proposta per i 15 tipi di `dataType`:

| Forma | Famiglia | Tipi (colore) |
|---|---|---|
| quadrato | file grezzi e convertiti | `raw` (arancio #E69F00), `mzml` (arancio chiaro) |
| cerchio | indici e tracce | `mzidx` (blu #0072B2), `xic` (azzurro #56B4E9) |
| rombo | picchi e spettri per file | `peaks` (verde #009E73), `spectra` (verde chiaro), `rtmap` (giallo #F0E442 con bordo) |
| triangolo | tabelle | `table` (vermiglio #D55E00), `annot`, `tp` (viola) |
| esagono | chimica | `structure` (rosa #CC79A7), `transform` (rosa con riga) |
| cerchio cavo | fine catena | `samples`, `qc`, `report` (grigio) |

Due tipi della stessa famiglia hanno la stessa forma: il colore e il nome li separano; chi non vede i colori ha comunque forma + nome + testo del suggerimento (V nel prototipo: `glyph()` in `flow-proto.js`).

**3.2 Mentre colleghi.** Al primo movimento da un'uscita (o al clic su di essa) ogni ingresso riceve uno di tre stati, calcolati da `check()` (nel prodotto: dalla risposta di `plan.check`):
- **ok**: pallino ingrandito, testo verde e in grassetto;
- **adapt**: testo arancio + «⇄», suggerimento «Needs: Build index»;
- **no**: porta al 40 %, suggerimento con il motivo dalla chiave (`err.flow.type`, `err.flow.cycle`, `err.flow.cardinality`).
I nodi senza nessun ingresso utile si attenuano al 35 %; la barra di stato annuncia «Pick an input: 2 compatible, 1 through a conversion» (regione `aria-live`). Rilasciare su un ingresso «adapt» apre una nuvoletta «Insert: Build index» con **Insert** e **Cancel** (V: prova «adapter proposed, not inserted»). Rilasciare nel vuoto apre la palette con i soli passi che accettano quel tipo (idea di Unreal).

**3.3 Il filo.** Colore del tipo che porta; raggiungibile con Tab (`role="button"`, `aria-label` «Link Align samples table to Export CSV table, Feature table»); al passaggio del mouse o con Invio mostra l'**anteprima**: tipo, descrittore (numero di file, MB, faccette `hr`/`lr`, polarità, righe), chiave della cache, e «Not computed yet» se il dato non esiste ancora. Canc lo toglie. Durante il calcolo del passo a valle il filo «scorre» (tratteggio animato, spento con `prefers-reduced-motion`).

**3.4 Errori.** Tre posti, sempre lo stesso testo dal catalogo delle chiavi `err.flow.*` (mai testo scritto nel codice della tela): il bordo e il quadratino rosso sul nodo, la porta che manca, la barra di stato che porta il fuoco sul nodo sbagliato (V: «Missing input “index” in Detect features»). Un passo fallito ferma solo i discendenti (COMUNICAZIONE.md §3): i fratelli restano verdi.

**3.5 Stati del nodo** (forma + colore + testo nascosto per i lettori di schermo): `idle` cerchio vuoto, `queued` tratteggiato, `running` che gira (fermo con movimento ridotto) e barra di avanzamento, `done` pieno verde, `cached` anello verde, `stale` («needs run») righe arancio, `error` quadrato rosso. La barra di avanzamento usa i messaggi `step.progress {done,total}` della barra unica di oggi (`progresso.js`, V nel codice).

## 4. Pochi clic, icone, movimento
- **Clic contati nel prototipo** (V): collegare = un trascinamento o due clic; conversione mancante = +1 clic; eseguire = 1 clic o tasto R; aggiungere un passo = A, due lettere, Invio.
- **Icone**: nel prodotto da `QICON` (`icons-modi.js`, `currentColor`, segue il tema; V), una per famiglia di passo (`passo/1.icon`). Il prototipo usa caratteri Unicode per non toccare il codice del sito.
- **Movimento**: solo `transform` e `opacity`, con i gettoni `--t-fast` 120 ms e `--t-med` 180 ms di `transizioni.css`, che `prefers-reduced-motion` e `data-a11y-reduce` portano a 0 (V nel codice). Comparsa di nodi e nuvolette con la stessa animazione dei pannelli; inserimento di un adattatore con FLIP come in `transizioni.js` (I).
- **Un solo modulo per i parametri**: doppio clic o Invio sul nodo apre il pannello generato dallo schema dei parametri (numero con unità e limiti, «Auto» per `x-estimate` con la prova accanto); cambiare un valore mette «needs run» sul nodo e sui discendenti (V nel prototipo).

## 5. Tocco, tastiera, prestazioni, salvataggio, esecuzione
**Tocco** (V nel prototipo): Pointer Events (mouse, dito, penna uguali), `touch-action:none` sulla tela, pizzico con due dita = zoom, bersagli 28 px col mouse e 40 px con `pointer:coarse`. Sul telefono Flow non si scarica (come Dati e Disegno, `telefono.js`); al massimo, più avanti, un elenco in sola lettura (I).

**Tastiera** (V nel prototipo, prova «keyboard only»): Tab tra nodi, porte e fili; frecce spostano il nodo di 16 px (Maiusc 64); Invio su un'uscita «arma» il collegamento, Invio su un ingresso lo chiude; Esc annulla; Canc toglie; A apre la palette; R esegue.
Manca nel prototipo e serve nel prodotto: **«Steps list»**, la tabella dei dati del grafo (un passo per riga: ingressi come menu a tendina con le sole uscite compatibili, stato, azioni). È l'alternativa completa alla tela per chi non la vede, come `TDATI` per i grafici (regola 6 di AGENTS.md).
WCAG 2.2 AA da rispettare: 2.5.7 trascinamento con alternativa, 2.5.8 bersagli ≥ 24 px, 2.4.11 fuoco non nascosto (la nuvoletta non deve coprire il nodo col fuoco), 1.4.11 contrasto 3:1 di porte e fili sullo sfondo (da misurare per ogni colore, I).

**Prestazioni** (regole usate nel prototipo, V): nodi come `div` spostati con `transform`; pan e zoom = un solo `transform` sul livello; posizioni delle porte misurate una volta per nodo e poi solo sommate; spostamenti raccolti in un `requestAnimationFrame` che ridisegna solo i fili del nodo mosso; prima tutte le letture del layout, poi tutte le scritture (questa sola regola ha portato il disegno di 506 nodi da 909 a 166-370 ms, V).
Oltre ~300 nodi serve nascondere i nodi fuori schermo e semplificarli quando lo zoom è basso (I); un workflow vero ha 10-30 passi (`PROVE_SU_CARTA.md`), i gruppi chiusi (sotto-flussi) tengono la tela piccola.

**Salvataggio** (V nel prototipo, prova «round trip»): il file è `mzflow/2` con `passi` (il piano) e un blocco **`ui`** a parte: `view {x,y,k}`, `nodes {id: {x,y}}`, più avanti gruppi chiusi e note. **Proposta allo studio WORKFLOW**: aggiungere `ui` come campo facoltativo di `mzflow/2`, escluso da `plan_sha256` (spostare un nodo non cambia il piano). Senza `ui` la tela dispone i nodi da sola per livelli dell'ordine di Kahn (I). La tela legge titoli, icone, porte e parametri dai descrittori `passo/1`: nessun passo scritto a mano nella grafica.

**Esecuzione e cache** (V nel prototipo, prova «second run from cache»): ordine di Kahn, chiave per nodo = passo + parametri canonici + chiavi degli ingressi; al secondo giro tutti i nodi sono «from cache». Nel prodotto: chiave SHA-256 del motore (COMUNICAZIONE.md §3), «Run» e «Run up to here» dal menu del nodo, annulla per nodo, parallelismo deciso dal pianificatore (la tela mostra più nodi «running» insieme).

## 6. Disegno e viste esistenti
- **Ketcher resta l'editor** (`draw.js`, iframe `sandbox`, V). Il nodo «Parent structure» ha un pulsante «Draw…» che apre Disegno con la struttura attuale; «Use in Flow» riporta un record `mol/1` (PR #1098) sull'uscita `structure`. L'anteprima del nodo è un disegno 2D di OpenChemLib (già nel sito, V), solo lettura. Nessun editor nostro.
- **Viste**: la tela non ridisegna grafici. Ogni uscita ha «Open in view»: `mzidx`/`xic` → pannelli di Dati (TIC/XIC/spettro), `table`/`tp` → tabella di Tabui (`PIPET`), `structure` → Disegno. Le miniature (XIC, spettro) riusano i disegnatori esistenti se possono scrivere in un canvas piccolo dato (I, da verificare per `cromato.js` e `spettro.js`), e ognuna ha la sua tabella dei dati.
- **Teoria**: «?» sul nodo apre il capitolo collegato (`#/teoria/N`), in italiano come sempre (Principio 5: nessun testo nuovo nella Teoria).

## 7. Licenze e regole
Idee sì, codice altrui no: nessuna riga copiata da xyflow, Rete, LiteGraph, Drawflow, Node-RED, Galaxy (letti solo documentazione e note). mzLab MIT; niente dipendenze nuove; niente binari in git (il prototipo è testo). Principio 1: nel mondo LR i passi `tp.*` e `annot.*` non compaiono nella palette (faccetta `lr` → la palette li toglie, come `in.requires` di COMUNICAZIONE.md §2).

## 8. Prove e criteri di accettazione
`python3 docs/studi/flow_grafica/misura.py [--cpu 4]` (Chromium 1194 di `/opt/pw-browsers`, finestra 1366×768, container cloud con 4 CPU). Esito di oggi: **7/7 prove funzionali verdi, axe WCAG 2.2 AA 0 violazioni in chiaro e in scuro** (V). Le misure sono rumorose (container condiviso): ogni riga riporta l'intervallo di 2-3 giri.

| Prova funzionale (V) | Esito |
|---|---|
| trascinando `raw` sull'ingresso `index` la porta diventa «adapt» | ok |
| al rilascio compare la proposta e il grafo non cambia | ok |
| «Insert» aggiunge 1 nodo e 2 fili | ok |
| il secondo «Run» prende tutto dalla cache | ok («0 step(s) run, 7 from cache») |
| collegamento fatto solo con Tab e Invio | ok |
| freccia → sposta un solo nodo | ok |
| salva → apri → salva dà lo stesso `passi` | ok |

Fluidità (V; intervallo tra fotogrammi durante 90 movimenti, mediana / 95° percentile / fotogrammi oltre 50 ms):

| Nodi | CPU | disegno iniziale | trascina nodo | sposta la tela |
|---|---|---|---|---|
| 6 | ×1 | 3 ms | 16,7 / 16,7 / 0 | 16,7 / 16,7 / 0 |
| 206 | ×1 | 65-95 ms | 16,7 / 16,7-16,8 / 0 | 16,7 / 33,2-33,3 / 0 |
| 506 | ×1 | 166-370 ms | 16,7 / 16,7-16,8 / 0 | 16,7 / 33,4 / 0-1 |
| 206 | ×4 | 344-382 ms | 16,7 / 33,4 / 0-1 | 16,7 / 33,4 / 1-2 |
| 506 | ×4 | 1,3-1,8 s | 16,7 / 16,8 / 1 | **33,3 / 83,4 / 34-42** |

Lettura: trascinare un nodo resta a 60 fotogrammi al secondo anche a 500 nodi; spostare tutta la tela a 500 nodi con CPU lenta scende a ~30 (V). Ridurre l'SVG dei fili da 20 000 px a 1 px con `overflow:visible` **peggiora** (fino a 89 fotogrammi oltre 50 ms a 206 nodi, V): la cura vera è nascondere i nodi fuori schermo (I).

**Criteri per il prodotto** (ogni fetta li eredita in un e2e):
1. le 7 prove funzionali sopra, sul codice vero, in `tests_e2e/`;
2. axe WCAG 2.2 AA a 0 violazioni in chiaro, scuro e contrasto alto; `e2e_axe_grafici` verde;
3. con 200 nodi e CPU ×4: mediana ≤ 20 ms e ≤ 2 fotogrammi oltre 50 ms per trascinamento e spostamento;
4. con `prefers-reduced-motion` nessuna animazione CSS attiva (`document.getAnimations().length == 0` dopo un «Run»);
5. «Steps list» permette di fare ogni cosa che fa la tela (stesso `mzflow/2` in uscita);
6. nessun testo della tela fuori dal catalogo dei messaggi; nessuna parola «formula» da sola (regola Tabui);
7. tour: un passo «Flow» trova il suo bersaglio visibile (`e2e_tour`).

## 9. Piano a fette (PR piccole, file ≤ 500 righe, funzioni ≤ 80, `// @ts-check`)
Prima servono W-1 e W-2 dello studio WORKFLOW (descrittori `passo/1`, `plan.check` in Rust): la tela non scrive regole (COMUNICAZIONE.md §5, «Cosa NON fare» 10).

| Fetta | Perimetro | Peso stimato (I) | Prova |
|---|---|---|---|
| F-1 sola lettura | apre un `mzflow/2`, disegna nodi e fili dai descrittori, zoom/pan, «Steps list» | ~350 righe in 2 file | e2e apertura + axe |
| F-2 modifica | sposta (mouse, tocco, frecce), collega (trascina, clic-clic, tastiera), toglie; stati da `plan.check` | ~350 righe | prove 5-6 di §8 |
| F-3 conversioni e palette | proposta «Insert», palette filtrata per tipo e per faccetta `lr` | ~200 righe | prove 1-3 |
| F-4 esecuzione | stati, avanzamento, cache, «needs run», annulla, «Run up to here» | ~250 righe | prova 4 + criterio 4 |
| F-5 anteprime | nuvoletta del filo, «Open in view», miniature con tabella dei dati | ~250 righe | e2e anteprima + `TDATI` |
| F-6 struttura | nodo «Parent structure» ↔ Disegno con `mol/1` | ~150 righe | e2e andata e ritorno |
| F-7 scala e tocco | nodi fuori schermo nascosti, gruppi chiusi, pizzico | ~200 righe | criterio 3 |
| F-8 app desktop | stessa pagina in Tauri, fili nativi | verifica | e2e desktop |

Totale ~1 750 righe JS in ~6 file, ~40-45 KB compressi in tutto (I, il prototipo pesa 13,6 KB compressi). Il JS di Flow si scarica solo quando si apre Flow, come i `.wasm` della Pipeline (V per la Pipeline). Nessun `.wasm` cresce: la grafica non ha calcoli.

## 10. Cosa NON fare
1. Non mettere regole di compatibilità nella tela: le decide il motore (§0.2).
2. Non usare `<canvas>` per i nodi: si perdono tastiera e lettori di schermo.
3. Non affidare un'informazione al solo colore; non usare colori fuori dalle variabili CSS.
4. Non inserire conversioni senza chiedere.
5. Non salvare le posizioni nel piano (`passi`): stanno in `ui`, fuori dall'impronta.
6. Non costruire un editor di strutture: Ketcher resta.
7. Non copiare codice di librerie o programmi a nodi, anche se MIT: il costo dell'adattamento supera quello della scrittura (V: il nucleo sono ~250 righe).

## 11. Decisioni per Federico
1. **Dove vive Flow**: dentro Tabui, solo inglese (raccomandato: è uno strumento di ricerca, come la Pipeline) oppure nell'app principale con `lang/` it/en (serve se lo usano gli studenti in bassa risoluzione).
2. **Proposta allo studio WORKFLOW**: campo `ui` facoltativo in `mzflow/2`, fuori da `plan_sha256`.
3. **Colori e forme del §3.1**: confermarli prima di F-1. Cambiarli dopo è facile: nei file salvati ci sono i tipi, non i colori.

## Fonti
- KNIME Analytics Platform User Guide: https://docs.knime.com/latest/analytics_platform_user_guide/ ; stati dei nodi (secondaria): https://abierta.ugr.es/pluginfile.php/83073/mod_resource/content/9/8.1_en.pdf
- Unreal Engine, nodi e porte: https://dev.epicgames.com/documentation/en-us/unreal-engine/nodes-in-unreal-engine ; `AutowireConversionNode`: https://docs.unrealengine.com/4.26/en-US/API/Editor/BlueprintGraph/UEdGraphSchema_K2/AutowireConversionNode/index.html
- Galaxy, conversioni implicite: https://lists.galaxyproject.org/archives/list/galaxy-dev@lists.galaxyproject.org/thread/FQUD3SZFEXP4ZRDR2MOCX6SZMXSFZ3C6/ ; editor: https://galaxyproject.org/learn/advanced-workflow/basic-editing/
- Orange, segnali dei widget: https://orange-widget-base.readthedocs.io/en/stable/widget.html
- React Flow, accessibilità: https://reactflow.dev/learn/advanced-use/accessibility ; https://reactflow.dev/api-reference/types/aria-label-config
- LiteGraph (fork ComfyUI): https://cdn.jsdelivr.net/npm/@comfyorg/litegraph@0.17.2/README.md ; licenza dell'originale: https://www.jsdelivr.com/package/npm/litegraph.js
- Drawflow: https://github.com/jerosoler/Drawflow (MIT, JS senza dipendenze; ultimo push da https://awesome.ecosyste.ms/projects/github.com%2Fjerosoler%2Fdrawflow)
- Rete.js plugin: https://classic.yarnpkg.com/en/package/rete-area-plugin ; https://classic.yarnpkg.com/en/package/rete-connection-plugin
- WCAG 2.2: https://www.w3.org/TR/WCAG22/ (2.5.7 Dragging Movements, 2.5.8 Target Size Minimum, 2.4.11 Focus Not Obscured, 1.4.11 Non-text Contrast)
- Codice di mzLab letto: `mzlab/web/transizioni.css`, `transizioni.js`, `app.css` (variabili), `icons-modi.js` (`QICON`), `tdati.js`, `draw.js`, `touch.js`, `explore.js` (Okabe-Ito), `TP_Mine/js/32_pipeline.js`, `tools/controlla.py`, `tools/indice.py`.
