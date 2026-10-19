# Prova su carta: quattro workflow veri di terzi nel formato `mzflow/2`

Parte di `docs/studi/WORKFLOW_MODULARI.md` (§5). Ogni workflow è stato letto in un file vero (URL nel §1 del principale) e riscritto a mano nel formato della bozza (`contratto.schema.json`). Per ogni caso: come si scrive, cosa non si esprime, quale correzione del contratto ne è nata. Le correzioni sono già nella bozza.

## A. mzmine 4.0.2, batch FBMN (`mzmine_microbial_workshop.mzbatch.xml`, 20 passi)
```json
{"formato":"mzflow/2","programma":{"nome":"mzLab","versione":"x"},"ingressi":[{"name":"S1.mzML","sha256":"…"}],
 "passi":[
  {"id":"imp","step":"import.raw@1","in":{"raw":"file:S1.mzML"}},
  {"id":"md1","step":"spectra.centroid@1","in":{"index":"imp.index"},"params":{"ms_level":1,"algorithm":{"algorithm":"factor_of_lowest","noise_factor":5.0}}},
  {"id":"md2","step":"spectra.centroid@1","in":{"index":"imp.index"},"params":{"ms_level":2}},
  {"id":"eic","step":"features.detect@1","in":{"index":"md1.index"},"params":{"min_points":6,"min_height":500000,"mz_tol":{"da":0.002,"ppm":10},"rt_range":[0.3,30.0]}},
  {"id":"res","step":"features.resolve@1","in":{"peaks":"eic.peaks"},"params":{"top_edge_ratio":2.0,"duration_min":[0,1.2]}},
  {"id":"iso","step":"features.isotopes@1","in":{"peaks":"res.peaks"},"params":{"mz_tol":{"da":0.0015,"ppm":3},"rt_tol_min":0.04,"max_charge":2}},
  {"id":"aln","step":"features.align@1","in":{"peaks":["iso.peaks"]},"params":{"mz_tol":{"da":0.0015,"ppm":5},"rt_tol_min":0.1,"weight_mz":3,"weight_rt":1}},
  {"id":"gap","step":"features.gapfill@1","in":{"table":"aln.table","index":["md1.index"]},"params":{"rt_tol_min":0.1,"min_points":3}}]}
```
- **Si esprime**: passi 1-10 (import → gap filling), con la stessa sequenza e gli stessi numeri.
- **Non si esprime**: lisciatura come passo a sé (nostro `features.smooth` non esiste: buco della matrice); «Original feature list = REMOVE» (gestione della memoria della sessione, non un risultato); passi 12-16 (correlazione, ion identity networking, reti spettrali, lipidi) e gli export GNPS/SIRIUS/GraphML (nessun modulo mzLab o solo in parte).
- **Correzioni nate qui**: (1) lo stesso tipo di passo compare due volte (Mass detection su MS1 e su MS2) → `id` unico per ISTANZA, il tipo sta in `step`; (2) le tolleranze sono coppie Da + ppm → tipo composto `mz_tol {da, ppm}` con unità esplicite, mai il nostro «ppm negativo = Da» di `Recipe::row_window`; (3) la scelta di un algoritmo con i suoi parametri (`selected_item` + `<module>`) → `oneOf` con il discriminante `algorithm`; (4) l'ingresso implicito `BATCH_LAST_FEATURELISTS` → riferimenti espliciti `passo.uscita` (un importatore li ricostruisce in ordine).

## B. OpenMS: FeatureFinderMetabo (`.ini` vero, versione 2.8.0) e catena tipica
```json
{"passi":[
  {"id":"ffm","step":"features.detect@1","in":{"index":["file:a.mzML","file:b.mzML"]},"params":{"noise_floor":10.0,"mz_tol":{"ppm":20},"min_trace_s":5.0,"snr_min":3.0,"fwhm_s":5.0}},
  {"id":"iso","step":"features.isotopes@1","in":{"peaks":"ffm.peaks"},"params":{"charge":[1,3],"model":"metabolites_5pct_rms"}},
  {"id":"rt","step":"series.rt_align@1","in":{"peaks":"iso.peaks"}},
  {"id":"link","step":"features.align@1","in":{"peaks":"iso.peaks","rtmap":"rt.rtmap"}}]}
```
- **Si esprime**: il rilevamento (con nomi nostri e unità dichiarate) e la catena FFM → allineamento → legame.
- **Non si esprime**: FFM in OpenMS fa tracce + profilo + isotopi in UN solo strumento; da noi sono tre passi, e `features.detect` di mzLab non accetta i parametri di FFM uno a uno (nomi e significati diversi: `noise_threshold_int` è sull'intensità grezza, il nostro `floor` può essere stimato). Un import di `.ini` resta quindi una TRADUZIONE dichiarata, non un'equivalenza. `MapAlignerPoseClustering` e `FeatureLinkerUnlabeledKD`: nessun `.ini` letto (ipotesi). Gli ingressi «file:» qui sono mzML: serve un passo implicito `index.mzidx`.
- **Correzioni**: (5) il flag `advanced` di OpenMS → `x-advanced` (il modulo mostra prima i 3-5 parametri che contano: pochi clic); (6) un ingresso «file:» di un tipo diverso da quello della porta (mzML verso `mzidx`) si converte da solo SOLO con i passi di conversione dichiarati nel registro, e la provenienza li scrive come attività vere.

## C. XCMS, vignetta (`xcms.Rmd`, ramo `devel`)
```json
{"passi":[
  {"id":"cw","step":"features.detect@1","in":{"index":["file:ko15.mzML","file:wt15.mzML"]},"params":{"algorithm":{"algorithm":"centwave","peakwidth_s":[20,80],"noise":5000,"prefilter":[6,5000]}}},
  {"id":"mrg","step":"features.merge_neighbours@1","in":{"peaks":"cw.peaks"},"params":{"expand_rt_s":4}},
  {"id":"rt","step":"series.rt_align@1","in":{"peaks":"mrg.peaks"},"params":{"algorithm":{"algorithm":"obiwarp","bin_mz":0.6}}},
  {"id":"grp","step":"features.align@1","in":{"peaks":"mrg.peaks","rtmap":"rt.rtmap","samples":"file:campioni.json"},"params":{"bandwidth_s":30,"min_fraction":0.4}},
  {"id":"fill","step":"features.gapfill@1","in":{"table":"grp.table"}}]}
```
- **Si esprime**: la sequenza intera; `sampleGroups` diventa un ingresso tipizzato `samples` (`campioni/2`), non un parametro.
- **Non si esprime oggi**: centWave e Obiwarp non sono algoritmi di mzLab (scelta `algorithm` con un solo valore oggi, `masstrace`/`anchors`); `refineChromPeaks` non ha un passo nostro. La vignetta ripete l'allineamento con parametri diversi su un sottoinsieme (`pdp_subs`): due istanze dello stesso passo sullo stesso ingresso = ramo del grafo.
- **Correzioni**: (7) XCMS modifica l'oggetto sul posto (`adjustRtime` cambia gli RT) → da noi ogni passo produce un dato NUOVO (`rtmap`), gli ingressi sono immutabili: è ciò che rende la riesecuzione e la cache per hash possibili; (8) i gruppi di campioni sono un tipo (`samples`), con il proprio hash.

## D. patRoon, percorso TP «logic» e «library» (handbook, cap. 7)
```json
{"passi":[
  {"id":"fg","step":"features.align@1","in":{"peaks":"det.peaks"}},
  {"id":"flt","step":"table.filter@1","in":{"table":"fg.table","samples":"file:campioni.json"},"params":{"abs_min_intensity":1e4,"blank_ratio":3}},
  {"id":"ms2","step":"features.ms2_link@1","in":{"table":"flt.table"}},
  {"id":"tpl","step":"tp.generate@1","in":{"table":"flt.table"},"params":{"algorithm":{"algorithm":"logic","rules":"file:trasformazioni.json"}}},
  {"id":"tpb","step":"tp.generate@1","in":{"structure":"file:timolol.mol"},"params":{"algorithm":{"algorithm":"library","library":"file:pubchem_trans.csv"}}},
  {"id":"lnk","step":"tp.link@1","in":{"table":"flt.table","spectra":"ms2.spectra","tp":["tpl.tp","tpb.tp"]},"params":{"rt_direction":true,"min_spec_sim":0.75,"min_frag_matches":1}}]}
```
- **Si esprime**: entrambi i rami e la loro unione (`tp` con `many`), cioè un grafo, non una catena.
- **Non si esprime né si esegue**: `algorithm = "biotransformer"` e CTS (programmi Java esterni e servizio web: fuori dai principi, §8 del principale); `convertToMFDB` + MetFrag; il confronto «before/after» per fold change di `as.data.table(FCParams)` (nessun passo nostro: c'è `tp/1` con la forma della serie, ma non con la stessa logica, ipotesi). `estimateIDConfidence` ≈ i nostri livelli, ma con criteri diversi.
- **Correzioni**: (9) un passo dichiarato nel registro ma non disponibile su una piattaforma (`runs_in` senza `wasm`) → il controllo del piano lo rifiuta PRIMA di partire con una chiave `err.flow.unavailable` e il nome del passo; (10) le liste dell'utente (trasformazioni, librerie) sono INGRESSI con hash, mai parametri di testo; (11) il grafo serve dal primo giorno nel formato (rami di patRoon e di XCMS), mentre l'interfaccia può mostrare solo catene all'inizio.

## Esito
| Workflow (conteggi a mano) | Passi letti | Espressi | Eseguibili oggi in mzLab | Cosa manca |
|---|---|---|---|---|
| A mzmine FBMN | 20 | 9 di 20 | 8 di 20 (manca la lisciatura) | lisciatura; reti molecolari ed export GNPS: fuori perimetro |
| B OpenMS | 4 (1 letto) | 4 | 4, con nomi e significati nostri | equivalenza dei parametri: solo traduzione dichiarata |
| C XCMS | 5 | 5 | 3 (manca centWave, Obiwarp, refine) | algoritmi alternativi, se mai si vorranno |
| D patRoon TP | 10 | 8 | 6 | BioTransformer, CTS, MetFrag: fuori dai principi |
