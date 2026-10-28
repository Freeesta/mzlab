# Grafo MSn v2: prompt per le chat DEV (G1-G5, G4b)
Passo 2 dell'ARCH MSn-grafo v2. Criteri, soglie e decisioni: `docs/archivio/studi/msn-grafo-v2/CRITERI.md` (§3 D1-D8, tabella §4 punti 0-12, prove §5 a-f). Decisioni confermate dal COO: `vista` in coda a `msngrafo/1`, `best-path/1` lessicografico, SVG statico Rust + modulo JS condiviso, disegni da molfile, `resvg` SOLO come feature cargo `png` facoltativa della CLI (non nel default, mai nei `.wasm`, `cargo-deny` verde).

## Ordine e dipendenze
| Prompt | Dipende da | Parallelo con | Note |
|---|---|---|---|
| G1 · dati, best path, livelli, %, perdite, energie (+ G0: 55 nodi senza formula vs 26 con formula, eV vs NCE) | CRITERI in main | G4b | parte SUBITO; G0 è il suo primo commit |
| G4b · .raw dell'infusione nel browser | nessuna | G1 | parte SUBITO; tocca lettura/indice, non `msngrafo/` |
| G2 · scheda: modulo `msngrafo-vista.js`, albero/radiale, pedici, filtri, disegni | G1 unita | G4a | |
| G3 · Flow/CLI: SVG Rust, HTML autonomo, PNG | G1 unita, G2 unita (usa `msngrafo-vista.js`) | G5 (parte cloud) | |
| G4a · apri un g.json trascinato | G1 unita (`completa`) | G2 (file diversi; se tocca `infusione-grafo.js` va dopo G2) | |
| G5 · prove esterne a-f | G1-G3 unite | | cloud + loc COO + Federico |

Regole comuni a ogni prompt (già scritte dentro ciascuno): leggi AGENTS.md e il file d'area; ramo `claude/<argomento>` da `origin/main`; NON unire: PR + SHA a CI verde al COO (session_01SqVGjScVGPMmoMETWKcX2F, con send_message); conferma subito la ricezione al COO; tetto `.wasm` `msngrafo` 512 000 B MAI alzato; non toccare workflow, CSP, altri tetti; testi del Flow in inglese senza chiavi `lang/`, testi della scheda in `lang/it.js` e `lang/en.js`; nessun dato privato (nomi, m/z o spettri dei file veri) nel repository pubblico.

---

## G1 · Dati del grafo v2 (`vista` in `msngrafo/1`)
```text
Sei DEV G1 «grafo MSn v2: dati» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO (≤ 5 righe).
Leggi: AGENTS.md (già in contesto), docs/archivio/studi/msn-grafo-v2/CRITERI.md (tutto, è il contratto), docs/agenti/msn-grafo.md §2-§4, docs/agenti/rust.md (parti su msngrafo), docs/agenti/wasm-tetti.md.
G0 (primo commit, prima del codice nuovo): spiega con numeri due discrepanze. (1) Sul timololo 55 nodi/101 archi sono il grafo senza formula (solo masse), 26 nodi/36 archi quello con la formula dello ione (pagina e CLI uguali, misura del Mac): riproduci sul sintetico (crates/mzlab-core/src/msngrafo/tests.rs, tests_e2e/fixtures/msngrafo_esempio.json, TP_Mine/tests/msn_synth.py) le opzioni che la pagina manda (infusione-grafo-stato.js `params`) e quelle della CLI (crates/mzlab-cli/src/msn_graph.rs): formula sì/no, interpreta, attivazione, k/tol/ce dell'albero; conferma sul sintetico che solo la formula cambia il conteggio.
Se serve il file vero scrivi nel CODA di mzlab-dati la richiesta al loc COO (comandi esatti, solo aggregati). (2) Energie Thermo: il lettore marca cid25.00 come eV (energy_is_nce=false): verifica nel lettore Thermo e nei filtri, e o correggi l'unità (NCE %) o lasciala com'è con un perché; se il fix sta nel lettore Thermo e cambia hash di altri test, fermati e chiedi al COO. Scrivi l'esito in CRITERI.md §2 (riscrivi la riga, niente cronaca).
Poi realizza D1-D3 di CRITERI.md:
- crates/mzlab-core/src/msngrafo/vista.rs (nuovo): `vista` calcolata SOLO dai campi già nel grafo: per nodo id, livelli (livelli MS degli spettri in cui lo ione compare, da archi[].misure), rel (% nello spettro del genitore sul best path, misura più intensa; radice null se non c'è MS1), rel_max, best {da, passi, costo}, entranti, comune, albero {col, riga}, radiale {anello, angolo}; per arco {da, a, best}; algoritmo "best-path/1" con la regola in chiaro; ordine.
- best-path/1: DP in ordine topologico sul DAG, costo lessicografico (passi min; poi minima rel lungo il cammino max; poi più archi con isolamento; poi id minore). Calcola anche, solo nei test, quanti nodi cambierebbero genitore con il «miglior arco entrante» (Chu-Liu/Edmonds degenerato sul DAG) e scrivilo nel test come numero atteso del sintetico.
- posa.rs: alla disposizione a baricentro aggiungi una passata di scambi adiacenti che riduce gli incroci; posa_radiale.rs: anello = passi, angolo per sottoalbero ∝ foglie del best path, in gradi a 2 decimali, senza trigonometria (il disegnatore fa seno e coseno).
- json.rs: `vista` in coda (dopo `strutture` se c'è), ordine fisso, numeri come il resto; sempre emessa. `pub fn completa(json: &str) -> Result<String>`: aggiunge `vista` a un msngrafo/1 che non l'ha (stessi byte di un grafo costruito con vista); errore con chiave se non è msngrafo/1.
- Perdite: `perdita.formula` per ogni arco fra nodi con una scelta (già c'è): verifica che sia sempre il nome solito (H2O, NH3, C4H8) e aggiungi i casi mancanti; nessuna logica di presentazione (pedici) qui.
- Opzione `molfile` (come `smiles`: solo ripetuta nel JSON, per i disegni di G2/G3; `--molfile FILE` in CLI, parametro del passo con --set) e `strutture=tutti` (i tagli di tutti i nodi di livello ≥ 2) se il tetto lo consente, altrimenti scrivi che resta per nodo.
- Op `completa` nel crate mzlab-msngrafo-wasm e in rust-msngrafo.js (per G4a); `mzlab-cli msn-graph complete IN.json [--out]`.
- Schema docs/schemi/msngrafo-1.json (chiave facoltativa `vista`), docs/agenti/msn-grafo.md (riscrivi §2 e §4 per `vista`, `best-path/1`, ordine; una riga per funzione nuova), golden e hash aggiornati UNA volta con il perché nel commit.
Criteri (CRITERI.md §4): punti 0, 1, 2, 4, 8 (parte dati), 12; stessi ingressi = stessi byte (ingressi permutati, due esecuzioni); completa(grafo senza vista) == grafo con vista byte per byte; test di parità CLI = .wasm (tests/test_msngrafo_wasm.py, e2e_msngrafo).
Tetto: `.wasm` msngrafo misurato oggi 505 993 B su 512 000: misura dopo ogni passo con tools/build_feat_wasm.sh; il tetto NON si alza. Se sfora: togli codice morto del modulo o chiedi al COO, mai alzare il tetto.
Non toccare: interfaccia (G2), Flow/CLI figure (G3), workflow, CSP, Cargo.toml (nessuna dipendenza nuova). Prima di spingere: tools/fondi_main.py, tools/pre_pr.py, cargo test -p mzlab-core msngrafo, python3 tools/verifica.py --cambiati.
```

## G4b · Infusione .raw nel browser (bloccante oggi)
```text
Sei DEV G4b «infusione .raw nel browser» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO.
Problema (dal Mac di Federico, sito 1.0.0): un'infusione Thermo .raw di 9000 scansioni (MS2-MS8, CID) caricata DIRETTAMENTE nella pagina si pianta oltre 8 min senza errore e la scheda «Fragmentation graph» non arriva; lo stesso file convertito in mzML (mzlab-cli convert, 1,3 s) funziona; la CLI dal .raw costruisce il grafo in 3,5 s. Nel Flow del browser il passo spectra.msn_graph sul .raw diretto fallisce (prima si vedeva err.scan.outside su api/msn_grafo).
Leggi: AGENTS.md, docs/agenti/lettura-nativa.md, docs/agenti/progresso.md, docs/agenti/msn-grafo.md §7 (modulo msngrafo), mzlab/web/rust-msngrafo.js (il modulo legge il .mzidx di OPFS e «l'mzML del file» tramite readerFor: verifica cosa riceve per un .raw), docs/agenti/flow-web.md.
Passi: (1) diagnosi riproducibile PRIMA del fix: in quale fase si ferma (indice, mzML su richiesta, lettura degli spettri nel modulo msngrafo, worker) con un registro di avanzamento per fase; senza il file vero usa un .raw pubblico di test se ce n'è (tests/golden, grep thermo) o chiedi al loc COO via CODA.md di mzlab-dati (comandi esatti, tempi per fase, solo aggregati). Ipotesi da verificare, non fatti: il modulo msngrafo riceve il .raw al posto di un mzML; la conversione in mzML su richiesta di 9000 scansioni è lenta o in un ciclo; un errore nel worker viene inghiottito.
(2) Fix minimo nella fase colpevole; (3) mai silenzio: se una fase non avanza per 30 s o fallisce, errore con chiave (lang/it.js e lang/en.js) e «Copia diagnosi» (diagnosi.md); (4) stesso JSON msngrafo/1 dal .raw e dal mzML dello stesso file (SHA-256).
Criteri (CRITERI.md §4 punto 11): grafo dal .raw nel browser ≤ 60 s sul Mac (misura del loc COO), barra che sale, errore entro 30 s dall'ultimo progresso, JSON identico al mzML. e2e su un sintetico in tests_e2e (stesso file via mzML e via formato nativo se ne esiste uno sintetico, altrimenti test di unità sulla fase colpevole) e MZLAB_LR_MISS vuoto.
Non toccare: crates/mzlab-core/src/msngrafo/ (lo fa G1 in parallelo; se il fix sta lì, coordina col COO), tetti .wasm, workflow, CSP. Prima di spingere: tools/fondi_main.py, tools/pre_pr.py, python3 tools/verifica.py --cambiati. Scrivi la riga del comportamento in lettura-nativa.md o progresso.md.
```

## G2 · Scheda «Fragmentation graph»: albero e radiale
```text
Sei DEV G2 «grafo MSn v2: scheda» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO. Parti solo con G1 in main (chiave `vista` in msngrafo/1).
Leggi: AGENTS.md, docs/archivio/studi/msn-grafo-v2/CRITERI.md (§3 D4, D5, D7, D8 e §4), docs/agenti/infusione-grafo.md, docs/agenti/msn-grafo-strutture.md, docs/agenti/accessibilita.md (sezione grafici), docs/agenti/lingue.md; codice: mzlab/web/infusione-grafo-svg.js (drawGraph), infusione-grafo-nodo.js, infusione-grafo-css.js, molui.js (MOLUI.svg, OpenChemLib), flow/lazy/flow-msn-graph.js (riusa drawGraph).
Fai:
- mzlab/web/msngrafo-vista.js (nuovo modulo ES, // @ts-check, progetto in tools/tipi/): disegnatore PURO senza globali (niente G, I18N: testi e scelte passati in `opts`), due viste da `vista`: albero (vista.nodi[].albero) e radiale (anello/angolo, seno e coseno qui), archi best pieni e altri sottili tratteggiati, nodi `comune` con doppio bordo e numero dei precursori. infusione-grafo-svg.js diventa un adattatore sottile su di esso; flow-msn-graph.js lo usa con testi inglesi. Il modulo sarà incorporato anche nell'HTML autonomo di G3: niente import da altri file della pagina.
- Etichette (regola unica, scrivila in testa al modulo; G3 la ripete in Rust): m/z 4 decimali; formula scelta (o i candidati come oggi) con pedici (tspan baseline-shift) e carica in apice, ppm; «MS2» o «MS2, MS3» da vista.livelli; rel % a 1 decimale (la radice senza); sull'arco la perdita come formula con pedici, altrimenti «Δ 18.011 Da». aria-label e testi per lettori di schermo in forma piana (C13H25N4O3S+). CSV, MGF, MSP, JSON e «Copy data» restano ASCII.
- Commutatore «Tree / Radial» (bottone, tastiera, ≥ 24×24 px, aria-pressed), nodo scelto mantenuto; filtri: min %, livelli, «Best path only», chiudi sottoalbero; zoom e spostamento con pulsanti e tastiera (niente trascinamento obbligatorio); nome del best path nella pagina: «Shortest observed path» con la regola in una riga sotto.
- Tabella dei dati (TDATI) con le nuove colonne (livelli, %, perdita, best, comune).
- Disegni (D7): quando si sceglie una molecola, la pagina ricava con OpenChemLib il molfile 2D e lo passa come opzione `molfile` (opzione `molfile` di G1, solo ripetuta); radice disegnata; per i nodi con tagli (`strutture`; chiedi `strutture=tutti` se G1 l'ha fatto, altrimenti per nodo come oggi) il disegno del genitore con gli atomi del `pezzo` evidenziati (colore Okabe-Ito + tratteggio) e l'etichetta «hypothesis».
- Testi in lang/it.js e lang/en.js (chiavi msngrafo.*), tour.js valido, docs/agenti/infusione-grafo.md riscritto per le parti cambiate.
Criteri (CRITERI.md §4): punti 1, 3, 4, 5, 6, 7 (0 sovrapposizioni; incroci ≤ 50 % del baricentro di oggi sul sintetico di 80 nodi da costruire in tests_e2e/fixtures), 8 (nessuna richiesta api/ dopo il JSON salvo spettri e strutture). Prove: e2e_infusione_grafo.py esteso (viste, pedici: nessun testo visibile che corrisponde a [A-Z][a-z]?[0-9] fuori da numeri, tastiera, axe 0 violazioni, it/en), tests/flow_msn_view.js, e2e_axe_grafici, e2e_tour.
Non toccare: crates/ (G1/G3), tetti, workflow, CSP (se il disegno ne avesse bisogno, fermati e chiedi). Prima di spingere: tools/fondi_main.py, tools/pre_pr.py, python3 tools/verifica.py --cambiati.
```

## G3 · Figura e HTML dal Flow e dalla CLI
```text
Sei DEV G3 «grafo MSn v2: figura e HTML» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO. Parti solo con G1 e G2 in main.
Richiesta di Federico: il grafo deve uscire da `flow run` e da mzlab-cli come figura (SVG, PNG) e come HTML autonomo interattivo, senza aprire l'app.
Leggi: AGENTS.md, docs/archivio/studi/msn-grafo-v2/CRITERI.md (§3 D5, D6, D8; §4 punto 9), docs/agenti/cli.md (regola CLI, registro steps, FileRunner), docs/agenti/rust-step.md, docs/agenti/flow-grafica.md (vista msngrafo, flow-msn-graph.js), crates/mzlab-cli/src/msn_graph.rs e flow_msngraph.rs, mzlab/web/msngrafo-vista.js (G2: la regola delle etichette in testa).
Fai:
- crates/mzlab-cli/src/msn_figura.rs (nuovo, solo nativo, MAI nei .wasm): SVG statico da msngrafo/1 + `vista` (vista albero o radiale), stesse etichette di msngrafo-vista.js (pedici con tspan, carica in apice, livelli, %, perdite), colori Okabe-Ito scritti nello SVG (il file vive da solo), <title> e <desc>, testi in inglese. Molecola: se le opzioni hanno un molfile, disegno da coordinate con mzlab-mol (linee e simboli, niente generatore di coordinate); senza: riga «no 2D coordinates».
- HTML autonomo (`msn_html.rs`): un file unico senza risorse esterne: JSON incorporato, msngrafo-vista.js incorporato con include_str!, tabella dei dati con «Copy data», due viste, tastiera; WCAG 2.2 AA (axe 0 violazioni); ≤ 2 MB a 55 nodi; apribile offline.
- PNG: feature cargo `png` FACOLTATIVA di mzlab-cli con resvg (autorizzata dal COO; non nel default, mai nei .wasm, cargo-deny verde, licenze ok); senza la feature `--figura png` dà un errore con chiave che dice come averla. Nel Flow del browser e nella scheda: PNG dallo SVG con canvas.
- CLI: `mzlab-cli msn-graph … --figura svg,html[,png] [--vista albero|radiale]` e `mzlab-cli msn-graph figure IN.json` (da un g.json già fatto, dopo `completa` se manca `vista`). Flow: uscite nuove `figure_svg`, `figure_html` (e `figure_png` con la feature) del passo spectra.msn_graph@1, oppure un passo `msn.figure@1` se tiene più pulito il registro: scegli e scrivi il perché; ogni parametro ha il suo --set (regola CLI, flow_rule_tests.rs verde, docs/schemi/steps-cli-1.json, BUCHI se resta un buco); testi del Flow in inglese, niente chiavi lang/.
Nel Flow grafico, scarica SVG/PNG/HTML dalla vista Graph.
- Parità: test che confronta i testi dei nodi e degli archi dello SVG Rust con quelli di msngrafo-vista.js sullo stesso JSON (node tests/ o e2e); stessi byte di SVG e HTML da CLI e Flow nativo.
Criteri (CRITERI.md §4 punto 9 e parte figura dei punti 1-5): 0 violazioni axe sull'HTML generato (e2e che lo apre da file), HTML ≤ 2 MB, nessuna richiesta di rete dall'HTML, SVG valido (xmllint o parser), parità dei testi 100 %.
Non toccare: crates/mzlab-core/src/msngrafo/ salvo una funzione di sola lettura se serve (chiedi), tetti .wasm, workflow, CSP. Prima di spingere: tools/fondi_main.py, tools/pre_pr.py, cargo test -p mzlab-cli, cargo test -p mzlab-cli --features png, cargo deny check, python3 tools/verifica.py --cambiati. Righe nuove in cli.md, msn-grafo.md §6, flow-grafica.md.
```

## G4a · Aprire un g.json trascinato
```text
Sei DEV G4a «grafo MSn: apri g.json» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO. Parti con G1 in main (op `completa` nel .wasm msngrafo); se G2 è aperta e tocca gli stessi file (infusione-grafo.js, banco.js), aspetta che sia unita.
Problema: trascinare il g.json (msngrafo/1) della CLI o del Flow nell'area «Data files» dà «format not recognised»; Federico vuole aprire il grafo senza il grezzo.
Leggi: AGENTS.md, docs/archivio/studi/msn-grafo-v2/CRITERI.md (§3 D1, §4 punti 8 e 10), docs/agenti/dati-carico.md, docs/agenti/infusione-grafo.md, docs/agenti/formato-nuovo.md, docs/agenti/diagnosi.md.
Fai: riconoscimento di un .json con `formato: "msngrafo/1"` al caricamento (area dati e scheda), senza aprirlo come file di dati; se manca `vista` la pagina chiama l'op `completa` del .wasm msngrafo (nessun calcolo in JS); apre la scheda «Fragmentation graph» in sola lettura: spettro del nodo, «Use», strutture e regole che chiedono il grezzo spenti con un perché (chiave lang/); JSON rotto → errore con chiave e «Copia diagnosi»; formato più nuovo → error.newerFormat. Nel Flow grafico il report msngrafo/1 si apre già (flow-report.js): verifica che usi lo stesso percorso.
Criteri (punto 10): tre casi (vecchio senza vista, nuovo, rotto) in e2e con fixture sintetiche; nessuna richiesta api/ oltre a `completa`; MZLAB_LR_MISS vuoto; it/en; axe; tour valido.
Non toccare: crates/ salvo l'op `completa` già fatta da G1, tetti, workflow, CSP. Prima di spingere: tools/fondi_main.py, tools/pre_pr.py, python3 tools/verifica.py --cambiati. Riga in infusione-grafo.md e dati-carico.md.
```

## G5 · Prove esterne
```text
Sei DEV G5 «grafo MSn v2: prove esterne» di mzLab (repo Freeesta/mzlab, rispondi in italiano breve). Coordinatore: COO 11, session_01SqVGjScVGPMmoMETWKcX2F: conferma SUBITO la ricezione con send_message. NON unire PR: a CI verde manda PR + SHA al COO. Parti con G1-G3 in main.
Leggi: AGENTS.md, docs/archivio/studi/msn-grafo-v2/CRITERI.md (§5 prove a-f, §6 chi fa cosa), docs/agenti/msn-grafo.md §6 (--verita, --riassunto, compare), docs/agenti/test.md (NEEDS, dati privati).
Regole: riporta i numeri anche se la soglia fallisce; ogni fonte con ID e data di accesso, verificata NELLA fonte (mai a memoria); nel repository pubblico solo metodo, strumenti e aggregati; dati, spettri e tabelle per frammento in mzlab-dati (esperimenti/<data>_msngrafo_v2_prove/). Strumenti di prova solo in tools/ (Python ok, RDKit da PyPI ok), mai nel sito.
Fai, nel cloud: (a) scarica da MassBank gli spettri MS2 del timololo [M+H]+ (C13H25N4O3S+) con formule annotate, tabella fonte/ID/CE/attivazione/frammenti rel ≥ 5 %; script tools/msngrafo_prove/ che, dato un g.json, calcola il richiamo per sottoformula (± 10 ppm) per fonte; (c) script target-decoy: picchi esca spostati di +0,02…+0,05 Da e formula esca della radice (compose entro 5 ppm, composizione diversa), FDR per fascia m/z < 150 e ≥ 150; (d) SHA-256 del JSON da .wasm (node tests/msngrafo_wasm_node.js), CLI, Flow nativo e Flow del browser sul sintetico mzML.
Prepara per il loc COO (richiesta nel CODA di mzlab-dati, comandi esatti, solo aggregati di ritorno): (a)-(c)-(d) su INF-A (.raw e mzML); (b) SIRIUS e MS-FINDER sul MGF esportato del nodo MS2 e dei nodi MSn (stesse sottoformule per i picchi spiegati da entrambi, soglia 95 %); (f) coerenza col grezzo con scandump/RFR (catena di filtro, attivazione, NCE, scansioni per nodo isolato = JSON, 100 %); misura (punto 11) del .raw nel browser.
Prepara per Federico: (a) mzCloud e NIST se ha accesso; (e) protocollo del test utenti (5 persone, 3 compiti per vista, ordine alternato, foglio dei tempi e degli errori) in docs/archivio/studi/msn-grafo-v2/TEST_UTENTI.md.
Uscita: docs/archivio/studi/msn-grafo-v2/RISULTATI.md (tabella prova/soglia/valore/esito/chi/data, solo aggregati) e un messaggio al COO con gli esiti; per ogni soglia mancata, la causa e la proposta (codice o soglia, con il perché).
Non toccare: crates/, mzlab/web/, tetti, workflow, CSP. Prima di spingere: tools/fondi_main.py, tools/pre_pr.py.
```
