# Matrice dei passi: chi li ha e con quali parametri

Parte di `docs/studi/WORKFLOW_MODULARI.md` (§3). Legenda: **V** = letto in un file vero (batch XML di mzmine 4.0.2, `.ini` di FeatureFinderMetabo di OpenMS, vignetta di XCMS, handbook di patRoon; URL nel §1 del documento principale) o nel nostro codice; **I** = ipotesi (memoria o documentazione non letta). Per mzLab il percorso è quello del codice, letto il 19/10/2026.

## 1. Pretrattamento e rilevamento (un file alla volta)
| Passo (`id` proposto) | mzmine | OpenMS | XCMS | patRoon | mzLab | Parametri chiave (nome come nel file) |
|---|---|---|---|---|---|---|
| `import.raw` lettura nativa | `AllSpectralDataImportModule` (V; Thermo e Bruker nativi, altri via msconvert: I) | no, chiede mzML (V: `in` = mzML) | no, mzML/mzXML via `MsExperiment` (I) | `convertMSFiles` con msconvert/TRFP (I) | `mzlab_lettori::apri` + trait `Lettore` (V), otto famiglie | mzLab: `Limiti` (memoria), campione `c` |
| `convert.mzml` | no (I) | `FileConverter` (I) | no | `convertMSFiles` (I) | `lettori_mzml::scrivi` (V) e un secondo scrittore `raw::to_mzml` (V, §6 del principale) | zlib, numpress (`Opzioni`, V) |
| `spectra.centroid` | «Mass detection» (V: «Factor of lowest signal», «Noise factor» 5.0) | `PeakPickerHiRes` (I) | `pickPeaks` di MSnbase (I) | via msconvert (I) | `lettori_centroidi`, `centroidi/` con Savitzky-Golay (V) | rumore, larghezza del lisciamento |
| `index.mzidx` | interno (I) | interno | interno | — | `mzidx::build_mzml_spill`, `upload::indicizza_lettore` (V) | `DEFAULT_BUDGET` (memoria, non cambia il risultato) |
| `features.detect` (tracce di massa) | ADAP chromatogram builder (V: «Minimum consecutive scans» 6, «Minimum intensity for consecutive scans» 1e5, «Minimum absolute height» 5e5, m/z 0,002 Da / 10 ppm) | `mtd` di FeatureFinderMetabo (V: `mass_error_ppm` 20, `min_trace_length` 5, `noise_threshold_int` 10) | `CentWaveParam` (V: `peakwidth` 20-80, `noise` 5000, `prefilter` 6/5000) | `findFeatures(algorithm=...)` (V) | `trace::detect` (V: `polarity`, `floor`, `min_points`, `min_height`, `max_gap`, `long_trace`, `axis`) | soglia, punti minimi, tolleranza m/z |
| `features.smooth` | `SmoothingModule` (V: Savitzky-Golay 7) | `use_smoothed_intensities` (V) | dentro centWave (I) | dell'algoritmo scelto | dentro `quality`/`resolve` (I: non letto) | finestra |
| `features.resolve` (picchi di una traccia) | Local minimum resolver (V: «Chromatographic threshold» 0,9, «Min ratio of peak top/edge» 2, «Peak duration range» 0-1,2) | `epd` (V: `enabled`, `width_filtering` fixed) | wavelet di centWave (I) | dell'algoritmo | `resolve::assess_and_resolve` (V; `split`) | rapporto apice/bordo, durata |
| `features.isotopes` | Isotope grouper (V: 0,0015 Da / 3 ppm, RT 0,04 min, carica ≤ 2) | `ffm` (V: `charge_lower_bound` 1, `charge_upper_bound` 3, `isotope_filtering_model`) | no (CAMERA a parte: I) | `generateComponents` (I) | `isotopes::group` (V) | tolleranze, carica |
| `features.ms2_link` | «MS/MS scan pairing» del resolver (V: 0,01 Da / 10 ppm) | no nel passo (I) | `featureSpectra` (I) | `generateMSPeakLists` (V) | `ms2::link` con purezza (V) | tolleranza del precursore |
| `features.estimate` stima dei parametri | no (I) | no (I) | no | no | `params::estimate`, `preview` (V; `floor` assente = stimato) | — |

## 2. Serie (più file)
| Passo | mzmine | OpenMS | XCMS | patRoon | mzLab | Parametri chiave |
|---|---|---|---|---|---|---|
| `series.rt_align` | non nel batch letto (RANSAC: I) | `MapAlignerPoseClustering` (I: nessun `.ini` letto) | `adjustRtime(ObiwarpParam(binSize=0.6))` (V) | `groupFeatures(rtalign=FALSE)` (V) | `rtmap` con ancore (V) | passo, ancore |
| `features.align` | Join aligner (V: 0,0015 Da / 5 ppm, «Weight for m/z» 3, RT 0,1 min, «Weight for RT» 1) | `FeatureLinkerUnlabeledKD` (I) | `groupChromPeaks(PeakDensityParam(bw=30, minFraction=0.4, sampleGroups))` (V) | `groupFeatures(maxGroupRT=6)` (V) | `align`/`famalign` (V; `split_gap` 0,03 min, modo Ion o Family) | tolleranze, frazione minima, gruppi di campioni |
| `features.gapfill` | Peak finder multithread (V: «Intensity tolerance» 0,2, 0,002 Da / 10 ppm, RT 0,1 min, punti ≥ 3) | no standard (I) | `fillChromPeaks(ChromPeakAreaParam())` (V) | via XCMS/OpenMS (I) | `gapfill` (V; `fill_gaps`) | tolleranze |
| `features.ions` (addotti, famiglie) | Ion identity networking, metaCorrelate (V: nomi; parametri non letti) | `MetaboliteAdductDecharger` (I) | CAMERA (I) | `generateComponents` (I) | `family`, `famalign` (V) | lista di addotti |
| `table.filter` | Rows filter, Duplicate filter (V: nomi) | `IDFilter`/`FileFilter` (I) | no | `filter(absMinIntensity, relMinReplicateAbundance, maxReplicateIntRSD, blankThreshold, retentionRange)` (V) | `filters.rs` (vista, registro `filtri/1`), ruoli `campioni/2` (V) | soglie, ruolo del bianco |
| `table.qc` | no nel batch | QC con mzQC (I) | no | no | `qc.rs`, solo numeri (V) | — |
| `table.stats` (PCA, differenza) | dashboard statistico (I) | no | no | no | `statistica.rs`, `statmat.rs` (V) | — |

## 3. Annotazione e TP
| Passo | mzmine | OpenMS | XCMS | patRoon | mzLab | Parametri chiave |
|---|---|---|---|---|---|---|
| `annot.formula` | formula prediction (I) | SIRIUS adapter (I) | no | `generateFormulas("genform", adduct)` (V) | `untargeted_formula`, `mzlab-chem` (V) | elementi, ppm |
| `annot.library` | `SpectralLibrarySearchModule` (V: nome) | `MetaboliteSpectralMatcher` (I) | no | `generateCompounds("metfrag")` o libreria (V) | `mzlab-lib` (`.mzlib`, V) | coseno minimo, picchi |
| `annot.insilico` | no | no | no | MetFrag (V) | `attributio/ms2_engine` sperimentale (V) | — |
| `annot.levels` | no | no | no | `estimateIDConfidence` (V) | `livelli`, cascata di Attributio (V) | — |
| `tp.generate` | no | no | no | `generateTPs("library", "logic", "biotransformer", "library_formula")` (V) | `hypo/` (`ipotesi/1`, Δ di formula; V) | trasformazioni (`add`, `sub`, `retDir`) |
| `tp.link` | no | no | no | `generateComponents("tp")` + `filter(retDirMatch, minSpecSimBoth, minTotFragMatches)` (V) | `interp` (`interp/1`), `tp/1` (V) | soglie di somiglianza |
| `tp.position` | no | no | no | no | `chem/posizioni.rs` (`posiz/1`, V) | — |
| `export.*` | GNPS FBMN, SIRIUS, GraphML, CSV (V) | mzTab, consensusXML (I) | no | `report` (V) | CSV «full» stile mzmine, MGF/MSP, XLSX (V) | formato |

## 4. Struttura chimica e trasformazioni (tipi `structure` e `transform`; il formato della struttura è dello studio dedicato `docs/studi/STRUTTURA_DATO*.md`, record `mol/1`)
| Passo | mzmine | OpenMS | XCMS | patRoon | mzLab | Parametri chiave |
|---|---|---|---|---|---|---|
| `structure.draw` | no (I) | no | no | no | Disegno con Ketcher in iframe `sandbox` (V) | — |
| `structure.import` (SMILES, molfile, SDF, InChI) | strutture delle librerie via CDK (I) | no (I) | no | via rcdk/Open Babel (I) | `grafo::Mol::parse` e `from_molfile` V2000 (V; stereo e isotopi persi) | formato |
| `structure.from_result` | no (I) | no | no | candidati di MetFrag/SIRIUS (I) | «Use this structure» di Tabui (V) | — |
| `structure.render` (con zone evidenziate) | struttura del candidato (I) | no | no | `plotStructure` (I) | `TPMOL.svg` + `parentesi.js`, aloni della reattività (V) | modo del disegno, incertezza |
| `structure.identity` (canonico, InChIKey) | da RDKit/CDK (I) | no | no | via rcdk (I) | nessuna InChIKey calcolata, solo dalle fonti (V); classi di `simmetria.rs` (V) | — |
| `tp.transform` (genitore → prodotto con mappatura atomica) | no | no | no | coppie genitore/TP della libreria, senza mappatura (I) | Δ di formula di `hypo/` (`ipotesi/1`), siti di `posizioni` (`posiz/1`), atomi + Δ di `parz/1` (V); nessuna mappatura atomo per atomo | regola, Δ |
| lettori della struttura | — | — | — | MetFrag, TP «library» (V) | `interp`, `positions_json`, `reactivity_json` (Fukui), RT-logP via InChIKey, librerie (V) | — |

## 5. Catalogo e buchi
- **mzLab ha tutto il tratto grezzo → tabella → TP** (V nel codice), ma come chiamate interne: nessun passo si descrive (niente `step/1`), i parametri sono strutture Rust (`Recipe`, `Params`) e l'interfaccia è scritta a mano.
- **Buchi veri** (nessun modulo mzLab): un grafo della struttura con stereo, isotopi e sali (oggi si perdono alla lettura, V), un calcolo dell'InChIKey, allineamento RT per deformazione continua (Obiwarp: abbiamo solo ancore), export mzTab-M e mzQC, generazione dei TP da struttura (BioTransformer, CTS: esclusa, §8 del principale), lisciatura come passo a sé.
- **Unico nostro** (nessun altro dei quattro): lettura nativa nel browser, stima dei parametri con prova, posizione della sostituzione, livelli LR/HR con prove e non verdetti.
