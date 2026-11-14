# File d'area: quale leggere
Una riga per file d'area, in ordine di nome file (max 300 caratteri): se tocchi la zona a sinistra, leggi il file a destra (solo quello). Un file nuovo = una riga nuova al suo posto. Trovare una riga: `grep -n '<parola>' docs/agenti/INDICE.md`. Ogni `docs/agenti/*.md` ha la sua riga; ordine e una riga per file sono controllati da `tools/gate_release.py`.

| Se tocchi… | Leggi |
|---|---|
| accessibilità dell'intera applicazione (WCAG 2.1/2.2 AA, axe-core, controlli, lacune note e regole tollerate) | `docs/agenti/accessibilita.md` |
| Agilent `.D` LC/MS: QqQ MRM/dMRM, Q-TOF profilo/centroidi/MS2, metodo LC/MS, DAD (`crates/mzlab-core/src/agilent_ext/`, feature `estesi`) | `docs/agenti/agilent-lcms.md` |
| lettore cartelle Agilent `.D` (`crates/mzlab-core/src/agilent/`) | `docs/agenti/agilent.md` |
| statistica in HR: tabella delle feature, bianco, differenza dal riferimento, PCA | `docs/agenti/alta-risoluzione-statistica.md` |
| mondo HR: Banco, formule, isotopi, librerie, albero MSn | `docs/agenti/alta-risoluzione.md` |
| lanciare la verifica nel container cloud (Chromium, `--setup`, test col sito costruito) | `docs/agenti/ambiente-cloud.md` |
| animazione di caricamento (`base`) e analizzatori animati spenti (`an-*.js`: quadrupolo, TOF, Orbitrap, ESI, trappola, FT-ICR, QqQ, Paul, settore, IMS, MALDI) | `docs/agenti/analizzatori.md` |
| proxy pubblico anonimo dell'assistente (`proxy/`, Worker Cloudflare, `Ledger` della spesa, porte, elenco degli hash) | `docs/agenti/assistente-proxy.md` |
| Banco 2b (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-banco.md` |
| esportazioni (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-export.md` |
| isotopi e composizione elementare (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-isotopi.md` |
| motore MS2 sperimentale, frammentazione in silico alla MetFrag, attribuzione dei frammenti alla MS-FINDER, previsione dell'RT (`attributio/ms2_engine/`) | `docs/agenti/attributio-ms2.md` |
| parità con mzmine (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-mz-parita.md` |
| binario MZ (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-mz.md` |
| pannello (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-pannello.md` |
| revisione, abbinamento e identità (area: Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di…) | `docs/agenti/attributio-revisione.md` |
| Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di rilevamento alla mzmine: `crates/mzlab-feat/src/attributio/`, `untargeted/`, `.wasm` `attributio*` e `untargeted`) | `docs/agenti/attributio.md` |
| avvio progressivo (`AVVIO`), regola «interfaccia subito»; area e tabella di caricamento (area: header (schermo intero, logo, (i)), area e tabella di caricamento,…) | `docs/agenti/avvio.md` |
| azioni dei pannelli (registro `azioni/1`, menu del tasto destro, Maiusc+F10) | `docs/agenti/azioni.md` |
| picchi spalmati: lotto sintetico, configurazioni MZmine default/wide, tabella di confronto (`tools/bench/confronto_larghi.py`) | `docs/agenti/bench-picchi-larghi.md` |
| lettore Bruker timsTOF `.d` (`crates/mzlab-lettori/src/bruker/`: SQLite minimo, zstd, `tdf_bin`/`tsf_bin`, calibrazione m/z, `BrukerD` in `agilent-d.js`) | `docs/agenti/bruker.md` |
| campi numerici scritti a mano (`campi.js`, `CAMPI.num`, `data-num`, calcolatori, nota di rifiuto `campo.*`, `numIt` senza Infinity, `lib_campi.sweep`) | `docs/agenti/campi-numerici.md` |
| ruoli dei file, tempi e metadati dei campioni (`campioni/2` e `/3`, `mzlab-core/src/samples/`: proposte dai nomi, modelli di nome, CSV, colonne libere, dati dal file, avvisi, migrazione, indici per il percorso TP) | `docs/agenti/campioni.md` |
| canali dei dati (file → canali: TIC, filtri/transizioni/precursori, UV, analogici; tipo di esperimento come etichetta; lavori aperti) | `docs/agenti/canali-dati.md` |
| capacità del browser, feature detection, avviso «non disponibile in questo browser: usa l'app» (`capacita.js`, `?cap-off=`) | `docs/agenti/capacita.md` |
| mappa del percorso di apertura di ogni file (clic → schermo, sito e app, per formato), strategie presenti/assenti, lotti di file per assistenti esterni (`tools/batch_gemini.py`) | `docs/agenti/caricamento-mappa.md` |
| shard pytest della CI (cache `target`, `doppi`, `aree`, `prebuilt`, ordine e metà dei test lenti) | `docs/agenti/ci-selezione.md` |
| workflow GitHub condizionali (concurrency, pages, job saltati) | `docs/agenti/ci-veloce.md` |
| classi chimiche da regole (`crates/mzlab-feat/src/classes/`, tabella `data/classi-1.json`, `classi/1`: frammenti diagnostici e perdite neutre) | `docs/agenti/classi.md` |
| `mzlab-cli convert`: file e cartelle dei produttori in mzML, `--peaks`, `--report`, codici d'uscita | `docs/agenti/cli-convert.md` |
| passo per passo (area: riga di comando `mzlab-cli features` (`crates/mzlab-cli`: feature…) | `docs/agenti/cli-passi.md` |
| riga di comando `mzlab-cli features` (`crates/mzlab-cli`: feature table in CSV, stesso motore e ricetta della pagina) | `docs/agenti/cli.md` |
| Safari e iPadOS (WebKit): API × versione minima × ripiego, pavimento 16.4, ripieghi di `compat.js` (`AbortSignal.any`, `createWritable`, appunti, `accept`); memoria Safari, iPad, iPhone (budget senza `deviceMemory`, rifiuto calmo, `estimate::check_reader`) | `docs/agenti/compat-safari.md` |
| rivelatore DAD (UV/Vis) del `.wiff`: matrice, mappa RT-λ, spettro UV, TWC/XWC (`xwc.js`), integrazione di una XWC, DAD sul pannello MRM (`dad-mrm.js`) | `docs/agenti/dad.md` |
| calcolatrice (gruppi «cosa può essere» sotto il numero), pannello Addotti, fatti sui dati del 3200, gradiente, tracce analogiche, tabelle dei dati, tema | `docs/agenti/dati-3200.md` |
| barra laterale della vista Dati, strumenti, suggerimenti, tour | `docs/agenti/dati-barra.md` |
| primo caricamento di più file (si apre solo il primo), campi della tabella di carico | `docs/agenti/dati-carico.md` |
| carica una cartella, cartella puntata e ricordata (elenco in grigio, «Altri», desktop), riscaldamento in background dei file della cartella puntata (coda, `lr_warm`, `lr_warm_claim`) | `docs/agenti/dati-cartella.md` |
| integrazione dei picchi nella vista Dati (strumento automatico, un clic = tutti i picchi in HR, parametri, `auto_integrate`) | `docs/agenti/dati-integrazione.md` |
| mappa RT×m/z, mirino, navigazione | `docs/agenti/dati-mappa.md` |
| liste, correzione, flusso di lavoro, QA/QC, statistica, rapporto | `docs/agenti/dati-strumenti.md` |
| vista Dati in bassa risoluzione: caricamento, schede, TIC/XIC/spettro (finestra XIC di default `xicWin`), integrazione | `docs/agenti/dati.md` |
| pseudo-spettri MS2 da DIA (`dia.rs`, `spectra.ms2` con `dia_pseudo`) | `docs/agenti/dia.md` |
| errori di lettura dei file: causa breve, «Dettagli», «Copia diagnosi» (`diagnosi.js`), mzML sintetico stile msconvert | `docs/agenti/diagnosi.md` |
| menu del tasto destro del Disegno (`#ctx`, `disegno-menu.js`, registro `menuAdd`, «Informazioni…», ricerche PubChem/ChEBI/BioTransformer, `sel/1`, `link/1`) | `docs/agenti/disegno-menu.md` |
| Disegno (Ketcher) | `docs/agenti/disegno.md` |
| esportazione di tutti i dati dell'utente: inventario dello storage nel browser, scrittore ZIP (`zip.rs`); esportazioni a pezzi in OPFS (`dl()`, `export-pezzi.js`, `export-worker.js`, provenienza, pulizia di `export/`) | `docs/agenti/esporta-dati.md` |
| contratto dei lettori dei produttori (`readers.rs`, trait `Lettore`, registro, `Limiti`), aggiungere un formato | `docs/agenti/estendere.md` |
| copertura di Tabui (area: Flow grafico (scheda: palette dei passi `step/1`, catena, moduli…) | `docs/agenti/flow-copertura-tabui.md` |
| Flow grafico (scheda: palette dei passi `step/1`, catena, moduli dei parametri, confronto `meta.compare`, esportazione/importazione `mzflow/2`; `flow/`) | `docs/agenti/flow-grafica.md` |
| scheduler a stati del Flow «live» (`mzlab_core::flow::sched`, `run_sched.rs`) | `docs/agenti/flow-live.md` |
| griglia dei metadati dei campioni (area: Flow grafico (scheda: palette dei passi `step/1`, catena, moduli…) | `docs/agenti/flow-meta-griglia.md` |
| corrispondenza con i passi di mzmine (area: Flow grafico (scheda: palette dei passi `step/1`, catena, moduli…) | `docs/agenti/flow-mzmine.md` |
| copertura di Tabui, pacchetto (area: Flow grafico (scheda: palette dei passi `step/1`, catena, moduli…) | `docs/agenti/flow-pacchetto-tabui.md` |
| Flow: nomi, gruppi e carte di vista del modello MSn infusione | `docs/agenti/flow-ux.md` |
| esecutore web del Flow («This browser»: `flow/lazy/flow-web-{run,ui,worker}.js`, eventi `flow:web-step`, stima `estimate_series`, avviso su telefono/tablet, Run spento oltre il livello alto) | `docs/agenti/flow-web.md` |
| derivati (`.mzidx`, `.mzfeat`, `.mzfeatf`) di formato più nuovo del programma: `error.newerFormat`, sola lettura | `docs/agenti/formato-nuovo.md` |
| cosa i file grezzi contengono e non estraiamo ancora (regola «nessuna perdita») | `docs/agenti/grezzi-non-estratti.md` |
| raggruppamento degli ioni di un composto (`features.group`, `ionnet/1`: correlazione, isotopi, ion identity networking, archi per GNPS; `crates/mzlab-feat/src/ionnet/`) | `docs/agenti/gruppi-ioni.md` |
| «Group by» della lista file (`#sb-by`, `gruppi.js`, `gruppi-per.js`, chiave, «Then by», «Contains», segue i file nuovi, annulla) | `docs/agenti/gruppi-per.md` |
| gruppi di esperimenti in Tabui: condizioni, `gruppi/1`, `crates/mzlab-feat/src/groups/`; vista «Compare conditions» (`TP_Mine/lazy/gruppi-vista.js`: matrice TP × condizione, UpSet, grafici, esportazioni) e innesto in Tabui LR | `docs/agenti/gruppi.md` |
| pulsante «XIC» e lente XIC spenti sui file d'infusione, regola dei dati `infusion.rs` (chiavi, blocchi, nome), comando `infusion_diagnosis`, disegno dei tracciati dalla finestra (`decima.js`); scheda «Fragmentation graph» nella vista Dati | `docs/agenti/infusione-grafo.md` |
| inglese nel codice (identificatori, commenti, nomi di file e cartelle): inventario misurato, `tools/check_english.py`, `tools/italian_words.tsv`, mappa delle rinomine (`tools/english_rename_map.py`), piano a lotti | `docs/agenti/inglese.md` |
| pacchetto dei candidati in silico (`mzlab-cli insilico pack`, `insilico-scarica.js`, `flow-web-insilico.js`, `crates/mzlab-desktop/src/insilico.rs`, `spectra.insilico` nel browser) | `docs/agenti/insilico.md` |
| interpretazione per riga dei TP (`crates/mzlab-feat/src/interp/`, `interp/1`, `filtri/1`, blocco `interp` nel `.mzfeat`, `.wasm` `interp`, rotte `table_interp`) | `docs/agenti/interp.md` |
| header (schermo intero, logo, (i)), area e tabella di caricamento, avvio progressivo (`AVVIO`), regola «interfaccia subito»; audit delle pagine (CLS, pannelli che si spostano, memoria, errori; `tests_e2e/audit_pagine.py`, `e2e_giro_intestazione`) | `docs/agenti/intestazione.md` |
| modulo «andi» (netCDF/ANDI `.cdf`: `crates/mzlab-lettori/src/andi/`) | `docs/agenti/lettori-andi.md` |
| modulo «aperti» (mzML con numpress, mzXML: `crates/mzlab-lettori/src/open/`, `numpress.rs`) | `docs/agenti/lettori-aperti.md` |
| moduli lettori, uno per famiglia (`crates/mzlab-lettori*`, `.wasm` a parte in `wasm-lettori/<famiglia>/`, feature `estesi`, `LETTORI_FAM`, regola R in `shell.js`, `LETTORI_MOD`, `?lettori=modulo`, tetti) | `docs/agenti/lettori-modulo.md` |
| Thermo `.raw`: attivazione, «unknown», tabella eventi rifiutata, scansioni e controller danneggiati (`warn.raw.damaged`), precursori (target, ione selezionato, catene MSn, SPS, MSX, SRM dall'evento) | `docs/agenti/lettori-thermo-precursori.md` |
| lettura nativa di `.raw`, `.wiff`, `.D` nel sito (campioni, OPFS, avanzamento, mzML su richiesta) | `docs/agenti/lettura-nativa.md` |
| formato `.mzlib`, parser delle librerie, indici e punteggi (`crates/mzlab-lib`) | `docs/agenti/librerie.md` |
| testi, `lang/`, `help.js`, inglese | `docs/agenti/lingue.md` |
| liste dell'utente (trasformazioni, addotti, sospetti: CSV con errori per riga, `liste/1`, SHA-256 per `prov/1`) | `docs/agenti/liste.md` |
| livelli di identificazione (scala di Schymanski 2a/2b/3a/3b, scala di previsione, etichette «L3» e «Prev. 2», Tabui, Attributio, untargeted) | `docs/agenti/livelli.md` |
| assistente LLM e agenti esterni: principi, scelte, rischi, proposte non realizzate | `docs/agenti/llm-scelte.md` |
| subagenti, letture ed esecutore dei lettori sulla pagina `llm-agente.js` (area: assistente LLM: catalogo `strumenti/1`) | `docs/agenti/llm-subagenti.md` |
| assistente LLM: catalogo `strumenti/1` | `docs/agenti/llm.md` |
| velocità della mappa: misure dei gesti (`tools/bench_map.py`), diagnosi e lavori aperti; visore 3D (`mappa3d.js`, `failure3d`, comandi e menu, precarico `preload3d`) | `docs/agenti/mappa-fluida.md` |
| motore di confronto spettrale con più librerie per mzFlow (nodo «Attribution», `table.attribute`) e pannello laterale MSⁿ: `libset`, `libmatch/1`, preset, analoghi con ΔFormula, catalogo `librerie/2` | `docs/agenti/match-spettrale.md` |
| memoria: regole di scelta del budget, esperienza d'uso dell'avviso, proposte aperte (`memoria-avviso.js`) | `docs/agenti/memoria-scelte.md` |
| contratto di memoria dei moduli WASM (`mzlab-core::budget`, uscite a offset, lastre, flusso delle prove) | `docs/agenti/memoria.md` |
| metodo di acquisizione (`metodo/1`, `method.rs`, `method_wiff`, `method_raw`, schema) | `docs/agenti/metodo.md` |
| metriche di main nel tempo (`tools/metriche.py`, ramo `stat-ci`, tendenze) | `docs/agenti/metriche.md` |
| caricamento della pagina (FCP, LCP, motore pronto, KB prima del paint per scenario; `tests_e2e/misura_primo_paint.py`) | `docs/agenti/misure-caricamento.md` |
| Misure: diario (NON LEGGERE) [titolo del file] | `docs/agenti/misure-diario.md` |
| formati di ingresso del Flow (area: misure dei motori, validazione delle esportazioni (mzTab-M), clic…) | `docs/agenti/misure-formati.md` |
| costo del disegno dei cromatogrammi su tracce dense (`tools/bench_charts.py`: 10^6 punti, 20 file, rotella, trascinamento, ridimensionamento, linea); tabella HR di 100 000 righe nella «Statistica» (`tools/bench_table_hr.py`: KMD, clic, «Dati») | `docs/agenti/misure-grafici.md` |
| misura su computer lenti (CPU rallentata 1×/4×/6×, 2/4 core: apertura demo, trascinamento della linea, mappe, XIC, memoria; `tests_e2e/misura_macchine.py`) | `docs/agenti/misure-macchine.md` |
| misure Rust e dei moduli (area: misure dei motori, validazione delle esportazioni (mzTab-M), clic…) | `docs/agenti/misure-rust.md` |
| lettori dei produttori, `.raw`, `.wiff`, `.D` (area: tabella cumulativa dei guadagni di velocità (data · PR · passo ·…) | `docs/agenti/misure-velocita-raw.md` |
| tabella cumulativa dei guadagni di velocità (data · PR · passo · prima · dopo · %) | `docs/agenti/misure-velocita.md` |
| misure dei motori, validazione delle esportazioni (mzTab-M), clic del flusso Tabui e della rete | `docs/agenti/misure.md` |
| record della struttura chimica `mol/1` (`crates/mzlab-mol`: tipi, JSON, hash del nucleo; lettori SMILES/molfile; descrittori CIP R/S ed E/Z, `cip/1`, `disegno-cip.js`) | `docs/agenti/mol.md` |
| database delle molecole (`crates/mzlab-core/src/molecules/`, `molecules/1`: record, CSV e JSON canonico, ricerca, livelli catalogo/importata/utente, catalogo `mzlab/web/molecole/catalogo.csv`, rotte `api/molecules/*`) | `docs/agenti/molecules.md` |
| grafo di frammentazione MSn dell'infusione (`msngrafo/1`: ioni uniti fra livelli ed energie, formule proposte dei frammenti, scheda «Fragmentation graph») e libreria di alberi MSn degli standard (`msnlib/1`, protocollo di acquisizione) | `docs/agenti/msn-grafo.md` |
| interpretazione dei frammenti MSn (perdite, regole, rotture), formule dei frammenti (`msngraph/formulas.rs`, `candidates.rs`, `annotate.rs`: vincolo DAG), tagli per un nodo (`strutture`), chiave `vista` (cammino migliore, disposizioni, `completa`) | `docs/agenti/msn-interpreta.md` |
| elenco degli esperimenti MSn di un file DDA, finestra «Albero MSn» (schede Experiments/Tree, `msn-elenco.js`, colonne `lev`/`z` di `/api/dda`); file di sole MS3 (`msn_level`, livello, `/api/ms2tr`, titoli); figura del grafo MSn; prove delle regole (`msn-rules`) | `docs/agenti/msn-manuale.md` |
| libreria (area: grafo di frammentazione MSn dell'infusione (`msngrafo/1`: ioni…) | `docs/agenti/msnlib.md` |
| prove di mutazione (`cargo-mutants`) di `mzlab-lettori` e del modulo wiff: comandi, campioni, sopravvissuti e perché restano | `docs/agenti/mutanti.md` |
| bande larghe (`broad.rs`), opzioni del passaggio mirato (`target.rs`: picchi stimati, controllo di forma /\, fusione di righe sovrapposte), correzione e allineamento dell'RT (`rtmap.rs`, `rtwarp.rs`), cinetica come prova (`kinetic.rs`, `kinetic/`) | `docs/agenti/mzfeat-broad.md` |
| dipendenze fra i moduli di `crates/mzlab-feat` (ciclo grande, archi di ritorno, tagli, separazione in crate del motore di attribuzione) | `docs/agenti/mzfeat-cicli.md` |
| pipeline delle feature a bassa risoluzione (full scan, asse in Da, profilo `lr`: `recipe_lr.rs`, `axis.rs`) | `docs/agenti/mzfeat-lr.md` |
| prove e misure (area: pipeline delle feature HR in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-prove.md` |
| Feature table nella Pipeline (passi 5-6: QC, barra dei filtri, `PIPET` con `TPTAB` e riga espansa `TPRIGA`) | `docs/agenti/mzfeat-tabella-vista.md` |
| tabella, id e campioni (area: pipeline delle feature HR in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-tabella.md` |
| vista dei prodotti `tp/1` (area: pipeline delle feature HR in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-tp.md` |
| pannello «Pipeline» e famiglia di un file a richiesta, `family_record` (area: mzFinder nel browser (`crates/mzlab-feat-wasm`,…) | `docs/agenti/mzfeat-wasm-pannello.md` |
| mzFinder nel browser (`crates/mzlab-feat-wasm`, `TP_Mine/files/feat-worker.js`, pannello Pipeline `TP_Mine/js/32_pipeline.js`, stima dei parametri `crates/mzlab-feat/src/params.rs`) | `docs/agenti/mzfeat-wasm.md` |
| pipeline delle feature HR in Rust (`crates/mzlab-feat`) | `docs/agenti/mzfeat.md` |
| benchmark di mzFinder contro altri strumenti (`tools/bench/`, tabella `bench/1`, metriche P1-P5, statistica) | `docs/agenti/mzfinder-benchmark.md` |
| ingresso di mzFinder: `.mzidx` per ogni file scelto o da IndexedDB (`TP_Mine/files/finder-idx.js`), lista di trasformazioni dell'utente in Rust | `docs/agenti/mzfinder-ingresso.md` |
| scavo LR (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-lr.md` |
| interfaccia e regola MS2, bersagli MS2 e passo «1 · Files» (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-ms2.md` |
| mzFinder: tabella dei prodotti HR (`TPTAB`, `TPRIGA`, `TPDEC`), Dig in diretta (`js/24_tpmine_diretta.js`, `TPLIVE`), coppie progenitore-TP e interfaccia HR (`js/21_tpmine_hr.js`, `mass_network`, previsione) | `docs/agenti/mzfinder-tabella.md` |
| tabelle untargeted, identità e lettori delle tabelle terze parti (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-untargeted.md` |
| validazione (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-validazione.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` |
| formato `.mzidx` | `docs/agenti/mzidx.md` |
| matrice di conformità mzML (varianti, fixture `tools/mzml_variants.py`, buchi noti) | `docs/agenti/mzml-matrice.md` |
| file `mzrun/1` (`*.mzrun.json`: record e preset di un'analisi, impronta `self_sha256`, regole di applicazione, schema `docs/schemi/mzrun-1.json`) | `docs/agenti/mzrun.md` |
| costruttore delle liste normative UE e USA (`mzlab-cli normative build`, `crates/mzlab-cli/src/normative/`, `normative/sources.json`, fonti, pin, `resolve`, uscita) | `docs/agenti/normative.md` |
| estremi di array grandi senza spread, ricerca binaria dei punti in una finestra (`numeri.js`: `maxOf`, `minOf`, `maxAbs`, `spanOf`) | `docs/agenti/numeri.md` |
| tentativi scartati, tagli della CI già misurati e tempi dei job Rust (area: tabella cumulativa dei guadagni di velocità (data · PR · passo ·…) | `docs/agenti/ottimizzazioni-tentate.md` |
| affiancare e spostare i pannelli (maniglia, zone di rilascio, divisori, annulla) | `docs/agenti/pannelli-affianca.md` |
| parità sito ↔ app (funzioni e passi del Flow presenti da una parte sola, operazioni del motore assenti nell'app, test di parità mancanti, prossimi passi); funzione × faccia × prova, guardia delle op di `rust-worker.js` senza ramo (`test_parita_operazioni.py`) | `docs/agenti/parita.md` |
| agenti esterni (Claude Code, Cowork) che usano `mzlab-cli` dalla shell: sequenza, codici, esempi di piani | `docs/agenti/per-agenti.md` |
| lista integrata di pesticidi e contaminanti ambientali (`mzlab/chem/pesticides.csv`, `pesticides_meta.json`, `tools/download_pesticides.py`, seconda lista di `contaminanti.json`) | `docs/agenti/pesticidi.md` |
| posizione della sostituzione negli isomeri dei TP (classi compatibili con la MS2, `chem/positions.rs`, `posiz/1`) | `docs/agenti/posizioni.md` |
| barra di caricamento unica per ogni formato (`progresso.js`, `PROGRESSO`, evento `mzlab-index`, pesi delle fasi, prove di monotonia) | `docs/agenti/progresso.md` |
| `qc.mass_accuracy` (accuratezza di massa, correzione della tabella, controllo della calibrazione dal tasto destro del TIC `masscal.js`: `crates/mzlab-feat/src/massacc/`, `flow_massacc.rs`, `flow-web-qc.js`, `flow-views-qc.js`) | `docs/agenti/qc-massa.md` |
| conversione `.raw` → mzML: banco `raw_mzml_bench`, `tools/bench/raw_mzml.py`, T_min, punti di partenza V1-V9, cause di D8 e D11, caso denso sintetico, codifica in parallelo (`convert_times`) | `docs/agenti/raw-mzml.md` |
| validazione sui siti sperimentali ed esperimenti scartati (area: motore di reattività del genitore (crates/mzlab-chem, .wasm chem,…) | `docs/agenti/reattivita-studi.md` |
| motore di reattività del genitore (crates/mzlab-chem, .wasm chem, reatt/1, viewer 3D di Tabui) | `docs/agenti/reattivita.md` |
| misure del reset del browser («tutto», «files», `qqHardReset`; `tests_e2e/misura_reset.py`, sonda `lib_reset.py`), righe MISURA reset_* | `docs/agenti/reset-misure.md` |
| rete dei prodotti (`crates/mzlab-feat/src/network/`, `rete/1`: archi per regola su `tp/1`, passi, coordinate, motivo dell'arco) | `docs/agenti/rete.md` |
| rilascio: tag `vX.Y.Z`, `release.yml`, `CHANGELOG.md`, versione unica, SBOM, cancello (`gate_release.py`); Dependabot, CodeQL, impostazioni di sicurezza da accendere a mano | `docs/agenti/rilascio.md` |
| mzFinder nativo (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-desktop-finder.md` |
| desktop (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-desktop.md` |
| piani `mzflow/2`, controllo e `prov/1` (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-flow.md` |
| mzidx (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-mzidx.md` |
| rotte (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-rotte.md` |
| descrittori `step/1` (area: motore Rust (`crates/`, dati d'oro `tools/golden.py`)) | `docs/agenti/rust-step.md` |
| motore Rust (`crates/`, dati d'oro `tools/golden.py`) | `docs/agenti/rust.md` |
| scandump, oracolo RFR | `docs/agenti/scandump.md` |
| service worker, cache per versione del sito, `sw-manifest.json`, aggiornamenti | `docs/agenti/service-worker.md` |
| lettore Shimadzu `.qgd` e `.lcd` (`crates/mzlab-lettori/src/shimadzu/`, modulo `shimadzu`) | `docs/agenti/shimadzu.md` |
| indice sospetti (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/sospetti.md` |
| statistica come passi: Welch/ANOVA con BH, PCA e KMD in Rust (`crates/mzlab-feat/src/stat/`, `table.stats`, `table.pca`, `table.kmd`, `stat/1`) | `docs/agenti/stat.md` |
| registro delle azioni che cambiano stato (`comandi.js`, `docs/azioni.md` generato), annulla/ripeti, voci «tolto» (togliere un file, chiudere un pannello, svuotare le marcature: nessun avviso, voci di `STORIA`) | `docs/agenti/storia-tolto.md` |
| studio spettrale su dati sintetici: generatore di alberi di frammentazione, scenari E0-E6, bracci contro il motore (null calibrato, p-value di coincidenza, sotto-formule, analoghi a più spostamenti), righe MISURA studio_* | `docs/agenti/studio-spettri.md` |
| tabella picchi dello spettro da copiare (TSV), asse y in % fino a 100, pulsante XIC solo su MS¹ | `docs/agenti/tabella-picchi.md` |
| dove Tabui perde un prodotto atteso (`mzlab-cli explain-missing`) | `docs/agenti/tabui-spiega.md` |
| struttura dei test, test nuovi | `docs/agenti/test.md` |
| velocità del lettore Thermo `.raw`: criteri A1-A7 e B1-B5, banco `thermo_bench`, `tools/bench/thermo_velocita.py`, numeri di partenza contro RFR (T-7) | `docs/agenti/thermo-velocita.md` |
| fondo chimico del TIC/BPC (ioni di fondo persistenti, linea di base per ione, finestra tranquilla, bianco, finestre m/z escluse: `ticbg.rs`, `api/tic_background`, `mzlab-cli background`) | `docs/agenti/tic-fondo.md` |
| titoli dei pannelli (regole uniche per tipo di pannello × segnale, `titoli.js`, etichetta «spettro di», nomi dei file esportati) | `docs/agenti/titoli-pannelli.md` |
| tabella delle transizioni SIM/MRM di un triplo quadrupolo (`chrom.transitions`, `transitions/`, integrazione per campione, candidati di RT, preset di esempio) | `docs/agenti/transizioni.md` |
| pulsante «XIC» nella barra dello spettro, lente XIC al passaggio sul picco (`xic-lente.js`), banco e tabella di partenza dei tempi del visualizzatore (`tools/bench_viewer.py`, `viewer_times.rs`) | `docs/agenti/visore-veloce.md` |
| potenziale di WebAssembly (profili `opt-level`, SIMD128, binaryen, zlib-rs, thread, memory64, browser minimi; misure) | `docs/agenti/wasm-potenziale.md` |
| misure dei .wasm (campo `misura` di `tools/moduli/*.toml`, riga prima → dopo in PR, cosa costa di più) | `docs/agenti/wasm-tetti-misure.md` |
| tetti dei .wasm di mzFinder, moduli fusi (gruppo HR e Attributio) | `docs/agenti/wasm-tetti.md` |
| lettore cartelle Waters MassLynx `.raw` (`crates/mzlab-lettori/src/waters/`, modulo `waters`, raggruppamento in `waters-raw.js`) | `docs/agenti/waters.md` |
| altri strumenti SCIEX, sMRM, PIS/NL, feature `estesi` (area: lettore `.wiff` + `.wiff.scan` (`crates/mzlab-core/src/wiff/`)) | `docs/agenti/wiff-varianti.md` |
| lettore `.wiff` + `.wiff.scan` (`crates/mzlab-core/src/wiff/`) | `docs/agenti/wiff.md` |
| specifica binaria (area: lettore `.wiff` + `.wiff.scan` (`crates/mzlab-core/src/wiff/`)) | `docs/agenti/wiff_specifica_completa.md` |
| scarto con segno dopo l'm/z o la formula nella finestra «Estrai ioni» (`288.2+16`), `split_offset`, `mzlab-cli ion-mz`, `err.formula.offset`, `err.formula.mzLow`; campi «m/z da / a» e coppie m/z / RT (`xic-bordi.js`, `ctl.range.order`); `tabNuovo`, `msg.noFiles` | `docs/agenti/xic-scarto.md` |
