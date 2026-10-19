# Paper test: four real third-party workflows in the `mzflow/2` format

Part of `docs/studi/WORKFLOW_MODULARI.md` (§5). Each workflow was read in a real file (URLs in §1 of the main file) and rewritten by hand in the draft format (`contract.schema.json`). For each case: how it is written, what cannot be expressed, which contract correction came from it. The corrections are already in the draft; the JSON blocks below validate it (V).

## A. mzmine 4.0.2, FBMN batch (`mzmine_microbial_workshop.mzbatch.xml`, 20 steps)
```json
{"format":"mzflow/2","program":{"name":"mzLab","version":"x"},"inputs":[{"name":"S1.mzML","sha256":"…"}],
 "steps":[
  {"id":"imp","step":"import.raw@1","in":{"raw":"file:S1.mzML"}},
  {"id":"md1","step":"spectra.centroid@1","in":{"index":"imp.index"},"algorithm":"factor_of_lowest","params":{"ms_level":1,"noise_factor":5.0}},
  {"id":"md2","step":"spectra.centroid@1","in":{"index":"imp.index"},"params":{"ms_level":2}},
  {"id":"eic","step":"features.detect@1","in":{"index":"md1.index"},"params":{"min_points":6,"min_height":500000,"mz_tol":{"da":0.002,"ppm":10},"rt_range":[0.3,30.0]}},
  {"id":"res","step":"features.resolve@1","in":{"peaks":"eic.peaks"},"params":{"top_edge_ratio":2.0,"duration_min":[0,1.2]}},
  {"id":"iso","step":"features.isotopes@1","in":{"peaks":"res.peaks"},"params":{"mz_tol":{"da":0.0015,"ppm":3},"rt_tol_min":0.04,"max_charge":2}},
  {"id":"aln","step":"features.align@1","in":{"peaks":["iso.peaks"]},"params":{"mz_tol":{"da":0.0015,"ppm":5},"rt_tol_min":0.1,"weight_mz":3,"weight_rt":1}},
  {"id":"gap","step":"features.gapfill@1","in":{"table":"aln.table","index":["md1.index"]},"params":{"rt_tol_min":0.1,"min_points":3}}]}
```
- **Expressed**: steps 1-10 (import → gap filling), same sequence and same numbers.
- **Not expressed**: smoothing as a step of its own (our `features.smooth` does not exist: a gap of the matrix); «Original feature list = REMOVE» (session memory management, not a result); steps 12-16 (correlation, ion identity networking, spectral networks, lipids) and the GNPS/SIRIUS/GraphML exports (no mzLab module, or only in part).
- **Corrections born here**: (1) the same kind of step appears twice (Mass detection on MS1 and MS2) → a unique `id` per INSTANCE, the kind is in `step`; (2) tolerances are Da + ppm pairs → a compound type `mz_tol {da, ppm}` with explicit units (never the «negative ppm = Da» of `Recipe::row_window`);
  (3) choosing an algorithm with its own parameters (`selected_item` + `<module>`) → `algorithms[]` of the step, each with its own parameters, and `algorithm` of the instance; (4) the implicit input `BATCH_LAST_FEATURELISTS` → explicit `STEP.OUTPUT` references (an importer rebuilds them in order).

## B. OpenMS: FeatureFinderMetabo (real `.ini`, version 2.8.0) and the usual chain
```json
{"steps":[
  {"id":"ffm","step":"features.detect@1","in":{"index":["file:a.mzML","file:b.mzML"]},"params":{"noise_floor":10.0,"mz_tol":{"ppm":20},"min_trace_s":5.0,"snr_min":3.0,"fwhm_s":5.0}},
  {"id":"iso","step":"features.isotopes@1","in":{"peaks":"ffm.peaks"},"params":{"charge":[1,3],"model":"metabolites_5pct_rms"}},
  {"id":"rt","step":"series.rt_align@1","in":{"peaks":"iso.peaks"}},
  {"id":"link","step":"features.align@1","in":{"peaks":"iso.peaks","rtmap":"rt.rtmap"}}]}
```
- **Expressed**: detection (with our names and declared units) and the chain FFM → alignment → linking.
- **Not expressed**: FFM does traces + profile + isotopes in ONE tool; we have three steps, and mzLab `features.detect` does not take the FFM parameters one by one (different names and meanings: `noise_threshold_int` is on raw intensity, our `floor` can be estimated). An `.ini` import is therefore a declared TRANSLATION, not an equivalence. `MapAlignerPoseClustering` and `FeatureLinkerUnlabeledKD`: no `.ini` read (hypothesis). The «file:» inputs here are mzML: an implicit `index.mzidx` step is needed.
- **Corrections**: (5) the OpenMS `advanced` flag → `x-advanced` (the form shows first the 3-5 parameters that matter: few clicks); (6) a «file:» input of a type other than the port's (mzML into `mzidx`) is converted automatically ONLY through the conversion steps declared in the registry, and provenance writes them as real activities.

## C. XCMS vignette (`xcms.Rmd`, `devel` branch)
```json
{"steps":[
  {"id":"cw","step":"features.detect@1","in":{"index":["file:ko15.mzML","file:wt15.mzML"]},"algorithm":"centwave","params":{"peak_width_s":[20,80],"noise":5000,"prefilter":[6,5000]}},
  {"id":"mrg","step":"features.merge_neighbours@1","in":{"peaks":"cw.peaks"},"params":{"expand_rt_s":4}},
  {"id":"rt","step":"series.rt_align@1","in":{"peaks":"mrg.peaks"},"algorithm":"obiwarp","params":{"bin_mz":0.6}},
  {"id":"grp","step":"features.align@1","in":{"peaks":"mrg.peaks","rtmap":"rt.rtmap","samples":"file:samples.json"},"params":{"bandwidth_s":30,"min_fraction":0.4}},
  {"id":"fill","step":"features.gapfill@1","in":{"table":"grp.table"}}]}
```
- **Expressed**: the whole sequence; `sampleGroups` becomes a typed `samples` input (`campioni/2`), not a parameter.
- **Not expressed today**: centWave and Obiwarp are not mzLab algorithms (in the descriptor they are `planned` entries of `algorithms[]`; only `masstrace` and `anchors` are `available`); `refineChromPeaks` has no step of ours. The vignette repeats the alignment with other parameters on a subset (`pdp_subs`): two instances of the same step on the same input = a branch of the graph (or a `meta.compare`).
- **Corrections**: (7) XCMS changes the object in place (`adjustRtime` changes the RTs) → our steps produce NEW data (`rtmap`), inputs are immutable: this is what makes re-running and the hash cache possible; (8) sample groups are a type (`samples`), with their own hash.

## D. patRoon, TP workflow «logic» and «library» (handbook, ch. 7)
```json
{"steps":[
  {"id":"fg","step":"features.align@1","in":{"peaks":"det.peaks"}},
  {"id":"flt","step":"table.filter@1","in":{"table":"fg.table","samples":"file:samples.json"},"params":{"abs_min_intensity":1e4,"blank_ratio":3}},
  {"id":"ms2","step":"features.ms2_link@1","in":{"table":"flt.table"}},
  {"id":"tpl","step":"tp.generate@1","in":{"table":"flt.table"},"algorithm":"logic","params":{"rules":"file:transformations.json"}},
  {"id":"tpb","step":"tp.generate@1","in":{"structure":"file:timolol.mol"},"algorithm":"library","params":{"library":"file:pubchem_trans.csv"}},
  {"id":"lnk","step":"tp.link@1","in":{"table":"flt.table","spectra":"ms2.spectra","tp":["tpl.tp","tpb.tp"]},"params":{"rt_direction":true,"min_spec_sim":0.75,"min_frag_matches":1}}]}
```
- **Expressed**: both branches and their union (`tp` with `many`), i.e. a graph, not a chain.
- **Neither expressed nor run**: `algorithm = "biotransformer"` and CTS (external Java programs and a web service: outside the principles, §8 of the main file); `convertToMFDB` + MetFrag (our `spectra.insilico` is the in-house replacement, PR #1059); the «before/after» fold-change comparison of `as.data.table(FCParams)` (no step of ours: `tp/1` has the series shape, but not the same logic, hypothesis). `estimateIDConfidence` ≈ our `annot.level`, with different criteria.
- **Corrections**: (9) a step declared in the registry but not available on a platform (`runs_in` without `wasm`) → the plan check rejects it BEFORE starting, with the key `err.flow.unavailable` and the step name; (10) user lists (transformations, libraries, regulatory snapshots) are INPUTS with a hash, never text parameters; (11) the graph is needed from day one in the format (patRoon and XCMS branches), while the interface may show only chains at first.

## Outcome
| Workflow (counted by hand) | Steps read | Expressed | Runnable in mzLab today | Missing |
|---|---|---|---|---|
| A mzmine FBMN | 20 | 9 of 20 | 8 of 20 (smoothing missing) | smoothing; molecular networks and GNPS export: out of scope |
| B OpenMS | 4 (1 read) | 4 | 4, with our names and meanings | parameter equivalence: declared translation only |
| C XCMS | 5 | 5 | 3 (centWave, Obiwarp, refine missing) | alternative algorithms, if ever wanted |
| D patRoon TP | 10 | 8 | 6 | BioTransformer, CTS, MetFrag: outside the principles |
