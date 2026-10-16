# Tabui: molte condizioni della stessa degradazione in una sessione

Studio dell'Architetto sulla richiesta di Federico: Tabui (mzFinder) deve aprire in una sola sessione molti file che sono la stessa degradazione della stessa molecola in condizioni diverse (stesso fotocatalizzatore con gas diversi, o fotocatalizzatori diversi; stesso metodo LC, stesso strumento, stessa concentrazione iniziale), raccogliere i TP di tutte le condizioni in una lista unica e confrontarli. Vale per la bassa risoluzione di oggi (full scan Q1 + MS2 a massa nominale, ESI + e −) e per l'alta risoluzione (HRMS) senza riscrivere nulla. Prompt delle chat DEV: `docs/studi/PROMPT_GRUPPI/`. Nessun dato reale in questo documento: il caso d'uso è descritto in modo generico (un pesticida, 6 gas, bianco, T0, standard del parent, solo endpoint).

## Sintesi per Federico
1. **Si fa con un solo allineamento di tutti i file e un'etichetta «condizione» per file.** Il programma cerca i picchi in tutti i file insieme (così lo stesso TP ha lo stesso nome, «TP 362», in ogni condizione), poi li conta e li confronta condizione per condizione.
2. L'etichetta esiste già nel modello dei campioni (`campioni/2`, campo `serie`): oggi la usa solo la Pipeline HR; Tabui in bassa risoluzione ha una sua tabella dei file più semplice e va portato sullo stesso modello.
3. **Errore da correggere prima di tutto**: oggi Tabui LR mette tutti i campioni con un tempo in UNA sola cinetica; con sei gas a 60 min avrebbe sei punti allo stesso tempo e giudizi sbagliati («grows over time»). Le cinetiche e i criteri vanno calcolati per condizione.
4. Cosa vedrai: una colonna «Condition» nella tabella dei file (proposta dai nomi, da confermare), la scelta della condizione di riferimento, poi una vista «Compare conditions» con la matrice TP × condizione (presente/assente, area in notazione scientifica, rapporto col riferimento), i gruppi «only in…» e «in all» (grafico UpSet), le barre raggruppate per TP (solo endpoint) o le curve sovrapposte (cinetiche), e le esportazioni in inglese.
5. **Il programma propone, il chimico decide**: «absent in T0», «absent in blank», «also in the parent standard», «above threshold», «MS2 linked» sono suggerimenti con la loro prova; nessun TP è dichiarato confermato.
6. Calcoli tutti in Rust (modulo nuovo `crates/mzlab-feat/src/gruppi/`), uguale per LR e HR: in HR si aggiungono solo colonne (formula neutra, errore ppm).
7. Lavoro stimato: 6 pacchetti, circa 9-11 giorni-chat; GR-1 (Rust) e GR-3/GR-4 (pagina, su dati di prova) partono subito e in parallelo; l'innesto nella tabella di Tabui (GR-5) aspetta le PR aperte #863, #871, #873.
8. Le tue risposte (§10) sono nel disegno: area grezza, un'etichetta libera per condizione con un secondo campo facoltativo senza vista, standard del parent come suggerimento, T0 e bianco condivisi come default (gestito anche uno per condizione).

## 1. Cosa c'è già (da non rifare)
- **Modello dei campioni** `campioni/2` (`crates/mzlab-core/src/campioni/`, `docs/agenti/campioni.md`): per file `tipo` (`sample t0 blank dark infusion std calibration qc`), `tempo` e `tempo_min`, `serie`, `replica`, `istd`; `proponi` ricava ruolo, tempo, serie (prefisso prima del tempo) e repliche dai nomi; `valida` dà avvisi per serie (`noControls`, `noT0`, `noBlank`, `onePoint`, `replicates`); `serie == ""` = condiviso da tutte le serie. Binding in `mzlab-feat-table-wasm` e `mzlab-wasm`. Lo usa la Pipeline HR (`TP_Mine/files/finder-ruoli.js`, `NB.campioni`).
- **Tabui LR** (`TP_Mine/js/20_tpmine.js`): tabella dei file propria (`20_tpmine.js:241`, tipi `sample/blank/standard/control` e tempo), file passati al Dig come `{name, type, time, kind}` (`:480`). Nessuna serie, nessuna replica.
- **Dig LR in Rust** (`crates/mzlab-feat/src/legacy/lr/`, `.wasm` `finder-lr`, `docs/agenti/mzfinder-lr.md`): un solo allineamento (`Aligned`) su tutte le colonne; `rows::cells(t, r)` dà una cella per file (rilevata, riempita o assente); `kin::kinetics` (`kin.rs:16`) prende **tutti** i `sample` con tempo, ordinati per tempo, senza distinguere serie; `labels::score` (`labels.rs:187`) giudica «grows over time», «persistent», controlli (`RATIO_MAX` 0,20) sull'intera lista dei file.
- **Nomi dei TP** (PR #871, `lr/nomi.rs::tp_names`): «TP 362» dall'm/z intero, `_31` per formule diverse con lo stesso intero, « A/B» per isomeri in ordine di RT, «Parent», «Candidate». Si calcolano una volta sulle righe del Dig: con un solo allineamento il nome è lo stesso in tutte le condizioni.
- **MS2** (PR #873, `lr/ms2assoc.rs`, `ms2link/`): esperimenti MS2 abbinati alle righe con m/z letto e picco della traccia; vale per file MS2 separati e per scansioni di livello 2 nei file full.
- **HR** (`crates/mzlab-feat/src/tp/`, `tptab/1`): `tp/tabella.rs` dà per riga `series` (aree dei trattati nel tempo), `in_t0`, `in_blank`, `t0_ratio`, `blank_ratio`, sempre su un'unica serie; `tp/rete.rs` correla le aree «lungo la serie». Ruoli da `campioni/2` (`role_of`).
- **Cinetica** (`crates/mzlab-feat/src/kinetic/`): forme, generazione, saturazione; `lr/kin.rs::decay` (k, t½, R² del parent).
- **Prove e dati sintetici**: `tools/dati_sintetici.py` (serie «B»: parent 364,4, TP 305,3, 229,1, 224,2, contaminante 391,3), `TP_Mine/tests/lr_synth.py`, e2e `e2e_tpmine_lr_tabella.py`, `e2e_tpmine_rtalign.py`, `e2e_tpmine_ms2_lr.py`.
- **Caricamento**: `tools/build_site.py::pack_mzfinder` mette tutti i file di `TP_Mine/js/` in `indice.json` e `tpmine-loader.js` li carica tutti all'apertura in modalità amministratore: oggi non c'è un caricamento pigro.

## 2. Decisioni di disegno
1. **Condizione = `serie` di `campioni/2`**, nessuno schema nuovo per l'assegnazione. Una condizione è un'etichetta libera («O2», «N2», «Reference», «TiO2 P25»); `serie == ""` vale per i controlli condivisi (un T0 e un bianco per tutte); un controllo con la sua `serie` vale solo per quella condizione e vince sui condivisi.
2. **Un solo allineamento** di tutti i file full (LR) o di tutte le feature (HR); niente Dig per condizione da unire dopo. Motivo: stessi nomi, stesse righe, tolleranza di RT unica, riempimento delle celle assenti già presente (`Aligned`, `gapfill.rs`). Un TP visto in una sola condizione ha comunque una cella (riempita o assente) in tutte le altre.
3. **Polarità**: righe separate per + e − (come oggi); se due righe hanno masse neutre uguali (Δ m/z = 2,0146 ± tolleranza) e RT entro la tolleranza, la riga porta «also in ESI−/ESI+» come suggerimento, senza unirle.
4. **Repliche**: area della condizione a un tempo = media delle repliche rilevate, con deviazione standard e CV; presenza = rilevata in almeno `ceil(0,5 · n)` repliche (n = repliche di quel tempo). Con una replica: nessuna statistica, e la tabella lo scrive («n = 1»).
5. **Area grezza** per i confronti (stesso metodo, stessa concentrazione iniziale); il contratto prevede `norm: "raw" | "istd"` ma GR-1 implementa solo `raw` (risposta Q1).
6. **Una sessione con una sola condizione** non cambia: niente vista nuova, stessa uscita del Dig byte per byte (criterio §8.1).
7. **Testi**: tutto Tabui in inglese, senza chiavi `lang/`; «Neutral formula» e «[M+H]+ formula», mai «Formula» da sola; aree in notazione scientifica (`1.23e+06`), come la tabella di #871.

## 3. Contratto `gruppi/1` (uscita del Rust)
Ingresso di `gruppi::build(table, campioni, opts)`: la matrice righe × file (area, altezza, stato della cella: rilevata/riempita/assente, apice RT), i metadati delle righe (id, nome TP, m/z, polarità, RT, ruolo della riga, formula neutra e ppm se HR, MS2 abbinata per file se c'è), il `campioni/2` canonico e le opzioni. Il modulo non legge file: lo chiamano il Dig LR (GR-2) e la tabella HR (GR-6).

Opzioni (`GroupOpts`, JSON `{"reference": str|null, "min_rep_frac": 0.5, "min_area": 0, "ctrl_ratio": 0.2, "norm": "raw", "shape_drop": 0.3, "factors": {}}`): condizione di riferimento (null = nessun rapporto), frazione minima di repliche, area minima (0 = spenta), rapporto massimo controllo/trattato per «absent in…» (`RATIO_MAX` di oggi), calo minimo dopo il massimo per dire «bell», `factors` = secondo campo facoltativo per condizione (`{"O2": {"catalyst": "TiO2 P25"}}`, chiavi e valori liberi; risposta Q2): si conserva e si riporta nell'uscita, nessun calcolo e nessuna vista lo usano.

Uscita (chiavi inglesi, perché è solo di Tabui):
```
{ "format": "gruppi", "version": 1, "reference": "O2" | null, "norm": "raw",
  "conditions": [ { "id": "O2", "factors": {}, "files": [3, 4, 5], "times": [60], "replicates": 1,
                    "controls": { "t0": [0], "blank": [1], "std": [2], "dark": [] },
                    "parent": { "k": 0.031 | null, "half_life": 22.4 | null, "remaining": 0.18 | null } } ],
  "rows": [ { "row": 7, "tp_name": "TP 362", "mz": 362.2, "polarity": "+", "rt": 9.84,
              "neutral_formula": null, "ppm": null, "other_polarity": null | 12,
              "present_in": ["O2", "Air"], "only_in": null | "O2", "in_all": false,
              "cond": [ { "id": "O2", "present": true, "n_det": 1, "n_rep": 1,
                          "area": 1.2e6, "area_sd": null, "cv": null, "ratio_ref": 1.0 | null,
                          "shape": "single" | "rising" | "bell" | "decay" | "flat" | null,
                          "t_max": 60 | null, "t_half_rise": null, "rate_rel": null,
                          "kin": [ { "t": 60, "area": 1.2e6, "sd": null, "n": 1 } ],
                          "prev": { "absent_t0": "pass"|"fail"|"na", "absent_blank": "...",
                                    "in_parent_std": "yes"|"no"|"na", "above_min": "...",
                                    "ms2": "linked"|"none"|"na", "verdict": "Prev. likely"|"Prev. possible"|"Prev. unlikely" } } ] } ],
  "sets": [ { "conditions": ["O2"], "rows": [7, 9] }, { "conditions": ["Air", "N2", "O2"], "rows": [3] } ],
  "warnings": [ { "code": "refMissing" | "condNoControls" | "condOnePoint" | "condNoReplicates" | "mixedTimes", "condition": "N2" } ] }
```
Regole:
- `present`: cella rilevata in ≥ `min_rep_frac` delle repliche di almeno un tempo trattato della condizione e area ≥ `min_area`. Le celle riempite contano come assenti per la presenza ma la loro area resta in `kin` (per vedere quanto «assente» è assente).
- `ratio_ref` = area(cond) / area(rif) al tempo comune più alto (endpoint); null se il riferimento non è presente (allora `only_in` lo dice) o non c'è riferimento; 0 se la condizione non è presente.
- `sets`: gli insiemi esatti di condizioni in cui la riga è presente (le colonne dell'UpSet), ordinati per numero di righe; `only_in` e `in_all` ne sono i due casi più usati.
- `shape` (≥ 3 tempi trattati): `rising` (massimo all'ultimo tempo), `bell` (massimo interno e calo ≥ `shape_drop` dopo), `decay`, `flat` (max/min < 1,5); `single` con un tempo; `t_half_rise` = primo tempo (interpolato) con area ≥ 50 % del massimo; `rate_rel` = `t_half_rise(rif) / t_half_rise(cond)` (> 1 = più veloce del riferimento). Riprende le funzioni di `crate::kinetic` dove esistono.
- `prev`: controlli della condizione (i suoi più i condivisi); `in_parent_std` = rilevata nello standard del parent con rapporto > `ctrl_ratio` (risposta Q3: solo un suggerimento, mai un'esclusione); `verdict` = «Prev. likely» se presente e tutti i criteri disponibili passano, «Prev. possible» se manca un controllo o la MS2, «Prev. unlikely» se un controllo fallisce. Mai un livello di Schymanski.
- `parent`: decadimento per condizione (`kin::decay` su quella condizione) e frazione rimasta all'endpoint rispetto al T0 della condizione.
- Determinismo: stesso JSON (stessi byte) per qualunque ordine dei file in ingresso; condizioni in ordine di prima comparsa in `campioni/2`, righe nell'ordine del Dig.
- Errori: `{error_key: "error.gruppi.*", params}` (`noConditions`, `badOpts`, `sizeMismatch`).

Schema in `docs/schemi/gruppi-1.json` (GR-1), con un file di prova `crates/mzlab-feat/tests/data/gruppi_fixture.json` che GR-3 e GR-4 usano prima che il Rust sia collegato.

## 4. Flusso dell'utente
1. **Files** (Tabui, scheda dei file): l'utente carica tutti i file. La tabella mostra File | Role | Condition | Time | Replicate; ruoli, tempi, condizioni e repliche sono proposti dai nomi (`proponi`, tempi in giallo finché non confermati, come nella Pipeline). «Condition» è un campo con `datalist` delle etichette già usate; un pulsante «Set condition…» la assegna ai file selezionati. Sotto: «Reference condition» (`select`, default: la condizione chiamata come «ref…/reference/control/no gas» se c'è, altrimenti nessuna) e gli avvisi di `valida` per condizione in `role=status`.
2. **Run**: un solo Dig. Durante il Dig la barra dice «Aligning N files of K conditions…».
3. **Table** (tabella TP di oggi): una riga per TP della lista unica; con ≥ 2 condizioni compare una colonna compatta «Conditions» (un punto per condizione: pieno = presente, vuoto = assente, con `aria-label` «present in O2, Air; absent in N2…») e il filtro «Only in… / In all / Present in…».
4. **Compare conditions** (linguetta nuova, solo con ≥ 2 condizioni; script caricati alla prima apertura):
   - matrice TP × condizione: area (notazione scientifica), rapporto col riferimento (`×2.3`, `only here`, `—`), presenza come simbolo e non solo colore, `prev` come gettone «Prev.» con la prova nel tooltip;
   - UpSet: barre degli insiemi `sets` con la matrice dei punti sotto (più leggibile di un Venn oltre 3 condizioni); clic su una barra = filtro della matrice;
   - grafico del TP scelto: endpoint → barre raggruppate per condizione (con le repliche come punti e la deviazione standard); cinetiche → curve sovrapposte, una per condizione, con il parent tratteggiato; scelta «area» o «% of max»;
   - tabella del parent per condizione (k, t½, frazione rimasta);
   - colori Okabe-Ito per le prime 8 condizioni; oltre 8 il grafico passa a piccoli multipli (un riquadro per condizione), mai colori ripetuti.
5. **Export** (in inglese): «Comparison (CSV, long)» una riga per TP × condizione × tempo; «Comparison (CSV, wide)» una riga per TP; «Sets (CSV)»; «gruppi/1 (JSON)»; grafici in SVG e PNG. Nomi: `tabui_conditions_<parent>_<yyyymmdd>.csv`, `tabui_upset_<parent>.svg`, `tabui_tp-<mz>_conditions.svg`.

## 5. Rust e JS
| Cosa | Dove |
|---|---|
| presenza, medie, CV, rapporti, insiemi, forme, `t_half_rise`, `rate_rel`, `prev`, decadimento per condizione, avvisi | Rust `crates/mzlab-feat/src/gruppi/` (GR-1) |
| cinetica e criteri del Dig per condizione, celle riempite nelle condizioni assenti, `gruppi/1` nell'uscita del Dig, «also in ESI−» | Rust `legacy/lr/` (GR-2) |
| proposta della condizione dai nomi, avvisi per condizione | Rust `campioni/` (esiste; GR-3 lo chiama) |
| formula neutra e ppm nelle righe, `gruppi/1` dalla tabella HR | Rust `tp/` (GR-6) |
| tabella dei file con «Condition», scelta del riferimento, `NB.campioni` per Tabui LR | JS `TP_Mine/lazy/` + innesto minimo (GR-3) |
| matrice, UpSet, barre/curve, esportazioni (solo formattazione e disegno) | JS `TP_Mine/lazy/` (GR-4) |
| colonna «Conditions» e filtro nella tabella TP, collegamento alla linguetta | JS `20_tpmine.js`, `42_tp_table.js` (GR-5, dopo #863/#871/#873) |

Il JS non calcola medie, rapporti né insiemi: legge `gruppi/1`. CSV e SVG si scrivono in JS dal JSON (sono formattazione), con i numeri come li dà il Rust.

**Caricamento pigro**: i file nuovi stanno in `TP_Mine/lazy/` (non in `TP_Mine/js/`, che si carica tutto all'avvio); `pack_mzfinder` li copia e li elenca in `indice.json` sotto `"lazy"`; `tpmine-loader.js` espone `TPLAZY.load(nome) → Promise`. Un e2e controlla che all'apertura di Tabui nessun file di `lazy/` sia scaricato (GR-3 crea il meccanismo).

## 6. HRMS senza rompere LR
- Stesso `gruppi::build`: le righe HR hanno `neutral_formula`, `ppm` (errore dell'[M+H]+ o dello ione scelto) e il livello della cascata come colonna a sé; in LR quei campi sono null e le colonne non compaiono.
- Identità della riga fra condizioni: in HR la tolleranza è in ppm (quella della pipeline), in LR in Da; il modulo `gruppi` non riallinea mai, usa le righe che riceve.
- La rete (`tp/rete.rs`, correlazione «lungo la serie») va calcolata per condizione quando ci sono ≥ 4 tempi per condizione; con solo endpoint non si calcola (GR-6).
- Esportazioni HR: colonne in più «Neutral formula», «[M+H]+ formula», «Δ ppm».

## 7. Rischi
| Rischio | Misura |
|---|---|
| Cinetiche mescolate fra condizioni (oggi) | GR-2 prima di ogni vista; test con 6 condizioni a un solo tempo: nessun «grows over time» |
| Deriva di RT fra decine di corse | allineamento unico con la correzione già usata (`e2e_tpmine_rtalign`); dati sintetici con deriva ±0,1 min per corsa; criterio §8.3 |
| Etichette proposte male dai nomi | proposte sempre da confermare, «Set condition…» sui selezionati; etichette proposte in giallo finché l'utente non preme «Confirm conditions» (stato della pagina, il modello non cambia); il Dig parte lo stesso |
| «Only in» da rumore vicino alla soglia | area delle celle riempite mostrata accanto; `min_area` regolabile; repliche |
| Più di 8 condizioni | piccoli multipli; matrice con intestazioni ruotate e scorrimento orizzontale interno, non della pagina |
| PR aperte su `20_tpmine.js`/`42_tp_table.js` (#863, #871, #873, tooltip senza «?») | GR-1..GR-4 e GR-6 non toccano quei file; GR-5 parte solo dopo il merge (`git log origin/main --oneline -- TP_Mine/js/42_tp_table.js`) |
| Prestazioni con 40-60 file | il Dig è già per file in worker; `gruppi::build` è lineare in righe × file; criterio §8.5 |
| Avvio più lento | nessun file nuovo in `TP_Mine/js/`; criterio §8.6 |
| Dati veri nel repository pubblico | solo dati sintetici (`tools/dati_sintetici.py gruppi`) e composti pubblici; `tests/test_esempi_anonimi.py` |

## 8. Criteri di successo (misurabili)
1. **Nessuna regressione**: con una sola condizione (o nessuna etichetta) l'uscita del Dig LR e di `tp_table` HR resta identica byte per byte ai test d'oro di oggi (`lr_dig.rs`, `finder_lr_contratto.json`, `tp_vista.rs`).
2. **Verità sintetica** (`tools/dati_sintetici.py gruppi`: 6 condizioni a endpoint + bianco + T0 + standard del parent; e 3 condizioni × 5 tempi × 2 repliche): matrice di presenza uguale alla verità 100 %; `sets`, `only_in`, `in_all` uguali; `shape` corretta per i 4 TP disegnati (rising, bell, decay, assente); `ratio_ref` entro ±10 % del rapporto delle ampiezze scritte nel generatore; nessuna riga «grows over time» con un solo tempo per condizione.
3. **Allineamento**: con deriva ±0,1 min per corsa lo stesso TP ha un solo nome in tutte le condizioni (0 righe doppie entro 0,5 Da e 0,3 min).
4. **Determinismo**: stesso `gruppi/1` (stessi byte) per 5 permutazioni dell'ordine dei file.
5. **Prestazioni**: `gruppi::build` su 2 000 righe × 60 file ≤ 50 ms nativo e ≤ 150 ms in WASM; Dig di 40 file full LR sintetici ≤ 2,2 × il tempo di 20 file (`docs/agenti/misure.md`), memoria di picco ≤ 2 × quella di 20 file.
6. **Avvio**: nessun byte in più scaricato all'apertura di Tabui (e2e: nessuna richiesta a `lazy/` prima di «Compare conditions»); il primo disegno della vista ≤ 500 ms dopo il clic con 300 righe × 8 condizioni.
7. **Accessibilità**: ogni grafico nuovo ha la sua `TDATI`, passa `e2e_axe_grafici`, bersagli ≥ 24×24 px, tastiera su matrice e UpSet.
8. **HR**: la stessa fixture HR sintetica produce `gruppi/1` con `neutral_formula` e `ppm`, e le colonne LR restano uguali.

## 9. Pacchetti e ordine
| Passo | Pacchetto | Parte dopo | Stima (giorni-chat) | Stato |
|---|---|---|---|---|
| 1 | GR-1 modulo Rust `gruppi/`, schema `gruppi-1.json`, fixture | subito | 2 | fatto: PR aperta; 2 000 righe × 60 file ≈ 40 ms nativo |
| 1 (parallelo) | GR-3 tabella dei file con «Condition», riferimento, `campioni/2` in Tabui LR, caricamento pigro | subito (usa la fixture di prova propria fino a GR-1) | 1,5-2 | fatto: componente `lazy/gruppi-campioni.js`, `TPLAZY`; prova `e2e_tabui_gruppi_file.py` su pagina isolata; l'innesto in `20_tpmine.js` resta a GR-5 |
| 1 (parallelo) | GR-4 vista «Compare conditions»: matrice, UpSet, grafici, esportazioni | subito (fixture) | 2-2,5 | da fare |
| 2 | GR-2 Dig LR per condizione, dati sintetici multi-condizione, `gruppi/1` nel `.wasm` `finder-lr` e nel desktop | merge di GR-1 | 2 | da fare |
| 2 (parallelo) | GR-6 HR: `gruppi/1` dalla tabella dei prodotti, rete per condizione | merge di GR-1 | 1,5 | da fare |
| 3 | GR-5 innesto nella tabella TP (colonna «Conditions», filtro, linguetta), MS2 per condizione, e2e completo, misura con 40 file | merge di GR-2, GR-3, GR-4 e di #863, #871, #873 | 1,5-2 | da fare |

GR-1, GR-3 e GR-4 toccano file diversi. GR-2 e GR-6 toccano `legacy/lr/` e `tp/`: nessuna sovrapposizione. Chi unisce per secondo fa `git merge origin/main` e prova di nuovo.

## 10. Domande per Federico e risposte
Risposte di Federico: Q1 area grezza; Q2 un'etichetta libera, con un secondo campo facoltativo nel contratto (`factors`, §3) senza vista; Q3 sì, standard del parent solo come suggerimento «Prev.», mai esclusione; Q4 T0 e bianco uno solo per tutte le condizioni come default, gestito anche uno per condizione.
- **Q1. Area grezza o standard interno?** Per confrontare le condizioni uso l'area così com'è (stesso metodo, stessa concentrazione di partenza). *Consiglio: area grezza ora; lo standard interno si aggiunge dopo se lo usate (il campo `istd` esiste già).*
- **Q2. Una condizione è una sola etichetta o due fattori?** Per esempio «TiO2 + O2» come etichetta unica, oppure «fotocatalizzatore» e «gas» separati da incrociare. *Consiglio: un'etichetta libera ora; il secondo fattore come campo facoltativo `factors` di `gruppi/1`, senza vista.*
- **Q3. Lo standard del solo parent fa da controllo?** Un segnale presente anche nello standard al RT del parent è quasi sempre un frammento in sorgente o un'impurezza. *Consiglio: sì, ma solo come suggerimento «also in the parent standard», senza togliere la riga.*
- **Q4. T0 e bianco: uno per tutte le condizioni o uno per condizione?** (Per esempio un T0 dopo il gorgogliamento di ciascun gas.) *Consiglio: il programma gestisce entrambi i casi; per i nomi dei file che non dicono la condizione, i controlli si considerano condivisi.*
