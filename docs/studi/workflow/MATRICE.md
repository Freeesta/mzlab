# Step matrix: who has each step, and with which parameters

Part of `docs/studi/WORKFLOW_MODULARI.md` (§3). Legend: **V** = read in a real file (mzmine 4.0.2 batch XML, OpenMS FeatureFinderMetabo `.ini`, XCMS vignette, patRoon handbook; URLs in §1 of the main file) or in our code; **I** = hypothesis (memory or documentation not read). For mzLab the path is the code as read on 19/10/2026; «PR #N» = code only in that open pull request.

## 1. Preprocessing and detection (one file at a time)
| Step (proposed `id`) | mzmine | OpenMS | XCMS | patRoon | mzLab | Key parameters (names as in the file) |
|---|---|---|---|---|---|---|
| `import.raw` native reading | `AllSpectralDataImportModule` (V; Thermo and Bruker native, others via msconvert: I) | no, needs mzML (V: `in` = mzML) | no, mzML/mzXML via `MsExperiment` (I) | `convertMSFiles` with msconvert/TRFP (I) | `mzlab_lettori::apri` + trait `Lettore` (V), eight families | mzLab: `Limiti` (memory), sample `c` |
| `convert.mzml` | no (I) | `FileConverter` (I) | no | `convertMSFiles` (I) | `lettori_mzml::scrivi` (V) and a second writer `raw::to_mzml` (V, §6 of the main file) | zlib, numpress (`Opzioni`, V) |
| `spectra.centroid` | «Mass detection» (V: «Factor of lowest signal», «Noise factor» 5.0) | `PeakPickerHiRes` (I) | `pickPeaks` of MSnbase (I) | via msconvert (I) | `lettori_centroidi`, `centroidi/` with Savitzky-Golay (V) | noise, smoothing width |
| `index.mzidx` | internal (I) | internal | internal | — | `mzidx::build_mzml_spill`, `upload::indicizza_lettore` (V) | `DEFAULT_BUDGET` (memory, does not change the result) |
| `features.detect` (mass traces) | ADAP chromatogram builder (V: «Minimum consecutive scans» 6, «Minimum intensity for consecutive scans» 1e5, «Minimum absolute height» 5e5, m/z 0.002 Da / 10 ppm) | `mtd` of FeatureFinderMetabo (V: `mass_error_ppm` 20, `min_trace_length` 5, `noise_threshold_int` 10) | `CentWaveParam` (V: `peakwidth` 20-80, `noise` 5000, `prefilter` 6/5000) | `findFeatures(algorithm=...)` (V) | `trace::detect` (V: `polarity`, `floor`, `min_points`, `min_height`, `max_gap`, `long_trace`, `axis`) | threshold, minimum points, m/z tolerance |
| `features.smooth` | `SmoothingModule` (V: Savitzky-Golay 7) | `use_smoothed_intensities` (V) | inside centWave (I) | of the chosen algorithm | inside `quality`/`resolve` (I: not read) | window |
| `features.resolve` (peaks of one trace) | Local minimum resolver (V: «Chromatographic threshold» 0.9, «Min ratio of peak top/edge» 2, «Peak duration range» 0-1.2) | `epd` (V: `enabled`, `width_filtering` fixed) | centWave wavelets (I) | of the algorithm | `resolve::assess_and_resolve` (V; `split`) | top/edge ratio, duration |
| `features.isotopes` | Isotope grouper (V: 0.0015 Da / 3 ppm, RT 0.04 min, charge ≤ 2) | `ffm` (V: `charge_lower_bound` 1, `charge_upper_bound` 3, `isotope_filtering_model`) | no (CAMERA apart: I) | `generateComponents` (I) | `isotopes::group` (V) | tolerances, charge |
| `features.ms2_link` | «MS/MS scan pairing» of the resolver (V: 0.01 Da / 10 ppm) | not in the step (I) | `featureSpectra` (I) | `generateMSPeakLists` (V) | `ms2::link` with purity (V) | precursor tolerance |
| `features.estimate` parameter estimation | no (I) | no (I) | no | no | `params::estimate`, `preview` (V; absent `floor` = estimated) | — |

## 2. Series (several files)
| Step | mzmine | OpenMS | XCMS | patRoon | mzLab | Key parameters |
|---|---|---|---|---|---|---|
| `series.rt_align` | not in the batch read (RANSAC: I) | `MapAlignerPoseClustering` (I: no `.ini` read) | `adjustRtime(ObiwarpParam(binSize=0.6))` (V) | `groupFeatures(rtalign=FALSE)` (V) | `rtmap` with anchors (V) | step, anchors |
| `features.align` | Join aligner (V: 0.0015 Da / 5 ppm, «Weight for m/z» 3, RT 0.1 min, «Weight for RT» 1) | `FeatureLinkerUnlabeledKD` (I) | `groupChromPeaks(PeakDensityParam(bw=30, minFraction=0.4, sampleGroups))` (V) | `groupFeatures(maxGroupRT=6)` (V) | `align`/`famalign` (V; `split_gap` 0.03 min, Ion or Family mode) | tolerances, minimum fraction, sample groups |
| `features.gapfill` | Peak finder multithread (V: «Intensity tolerance» 0.2, 0.002 Da / 10 ppm, RT 0.1 min, points ≥ 3) | no standard (I) | `fillChromPeaks(ChromPeakAreaParam())` (V) | via XCMS/OpenMS (I) | `gapfill` (V; `fill_gaps`) | tolerances |
| `features.ions` (adducts, families) | Ion identity networking, metaCorrelate (V: names; parameters not read) | `MetaboliteAdductDecharger` (I) | CAMERA (I) | `generateComponents` (I) | `family`, `famalign` (V) | adduct list |
| `table.filter` | Rows filter, Duplicate filter (V: names) | `IDFilter`/`FileFilter` (I) | no | `filter(absMinIntensity, relMinReplicateAbundance, maxReplicateIntRSD, blankThreshold, retentionRange)` (V) | `filters.rs` (view, `filtri/1` registry), roles of `campioni/2` (V) | thresholds, blank role |
| `table.qc` | not in the batch | QC with mzQC (I) | no | no | `qc.rs`, numbers only (V) | — |
| `table.stats` (PCA, difference) | statistics dashboard (I) | no | no | no | `statistica.rs`, `statmat.rs` (V) | — |

## 3. Identification (the chain up to a level)
Data types: `spectra` → `formulas` / `candidates` → `annot` (contract §4). All `requires: {"resolution": ["hr"]}`: in the LR world these steps do not run (principle 1).
| Step | mzmine | OpenMS | XCMS | patRoon | mzLab | External truth for its test |
|---|---|---|---|---|---|---|
| `spectra.chimeric` (co-isolation: purity, intruders, deconvolution) | «MS/MS scan pairing» with a purity filter (I) | `PrecursorPurity` (I) | no | no | `purity_flag`: `fill_purity` in `interp/ms2link.rs` (V); `intruders`: `ms2_engine/chimeric.rs` (PR #1061); library deconvolution as DecoID (Stancliffe 2021, doi:10.1038/s41592-021-01195-3): absent | MassBank spectra of two known compounds mixed in silico at known ratios and windows (truth = the mix); purity against MS1 of the public DDA files of `tools/bench/` |
| `formula.ms1` (exact mass + isotope pattern) | formula prediction (I) | SIRIUS adapter (I) | no | `generateFormulas("genform")` (V) | `untargeted_formula` (V), `isotope_score` in `evid/producers.rs` (V) | MassBank records with known formula, CASMI 2016/2017 challenges (I: licence to read) |
| `formula.ms2` (fragment subformulas, neutral losses, H shifts) | no (I) | SIRIUS (I) | no | GenForm MS/MS score (I) | `subformulas` in `mzlab-core` (V), `ms2_engine/rearrange.rs` with H shifts ±1/±2 (PR #1060), neutral losses in `tp/cancelli.rs` (V) | same sets; the formula must rank first more often with MS2 than with MS1 alone (measured gain, declared before) |
| `spectra.match` (library) | `SpectralLibrarySearchModule` (V: name) | `MetaboliteSpectralMatcher` (I) | no | `generateCompounds` with a library (V) | `search::identity`, `flash_scores`, `analogs` in `mzlab-lib/src/search.rs` (V); scores `cosine`, `modified_cosine`, `spectral_entropy` (V); `ms2_engine/matcher.rs` (V); decoys and q-values `decoy.rs` (V) | the same compound in two public libraries (MassBank against GNPS/MoNA): rank-1 rate and FDR from the decoys, per score |
| `spectra.insilico` (candidate structures, MetFrag-like) | no | no | no | MetFrag (V) | `ms2_engine/insilico.rs` (PR #1059) | CASMI challenges with known answer; MassBank spectra scored against their own structure among decoy candidates |
| `rt.predict` (RT plausibility) | no | no | no | via MetFrag (I) | `rt_logp` (V), `ms2_engine/rtpredict.rs` random forest (PR #1061) | RepoRT public RT sets (I: licence to read) with a held-out column |
| `annot.level` (confidence levels, Schymanski 2014) | no | no | no | `estimateIDConfidence` (V) | cascade of Attributio (`annotate/decide.rs`, `annotate/reasons.rs`, `identity.rs`), scale `data/livelli-1.json` (V) | hand-labelled cases of the Schymanski paper (each level with its evidence) as fixed fixtures; levels of a published annotated dataset (I) |
| `tp.generate` | no | no | no | `generateTPs("library", "logic", "biotransformer", "library_formula")` (V) | `hypo/` (`ipotesi/1`, formula Δ; V) | transformations (`add`, `sub`, `retDir`) |
| `tp.link` | no | no | no | `generateComponents("tp")` + `filter(retDirMatch, minSpecSimBoth, minTotFragMatches)` (V) | `interp` (`interp/1`), `tp/1` (V) | similarity thresholds |
| `tp.position` | no | no | no | no | `chem/posizioni.rs` (`posiz/1`), `legacy/msn/localize.rs`, `interp/sito.rs` (V) | — |
| `export.*` | GNPS FBMN, SIRIUS, GraphML, CSV (V) | mzTab, consensusXML (I) | no | `report` (V) | CSV «full» in mzmine style, MGF/MSP, XLSX, Schymanski export of Attributio (V) | format |

## 4. Chemical structure and transformations (types `structure` and `transform`; the structure format belongs to the dedicated study `docs/studi/STRUTTURA_DATO*.md`, record `mol/1`)
| Step | mzmine | OpenMS | XCMS | patRoon | mzLab | Key parameters |
|---|---|---|---|---|---|---|
| `structure.draw` | no (I) | no | no | no | Drawing with Ketcher in a `sandbox` iframe (V) | — |
| `structure.import` (SMILES, molfile, SDF, InChI) | library structures via CDK (I) | no (I) | no | via rcdk/Open Babel (I) | `grafo::Mol::parse` and `from_molfile` V2000 (V; stereo and isotopes lost) | format |
| `structure.from_result` | no (I) | no | no | MetFrag/SIRIUS candidates (I) | «Use this structure» of Tabui (V) | — |
| `structure.render` (with highlighted regions) | candidate structure (I) | no | no | `plotStructure` (I) | `TPMOL.svg` + `parentesi.js`, reactivity halos, `interp/disegno.rs` (V) | drawing mode, uncertainty |
| `structure.identity` (canonical, InChIKey) | from RDKit/CDK (I) | no | no | via rcdk (I) | no InChIKey computed, only from sources (V); classes of `simmetria.rs` (V) | — |
| `tp.transform` (parent → product with atom mapping) | no | no | no | parent/TP pairs of the library, no mapping (I) | formula Δ of `hypo/` (`ipotesi/1`), sites of `posizioni` (`posiz/1`), atoms + Δ of `parz/1` (V); no atom-by-atom mapping | rule, Δ |
| readers of a structure | — | — | — | MetFrag, TP «library» (V) | `interp`, `positions_json`, `reactivity_json` (Fukui), RT-logP via InChIKey, libraries (V) | — |

## 5. Attributio and Tabui as Flow families
Both become module families of the Flow palette (`package` of `step/1`: `attributio`, `tabui`), not separate apps. Their code is already Rust behind `.wasm` modules (`attributio`, `attributio-rev`, `attributio-export`, `untargeted`, `interp`, `chem`: V); what is missing is the descriptor, the typed ports and, where noted, an adapter (a thin function that reads and writes the contract types around an existing entry point).

| Package (code) | Step (`package`) | In → out | State | Adapter needed |
|---|---|---|---|---|
| `ms2_engine/compile.rs` (`compile_library`) | `library.compile` (attributio) | MSP/GNPS JSON → `library` | V | `library` descriptor over the compiled entries |
| `ms2_engine/matcher.rs` (`Ms2Library::search`), `mzlab-lib/search.rs` | `spectra.match` (attributio) | `spectra` + `library` → `candidates` | V | one `candidates` record for both engines (they score with the same `scores.rs`) |
| `ms2_engine/insilico.rs` | `spectra.insilico` (attributio) | `spectra` + suspect list → `candidates` | PR #1059 | candidates with `source: insilico` |
| `ms2_engine/rearrange.rs` (`attribute`) | `formula.ms2` (attributio) | `spectra` + `formulas` → `formulas` (re-ranked, explained peaks) | PR #1060 | none beyond the types |
| `ms2_engine/rtpredict.rs`, `rt_logp` | `rt.predict` (attributio) | `candidates` + `table` → `candidates` (RT evidence) | PR #1061, V | RT evidence as a field of each candidate |
| `ms2_engine/chimeric.rs` (`intruders`), `interp/ms2link.rs` (`fill_purity`) | `spectra.chimeric` (attributio) | `spectra` + `mzidx` → `spectra` | PR #1061, V | purity and intruders as fields of `spectra` |
| `annotate/`, `identity.rs`, `untid` | `annot.level` (attributio) | `candidates` + `formulas` → `annot` | V | levels from `livelli-1.json` in the `annot` record |
| `tox/` (`snapshot.rs`, `ghs.rs`), `reglists/`, `tox/amenability.rs` | `annot.regulatory`, `annot.hazard` (attributio) | `annot` → `annot` (list membership, GHS/CLP class, ECOTOX values) | `tox/` V; `reglists/` and amenability PR #1070; hazard records T1-T5 PR #1064 | snapshots as typed inputs with hash (correction 10 of `PROVE_SU_CARTA.md`) |
| `export/` (publication, Schymanski) | `export.attributio` (attributio) | `annot` → `report` | V | none |
| `hypo/` (`ipotesi/1`) | `tp.generate` (tabui) | `table` + rules → `transform` (Δ, rule) | V | `transform` records from the hypotheses |
| `interp/` (`build`, `run_to`, `against_dda`), `mzlab-core/ms2sim.rs` | `tp.link` (tabui) | `table` + `spectra` + `transform` → `tp` | V | `interp/1` read as `tp` |
| `chem/posizioni.rs`, `legacy/msn/localize.rs`, `interp/sito.rs` | `tp.position` (tabui) | `tp` + `structure` + `spectra` → `transform` (sites with support) | V (merged with TT-R2) | sites as `transform.sites` |
| `mzlab-chem/reatt.rs` (`reactivity_json`) | `structure.reactivity` (tabui) | `structure` → `annot` (reactive sites) | V | none |
| `interp/disegno.rs`, Drawing (Ketcher) | `structure.render`, `structure.draw` (tabui) | `structure` / `transform` → drawing | V | `mol/1` from the structure study |

**How the two feed each other** (both through the `structure` and `transform` ports, never through files of one app read by the other):
- Tabui → Attributio: a TP with a proposed structure (`transform.product`, or only `transform.delta` + `sites` for level 3b) becomes a suspect of `spectra.insilico` and `spectra.match` (analogue search with `modified_cosine` at the Δ of the transformation); the evidence goes back into `annot.level`.
- Attributio → Tabui: an identified parent (`annot` at level 1-2) gives `tp.generate` its structure; `rt.predict` and the formula from MS2 constrain which TPs are plausible; regulatory and hazard records attach to the TP row.
- One identity record: both sides write `candidates` and `annot` with the same fields, so «Compare conditions» of Tabui and the review of Attributio read the same rows.

## 6. Catalogue and gaps
- **mzLab has the whole raw → table → identification → TP chain** (V in the code, part only in open PRs), but as internal calls: no step describes itself (no `step/1`), parameters are Rust structs (`Recipe`, `Params`) and the interface is written by hand.
- **Real gaps** (no mzLab module): a structure graph with stereo, isotopes and salts (lost on reading today, V), InChIKey computation, continuous-warping RT alignment (Obiwarp: we only have anchors), library-assisted deconvolution of chimeric spectra, mzTab-M and mzQC export, TP generation from structure (BioTransformer, CTS: excluded, §8 of the main file), smoothing as a step of its own.
- **Ours only** (none of the other four): native reading in the browser, parameter estimation with proof, position of the substitution, LR/HR levels with evidence and no verdicts, regulatory and hazard records next to the identification.
