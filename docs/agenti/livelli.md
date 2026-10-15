# Livelli di identificazione (Tabui, Attributio, flusso untargeted)

Stato corrente delle definizioni scelte da Federico. Valgono per Tabui (MS2 a bassa risoluzione), Attributio e il flusso untargeted di mzFinder. Il codice e gli altri file d'area che le descrivono in modo diverso sono elencati in fondo, in «Differenze da decidere».

## Scala di Schymanski (livello di identificazione)
- **2a**: corrispondenza dello spettro MS2 ad alta risoluzione con una libreria sperimentale (mzCloud, MassBank, MoNA).
- **2b**: struttura probabile supportata da frammenti diagnostici e da spettri in silico (MetFrag, CFM-ID, SIRIUS/CSI:FingerID), oppure da evidenze di contesto chimico univoche senza standard in libreria. **Solo il chimico assegna il 2b, mai il programma.**
- **3a**: pochi candidati posizionali/isomerici tra omologhi.
- **3b**: classi generiche con molti isomeri plausibili.
- **Tabui**: la decisione D6 è «no»: una MS2 abbinata con frammenti spiegati dal genitore non alza la riga al livello 3; resta al suo livello con la prova scritta.

## Due scale, due etichette
- Scala di Schymanski: etichetta «L3» (livello + numero, sottolivello in minuscolo: «L2a», «L3b»).
- Scala di previsione di Löffler et al., ES&T 59:19095 (2025): etichetta «Prev. 2» (gradi 1-4).
- Sono scale diverse e nell'interfaccia non si confondono mai: la stessa cifra con etichette diverse («L3» contro «Prev. 3») è normale.
- Il programma propone un livello e mostra le prove; il livello deciso resta vuoto finché il chimico non decide.

## Differenze da decidere
Punti che oggi usano definizioni o etichette diverse da quelle sopra (percorso:riga).

**Etichette**
- `TP_Mine/js/21_tpmine_hr.js:29`: il badge di Schymanski mostra la sola cifra (`2a`, `3`), senza «L».
- `TP_Mine/js/21_tpmine_hr.js:27` e `:56`: la scala di previsione è il badge «P1…P4» e la colonna «Pred.», non «Prev. N».
- `docs/agenti/mzfinder.md:42-43`: descrive la previsione come «Pred.», badge `P1…P4`.
- `mzlab/web/lang/it.js:2045-2051` (e `en.js`): `lv.name.*` dà nomi senza «L» («Probabile (libreria)», «Provvisorio»).

**Sottolivelli 3a/3b: assenti**
- `mzlab/web/livelli-1.js:12`: la scala è `["1","2a","2b","3","4","5","0"]`, un solo «3».
- `mzlab/web/livelli-1.js:88-91`: quattro basi diverse (conflitto di nomi, isomeri non discriminati, isomero di RT, revisione dell'esperto) e CSI:FingerID danno tutte «3», senza distinguere pochi candidati (3a) da classi generiche (3b).
- `crates/mzlab-feat/src/chem/frammenti.rs:136` e `mzlab/web/disegno-parziali.js:7`: «level 3 of Schymanski: a candidate class», cioè la definizione di 3b, senza sottolivello.
- `crates/mzlab-feat/src/attributio/readers/feature_decisions.rs:5` e `:157`: il chimico può scrivere solo `2`, `2b`, `3` o vuoto; `3a`/`3b` non sono ammessi.
- `docs/agenti/attributio.md:15`: `livelli_attesi.csv` ha la colonna `level_sub`, ma `rules` e le decisioni in Rust non conoscono 3a/3b.

**2a: libreria sperimentale HR**
- `mzlab/web/livelli-1.js:85` e `docs/agenti/alta-risoluzione.md:30`: il 2a chiede soglie (coseno/entropia, picchi comuni, ppm) su «una fonte di libreria valida», senza limitarla a librerie sperimentali HR (mzCloud, MassBank, MoNA); le librerie in silico non sono escluse per regola.
- `crates/mzlab-feat/src/evid/graph.rs:20`: `MAX_LEVEL_WITH_MS2 = "2a"` dà 2a a un'identità con MS2 in HR; `docs/agenti/mzfeat-prove.md:10` lo ripete.

**2b: chi lo assegna e da dove nasce**
- `mzlab/web/livelli.js:91-95`, `lang/it.js:2073`, `docs/agenti/alta-risoluzione.md:29`, `docs/agenti/mzfinder-untargeted.md:10`: il 2b si scrive solo «da 2a o da 3» e richiede una prova diagnostica «su uno spettro che ha già un candidato». La definizione sopra lo ammette anche senza candidato in libreria (evidenze di contesto chimico univoche, spettri in silico).
- `mzlab/web/livelli-1.js:86` e `:111`: la base del 2b è solo «diagnostic evidence», senza spettri in silico né contesto chimico.
- `crates/mzlab-feat/src/attributio/readers/feature_decisions.rs:151-187`: il 2b chiede solo `motivazione`, non la distinzione tra frammenti diagnostici, spettri in silico e contesto.

**Tabui (D6)**
- `docs/studi/TABUI_MS2_LR.md:82` e `:92`: D6 è ancora una domanda aperta («(a) no… (b) sì»), non la decisione «no».
- `docs/studi/TABUI_MS2_LR.md:12` e `crates/mzlab-feat/src/legacy/lr/summary.rs:16`: in LR le MS2 non cambiano il livello (5/4), coerente con D6 = no; `mzlab/web/livelli-1.js:125` però fissa il tetto LR a «3».

**Altri riferimenti alla scala**
- `mzlab/web/rapporto.js:6` e `lang/it.js:3411`: la sezione 5 del rapporto riporta i «livelli di Schymanski» senza dire che la previsione è un'altra scala.
