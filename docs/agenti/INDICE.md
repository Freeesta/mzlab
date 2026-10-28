# File d'area: quale leggere
Una riga per area: se tocchi la zona a sinistra, leggi il file a destra (solo quello). Trovare una riga: `grep -n '<parola>' docs/agenti/INDICE.md`. Ogni `docs/agenti/*.md` è citato qui (`tools/cancello_rilascio.py`).
| Se tocchi… | Leggi |
|---|---|
| vista Dati in bassa risoluzione: caricamento, schede, TIC/XIC/spettro, integrazione | `docs/agenti/dati.md` |
| primo caricamento di più file (si apre solo il primo), campi della tabella di carico | `docs/agenti/dati-carico.md` |
| barra laterale della vista Dati, strumenti, suggerimenti, tour | `docs/agenti/dati-barra.md` |
| affiancare e spostare i pannelli (maniglia, zone di rilascio, divisori, annulla) | `docs/agenti/pannelli-affianca.md` |
| esportazioni a pezzi in OPFS (`dl()`, `export-pezzi.js`, `export-worker.js`, provenienza, pulizia di `export/`) | `docs/agenti/dati-esporta.md` |
| azioni dei pannelli (registro `azioni/1`, menu del tasto destro, Maiusc+F10) | `docs/agenti/azioni.md` |
| registro delle azioni che cambiano stato (`comandi.js`, `docs/azioni.md` generato), annulla/ripeti, voci «tolto» (togliere un file, chiudere un pannello, svuotare le marcature: nessun avviso, voci di `STORIA`) | `docs/agenti/storia-tolto.md` |
| animazione di caricamento (`base`) e analizzatori animati spenti (`an-*.js`: quadrupolo, TOF, Orbitrap, ESI, trappola, FT-ICR, QqQ, Paul, settore, IMS, MALDI) | `docs/agenti/analizzatori.md` |
| header (schermo intero, logo, (i)), area e tabella di caricamento, avvio progressivo (`AVVIO`), regola «interfaccia subito» | `docs/agenti/intestazione.md`, `avvio.md`, `avvio-rapido.md` |
| barra di caricamento unica per ogni formato (`progresso.js`, `PROGRESSO`, evento `mzlab-index`, pesi delle fasi, prove di monotonia) | `docs/agenti/progresso.md` |
| carica una cartella, cartella puntata e ricordata (elenco in grigio, «Altri», desktop) | `docs/agenti/dati-cartella.md` |
| mappa RT×m/z, mirino, navigazione | `docs/agenti/dati-mappa.md` (visore 3D: `dati-mappa-3d.md`) |
| velocità della mappa: misure dei gesti (`tools/bench_mappa.py`), diagnosi e pacchetti P1-P10 | `docs/agenti/mappa-fluida.md` |
| liste, correzione, flusso di lavoro, QA/QC, statistica, rapporto | `docs/agenti/dati-strumenti.md` |
| calcolatrice «Cosa può essere?», pannello Addotti, fatti sui dati del 3200, gradiente, tracce analogiche, tabelle dei dati, tema | `docs/agenti/dati-3200.md` |
| mondo HR: Banco, formule, isotopi, librerie, albero MSn | `docs/agenti/alta-risoluzione.md` |
| statistica in HR: tabella delle feature, bianco, differenza dal riferimento, PCA | `docs/agenti/alta-risoluzione-statistica.md` |
| formato `.mzlib`, parser delle librerie, indici e punteggi (`crates/mzlab-lib`) | `docs/agenti/librerie.md` |
| motore di confronto spettrale con più librerie per mzFlow (nodo «Attribution», `table.attribute`) e pannello laterale MSⁿ: `libset`, `libmatch/1`, preset, analoghi con ΔFormula, catalogo `librerie/2` | `docs/agenti/match-spettrale.md` (prompt dei lavori: `docs/agenti/prompt-match/`) |
| Disegno (Ketcher) | `docs/agenti/disegno.md` |
| Teoria e Pratica (`mzlab/web/teoria/`) | `docs/agenti/teoria.md` (assistente: `assistente-teoria.md`; `teoria/LEGGIMI.md` se scrivi capitoli) |
| accessibilità dell'intera applicazione (WCAG 2.1/2.2 AA, axe-core, controlli) | `docs/agenti/accessibilita.md` (lacune note: `a11y-da-sistemare.md`) |
| testi, `lang/`, `help.js`, inglese | `docs/agenti/lingue.md` |
| metodo di acquisizione (`metodo/1`, `metodo.rs`, `metodo_wiff`, `metodo_raw`, schema) | `docs/agenti/metodo.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` (tabelle untargeted e identità: `mzfinder-untargeted.md`; tabelle terze parti: `mzfinder-import.md`; validazione: `mzfinder-validazione.md`; tabella prodotti HR: `mzfinder-tabella.md`; indice sospetti: `sospetti.md`; scavo LR: `mzfinder-lr.md`, bersagli: `mzfinder-lr-bersagli.md`, passo «1 · Files»: `mzfinder-file.md`, interfaccia MS2: `mzfinder-ms2.md`, regola MS2: `mzfinder-ms2-regola.md`) |
| mzFinder: Dig in diretta (`js/24_tpmine_diretta.js`, `TPLIVE`), coppie progenitore-TP e interfaccia HR (`js/21_tpmine_hr.js`, `mass_network`, previsione) | `docs/agenti/mzfinder-diretta.md`, `mzfinder-coppie.md` (nucleo: `mzfinder.md`) |
| mzFinder nel browser (`crates/mzlab-feat-wasm`, `TP_Mine/files/feat-worker.js`, pannello Pipeline `TP_Mine/js/32_pipeline.js`, stima dei parametri `crates/mzlab-feat/src/params.rs`) | `docs/agenti/mzfeat-wasm.md` (famiglia di un file a richiesta, `family_record`: `mzfeat-wasm-famiglia.md`; pannello «Pipeline»: `mzfeat-wasm-pannello.md`) |
| scandump, oracolo RFR | `docs/agenti/scandump.md` |
| velocità del lettore Thermo `.raw`: criteri A1-A7 e B1-B5, banco `thermo_banco`, `tools/bench/thermo_velocita.py`, numeri di partenza contro RFR (T-7) | `docs/agenti/thermo-velocita.md` |
| conversione `.raw` → mzML: banco `raw_mzml_banco`, `tools/bench/raw_mzml.py`, T_min, punti di partenza V1-V9, cause di D8 e D11, caso denso sintetico | `docs/agenti/raw-mzml.md` |
| tabella cumulativa dei guadagni di velocità (data · PR · passo · prima · dopo · %) | `docs/agenti/misure-velocita.md` (tentativi scartati: `ottimizzazioni-tentate.md`) |
| lettore `.wiff` + `.wiff.scan` (`crates/mzlab-core/src/wiff/`) | `docs/agenti/wiff.md` (altri strumenti SCIEX, sMRM, PIS/NL, DAD, feature `estesi`: `wiff-varianti.md`; specifica binaria: `wiff_specifica_completa.md`) |
| lettore cartelle Agilent `.D` (`crates/mzlab-core/src/agilent/`) | `docs/agenti/agilent.md` |
| Agilent `.D` LC/MS: QqQ MRM/dMRM, Q-TOF profilo/centroidi/MS2, metodo LC/MS, DAD (`crates/mzlab-core/src/agilent_ext/`, feature `estesi`) | `docs/agenti/agilent-lcms.md` |
| lettore Bruker timsTOF `.d` (`crates/mzlab-lettori/src/bruker/`: SQLite minimo, zstd, `tdf_bin`/`tsf_bin`, calibrazione m/z, `BrukerD` in `agilent-d.js`) | `docs/agenti/bruker.md` |
| lettore Shimadzu `.qgd` e `.lcd` (`crates/mzlab-lettori/src/shimadzu/`, modulo `shimadzu`) | `docs/agenti/shimadzu.md` |
| lettore cartelle Waters MassLynx `.raw` (`crates/mzlab-lettori/src/waters/`, modulo `waters`, raggruppamento in `waters-raw.js`) | `docs/agenti/waters.md` |
| benchmark di mzFinder contro altri strumenti (`tools/bench/`, tabella `bench/1`, metriche P1-P5, statistica) | `docs/agenti/mzfinder-benchmark.md` |
| picchi spalmati: lotto sintetico, configurazioni MZmine default/wide, tabella di confronto (`tools/bench/confronto_larghi.py`) | `docs/agenti/bench-picchi-larghi.md` |
| pipeline delle feature a bassa risoluzione (full scan, asse in Da, profilo `lr`: `recipe_lr.rs`, `axis.rs`) | `docs/agenti/mzfeat-lr.md` |
| opzioni del passaggio mirato (`target.rs`: picchi stimati, controllo di forma /\, fusione di righe sovrapposte) | `docs/agenti/mzfeat-mirato.md` |
| ingresso di mzFinder: `.mzidx` per ogni file scelto o da IndexedDB (`TP_Mine/files/finder-idx.js`), lista di trasformazioni dell'utente in Rust | `docs/agenti/mzfinder-ingresso.md` |
| Feature table nella Pipeline (passi 5-6: QC, barra dei filtri, `PIPET` con `TPTAB` e riga espansa `TPRIGA`) | `docs/agenti/mzfeat-tabella-vista.md` |
| pseudo-spettri MS2 da DIA (`dia.rs`, `spectra.ms2` con `dia_pseudo`) | `docs/agenti/dia.md` |
| pipeline delle feature HR in Rust (`crates/mzlab-feat`) | `docs/agenti/mzfeat.md` (prove e misure: `mzfeat-prove.md`; tabella, id e campioni: `mzfeat-tabella.md`; vista dei prodotti `tp/1`: `mzfeat-tp.md`; bande larghe: `mzfeat-broad.md`; cinetica: `mzfeat-cinetica.md`; correzione dell'RT: `mzfeat-rt.md`) |
| Attributio in Rust (cascata dei livelli, Banco 2b, binario MZ di rilevamento alla mzmine: `crates/mzlab-feat/src/attributio/`, `untargeted/`, `.wasm` `attributio*` e `untargeted`) | `docs/agenti/attributio.md` (binario MZ: `attributio-mz.md`, parità con mzmine: `attributio-mz-parita.md`; isotopi: `attributio-isotopi.md`; Banco 2b: `attributio-banco.md`; esportazioni: `attributio-export.md`; abbinamento: `attributio-abbinamento.md`; identità: `attributio-identita.md`; revisione: `attributio-revisione.md`; pannello: `attributio-pannello.md`; formule: `attributio-formula.md`) |
| vista «Compare conditions» di Tabui (`TP_Mine/lazy/gruppi-vista.js`: matrice TP × condizione, UpSet, grafici, esportazioni; contratto `gruppi/1`) | `docs/agenti/gruppi-vista.md` (studio: `docs/studi/TABUI_GRUPPI_ESPERIMENTI.md`) |
| grafo di frammentazione MSn dell'infusione (`msngrafo/1`: ioni uniti fra livelli ed energie, formule proposte dei frammenti, scheda «Fragmentation graph») e libreria di alberi MSn degli standard (`msnlib/1`, protocollo di acquisizione) | `docs/agenti/msn-grafo.md` (libreria: `msnlib.md`; vista: `msn-grafo-vista.md`; scheda «Fragmentation graph» nella vista Dati: `infusione-grafo.md`; tagli per un nodo: `msn-grafo-strutture.md`; interpretazione dei frammenti: `msn-interpreta.md`; prove delle regole: `msn-regole-prove.md`; manuale: `msn-manuale.md`; figura: `msn-figura.md`) |
| formule dei frammenti del grafo MSn (`msngrafo/formule.rs`, `candidati.rs`, `annota.rs`: ingresso, radice, finestre, vincolo DAG, perdite) | `docs/agenti/msn-grafo-formule.md` (sintesi: `msn-grafo.md`) |
| motore MS2 sperimentale, frammentazione in silico alla MetFrag, attribuzione dei frammenti alla MS-FINDER, previsione dell'RT (`attributio/ms2_engine/`) | `docs/agenti/attributio-ms2.md` |
| interpretazione per riga dei TP (`crates/mzlab-feat/src/interp/`, `interp/1`, `filtri/1`, blocco `interp` nel `.mzfeat`, `.wasm` `interp`, rotte `table_interp`) | `docs/agenti/interp.md` |
| rete dei prodotti (`crates/mzlab-feat/src/rete/`, `rete/1`: archi per regola su `tp/1`, passi, coordinate, motivo dell'arco) | `docs/agenti/rete.md` |
| posizione della sostituzione negli isomeri dei TP (classi compatibili con la MS2, `chem/posizioni.rs`, `posiz/1`) | `docs/agenti/posizioni.md` |
| motore di reattività del genitore (crates/mzlab-chem, .wasm chem, reatt/1, viewer 3D di Tabui) | `docs/agenti/reattivita.md` (validazione sui siti sperimentali ed esperimenti scartati: `reattivita-studi.md`) |
| classi chimiche da regole (`crates/mzlab-feat/src/classi/`, tabella `data/classi-1.json`, `classi/1`: frammenti diagnostici e perdite neutre) | `docs/agenti/classi.md` |
| livelli di identificazione (scala di Schymanski 2a/2b/3a/3b, scala di previsione, etichette «L3» e «Prev. 2», Tabui, Attributio, untargeted) | `docs/agenti/livelli.md` |
| gruppi di esperimenti in Tabui: condizioni, `gruppi/1`, `crates/mzlab-feat/src/gruppi/` | `docs/agenti/gruppi.md` |
| statistica come passi: Welch/ANOVA con BH, PCA e KMD in Rust (`crates/mzlab-feat/src/stat/`, `table.stats`, `table.pca`, `table.kmd`, `stat/1`) | `docs/agenti/stat.md` |
| estremi di array grandi senza spread, ricerca binaria dei punti in una finestra (`numeri.js`: `maxOf`, `minOf`, `maxAbs`, `spanOf`) | `docs/agenti/numeri.md` |
| liste dell'utente (trasformazioni, addotti, sospetti: CSV con errori per riga, `liste/1`, SHA-256 per `prov/1`) | `docs/agenti/liste.md` |
| database delle molecole (`crates/mzlab-core/src/molecules/`, `molecules/1`: record, CSV e JSON canonico, ricerca, livelli catalogo/importata/utente, catalogo `mzlab/web/molecole/catalogo.csv`, rotte `api/molecules/*`) | `docs/agenti/molecules.md` (studio e piano a PR: `docs/studi/molecules/README.md`) |
| canali dei dati (file → canali: TIC, filtri/transizioni/precursori, UV, analogici; tipo di esperimento come etichetta; piano DEV-a…f) | `docs/agenti/canali-dati.md` (prompt dei DEV: `prompt-canali/a.md`…`f.md`) |
| ruoli dei file, tempi e metadati dei campioni (`campioni/2` e `/3`, `mzlab-core/src/campioni/`: proposte dai nomi, modelli di nome, CSV, colonne libere, dati dal file, avvisi, migrazione, indici per il percorso TP) | `docs/agenti/campioni.md` |
| derivati (`.mzidx`, `.mzfeat`, `.mzfeatf`) di formato più nuovo del programma: `error.newerFormat`, sola lettura | `docs/agenti/formato-nuovo.md` |
| tetti dei .wasm di mzFinder, moduli fusi (gruppo HR e Attributio), build separata `MZLAB_WASM_SEPARATI` | `docs/agenti/wasm-tetti.md` |
| parità sito ↔ app (funzioni e passi del Flow presenti da una parte sola, operazioni del motore assenti nell'app, test di parità mancanti, prossimi passi) | `docs/studi/parita-browser.md` (formati: `docs/studi/formati-matrice.md`) |
| potenziale di WebAssembly (profili `opt-level`, SIMD128, binaryen, zlib-rs, thread, memory64, browser minimi; misure e prompt DEV) | `docs/agenti/wasm-potenziale.md` (prompt: `docs/studi/PROMPT_WASM_POTENZIALE/`) |
| esportazione di tutti i dati dell'utente: inventario dello storage nel browser, scrittore ZIP (`zip.rs`) | `docs/agenti/esporta-dati.md` |
| contratto di memoria dei moduli WASM (`mzlab-core::budget`, uscite a offset, lastre, flusso delle prove) | `docs/agenti/memoria.md` |
| formato `.mzidx` | `docs/agenti/mzidx.md` |
| cosa i file grezzi contengono e non estraiamo ancora (regola «nessuna perdita») | `docs/agenti/grezzi-non-estratti.md` |
| errori di lettura dei file: causa breve, «Dettagli», «Copia diagnosi» (`diagnosi.js`), mzML sintetico stile msconvert | `docs/agenti/diagnosi.md` |
| assistente LLM: catalogo `strumenti/1` | `docs/agenti/llm.md` (subagenti e letture: `llm-subagenti.md`) |
| assistente LLM: esecutore dei lettori sulla pagina (`llm-agente.js`, router, `sub`) | `docs/agenti/llm-lettori.md` |
| riga di comando `mzlab-cli features` (`crates/mzlab-cli`: feature table in CSV, stesso motore e ricetta della pagina) | `docs/agenti/cli.md` (passo per passo: `cli-passi.md`) |
| agenti esterni (Claude Code, Cowork) che usano `mzlab-cli` dalla shell: sequenza, codici, esempi di piani | `docs/agenti/per-agenti.md` |
| lettura nativa di `.raw`, `.wiff`, `.D` nel sito (campioni, OPFS, avanzamento, mzML su richiesta) | `docs/agenti/lettura-nativa.md` |
| motore Rust (`crates/`, dati d'oro `tools/golden.py`) | `docs/agenti/rust.md` (+ `rust-rotte.md`, `rust-desktop.md` (mzFinder nativo: `rust-desktop-finder.md`), `rust-mzidx.md`, `rust-step.md`: descrittori `step/1`, `rust-flow.md`: piani `mzflow/2`, controllo e `prov/1`) |
| record della struttura chimica `mol/1` (`crates/mzlab-mol`: tipi, JSON, hash del nucleo; lettori SMILES/molfile) | `docs/agenti/mol.md` (studio: `docs/studi/STRUTTURA_DATO.md`) |
| misure dei motori | `docs/agenti/misure.md` (misure Rust e dei moduli: `misure-rust.md`; clic del flusso Tabui: `misure-clic.md`; conversione: `misure-convertitore.md`; formati di ingresso del Flow: `misure-formati.md`; esportazioni: `misure-esporta.md`) |
| struttura dei test, test nuovi | `docs/agenti/test.md` |
| workflow GitHub condizionali (concurrency, pages, job saltati) | `docs/agenti/ci-veloce.md` |
| contratto dei lettori dei produttori (`lettori.rs`, trait `Lettore`, registro, `Limiti`), aggiungere un formato | `docs/agenti/estendere.md` |
| modulo «aperti» (mzML con numpress, mzXML: `crates/mzlab-lettori/src/aperti/`, `numpress.rs`) | `docs/agenti/lettori-aperti.md` |
| modulo «andi» (netCDF/ANDI `.cdf`: `crates/mzlab-lettori/src/andi/`) | `docs/agenti/lettori-andi.md` |
| moduli lettori, uno per famiglia (`crates/mzlab-lettori*`, `.wasm` a parte in `wasm-lettori/<famiglia>/`, feature `estesi`, `LETTORI_FAM`, regola R in `shell.js`, `LETTORI_MOD`, `?lettori=modulo`, tetti) | `docs/agenti/lettori-modulo.md` |
| rilascio: tag `vX.Y.Z`, `release.yml`, `CHANGELOG.md`, versione unica, SBOM, cancello (`cancello_rilascio.py`) | `docs/agenti/rilascio.md` |
| Flow grafico (scheda amministratore: palette dei passi `step/1`, catena, moduli dei parametri, confronto `meta.compare`, esportazione/importazione `mzflow/2`; `flow/`) | `docs/agenti/flow-grafica.md` (corrispondenza con i passi di mzmine: `flow-mzmine.md`; copertura di Tabui: `flow-copertura-tabui.md`, `flow-pacchetto-tabui.md`; griglia dei metadati dei campioni: `flow-meta-griglia.md`) |
| esecutore web del Flow («This browser»: `flow/lazy/flow-web-{run,ui,worker}.js`, eventi `flow:web-step`, stima `stima_series`, avviso su telefono/tablet, Run spento oltre il livello alto) | `docs/agenti/flow-web.md` |
| usabilità del Flow (modello MSn infusione: grafo dopo la corsa, suggerimenti dei campi, Raw files, nomi, barra, terminale; prompt dei lavori) | `docs/agenti/flow-ux.md` |
| Libro: convertitore (`tools/libro/`, dialetto Markdown, fixture CC0, `--pubblico`) e pagina editor admin (`mzlab/web/libro/`, token, salvataggio nel repository privato) | `docs/agenti/libro.md` |
| tutor sulla Teoria (`mzlab-tutor`: blocchi, BM25) | `docs/agenti/tutor.md` |
| lanciare la verifica nel container cloud (Chromium, `--setup`, test col sito costruito) | `docs/agenti/ambiente-cloud.md` |
| scheduler a stati del Flow «live» (`mzlab_core::flow::sched`, `run_sched.rs`) | `docs/agenti/flow-live.md` |
| dove Tabui perde un prodotto atteso (`mzlab-cli explain-missing`) | `docs/agenti/tabui-spiega.md` |
| pulsante «XIC» nella barra dello spettro, lente XIC al passaggio sul picco (`xic-lente.js`), banco e tabella di partenza dei tempi del visualizzatore (`tools/bench_visore.py`, `visore_tempi.rs`) | `docs/agenti/visore-veloce.md` |
| pulsante «XIC» e lente XIC spenti sui file d'infusione, regola dei dati `infusione.rs` (chiavi, blocchi, nome), comando `diagnosi_infusione` | `docs/agenti/infusione-lente.md` |
| misura su computer lenti (CPU rallentata 1×/4×/6×, 2/4 core: apertura demo, trascinamento della linea, mappe, XIC, memoria; `tests_e2e/misura_macchine.py`) | `docs/agenti/misure-macchine.md` |
| capacità del browser, feature detection, avviso «non disponibile in questo browser: usa l'app» (`capacita.js`, `?cap-off=`) | `docs/agenti/capacita.md` |
