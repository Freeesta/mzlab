# Study «modular workflows»: the step contract and the workflow format of mzLab

Goal (Federico, 19/10/2026): mzLab reads the raw files of every vendor, converts them to mzML and lets users COMPOSE workflows (preprocessing → peaks → chromatograms → features → identification → TPs), gathered in one interface, on the web or downloaded. Attributio (identification, regulatory lists, ecotox) and Tabui (TPs, transformations, positions, drawing) become families of modules inside Flow, not separate apps. This study chooses the contract of a step and the workflow format on evidence (real third-party files, our code, a measured prototype), not on ideals. Base: `main` at `f072fcd`.

Files of the study:
- this one: verdict, sources, standards, contract, prototype, external tests, plan, constraints, what not to do;
- `docs/studi/workflow/MATRICE.md`: step → who has it (mzmine, OpenMS, XCMS, patRoon, mzLab) → parameters; identification steps with their external truth (§3); Attributio and Tabui mapped to steps and ports (§5); catalogue and gaps;
- `docs/studi/workflow/PROVE_SU_CARTA.md`: four real third-party workflows rewritten in the format, what cannot be expressed and the corrections they produced;
- `docs/studi/workflow/contract.schema.json`: the draft (JSON Schema 2020-12) of `step/1`, `mzflow/2` and `prov/1`;
- `docs/studi/workflow/example.mzflow.json`: a raw → mzML → features workflow with a preset comparison; step descriptors `step.features.detect.json` (real fields and defaults of `trace::Params`, three algorithms), `step.spectra.match.json` (the three scores of `mzlab-lib`), `step.spectra.chimeric.json` (co-isolation), `step.meta.compare.json` (all validate the draft, V);
- `docs/studi/workflow/COMUNICAZIONE.md`: how the modules of the future «Flow» canvas talk (ports, compatibility, proposed missing step, derived order and parallelism, algorithms, presets and comparison; §4b);
- `docs/studi/workflow/PUBBLICAZIONE.md`: which crates to publish and in which order, licences, what is needed (§12, analysis only);
- `crates/mzlab-cli/examples/flow.rs`: the prototype (§6).

**Marks**: **V** = verified (read in a real file or in the code, or measured here); **I** = hypothesis (not read or not tested).

**Language**: everything that belongs to the modules is in ENGLISH: field names, ids of steps, ports, parameters and algorithms, descriptions, labels, error keys and texts, the Flow interface, and the internal documentation and studies such as this one. Italian stays only for Teoria, Drawing texts for students and laboratory data. Schema, examples and prototype follow the rule (V); the only exceptions are names of records that already exist (`campioni/2`, `interp/1`, `livelli/1`) and the `mzflow/1` keys, which a reader maps (`formato`, `programma`, `operazioni`).

## 0. Verdict
1. **No new format from scratch is needed**: mzLab already has `mzflow/1` (`mzlab/web/workflow.js`: file SHA-256, version, commit, parameters, operations in order; V) and the `feature_pipeline/1` schema (`docs/schemi/`, recipe, proofs of the estimates, table hash; V). The proposed format is **`mzflow/2`** = `mzflow/1` + a `steps` field (the declarative PLAN, a graph), beside `operations` (the LOG of `mzflow/1`, unchanged; English keys, §4).
2. **The missing piece is the step descriptor** (`step/1`): parameters are Rust structs today (`Recipe`, `trace::Params`, V) and the Pipeline panel draws them by hand. With `step/1` (typed ports, alternative algorithms, parameters in JSON Schema, a version tied to the result, error keys, `.wasm` module, palette family) a module draws itself.
3. **Provenance = a subset of PROV-JSON** (`prov/1`): entities with SHA-256, activities with canonical parameters, time and memory. The prototype already writes it (V).
4. **Determinism proven on raw → mzML → features**: same bytes on two runs, native and WASM (V, §6). Two defects found: two mzML writers with different results downstream, and the recipe fingerprint computed on the Rust `Debug` text (§6.3).
5. **The chain reaches identification**: chimeric handling → formula from MS1 and MS2 → library match (three scores) → in silico candidates → RT plausibility → Schymanski level → regulatory and hazard records, and back to TPs through `structure`/`transform` (§4d, `MATRICE.md` §3 and §5). Every step already exists as code, in `main` or in open PRs, except library-assisted deconvolution (V).
6. **First chain to close**: raw → mzML → index → features → table → identification → timolol TPs (§8), with external oracles (mzmine, OpenMS, ThermoRawFileParser, public libraries) and the tests of §7.

## 1. Raw material read
- **mzmine 4.0.2**: a real FBMN batch, `https://raw.githubusercontent.com/lfnothias/FBMN_annotation_fusion_visualization_Cytoscape/main/results_download/mzmine/mzmine_microbial_workshop.mzbatch.xml` (935 lines; V). Root `<batch mzmine_version>`, flat steps `<batchstep method parameter_version>`, parameters by human label, implicit input `BATCH_LAST_FEATURELISTS`; no hash, no error, no graph (V).
- **OpenMS**: FeatureFinderMetabo 2.8.0 `.ini`, `https://raw.githubusercontent.com/PayamEmami/nf-core-metabolinden/5a0b165eff54b1f62e366545365ff61ee65c9d07/assets/parameters/FeatureFinderMetabo.ini` (V). `<PARAMETERS version>` + `<NODE>`/`<ITEM name value type restrictions advanced>`; one `.ini` per tool, the chain lives outside (Nextflow, KNIME). No `.ini` read for MapAligner and FeatureLinker (I).
- **XCMS**: vignette `https://raw.githubusercontent.com/sneumann/xcms/devel/vignettes/xcms.Rmd` (V): `findChromPeaks(CentWaveParam)` → `refineChromPeaks` → `adjustRtime(ObiwarpParam)` → `groupChromPeaks(PeakDensityParam)` → `fillChromPeaks`. A script, no workflow file; parameters travel in the object (I).
- **patRoon 3.1.0** (GPL-3): handbook `https://rickhelmus.github.io/patRoon/handbook_bd/` pages `features`, `filtering`, `genTPs`, `TPsExamples` (V; only function and argument names, no code copied). Dedicated study: `docs/studi/patRoon_TP.md`.
- **PSI**: schema `mzML1.1.1.xsd` (58 KB) and example `1min.mzML` from the HUPO-PSI repository (V); `psi-ms.obo` `data-version 4.2.6` of 14/10/2026 (V); mzQC 1.0.0 schema (V); mzTab-M 2.1-dev: detail pages not read (I).
- **Identification papers** (metadata read on PubMed, V): DecoID, library-assisted deconvolution of chimeric spectra (Stancliffe et al., Nat. Methods 2021, doi:10.1038/s41592-021-01195-3); spectral entropy (Li et al., Nat. Methods 2021, doi:10.1038/s41592-021-01331-z); modified cosine in molecular networking (Watrous et al., PNAS 2012, doi:10.1073/pnas.1203689109). Levels: Schymanski et al. 2014, the reference of `data/livelli-1.json` (V).
- **Our code** (V): trait `Lettore` and `apri` (`crates/mzlab-core/src/lettori.rs`, `crates/mzlab-lettori/src/lib.rs`), `lettori_mzml::scrivi`, `mzidx::build_mzml_spill`, `upload::indicizza_lettore`, `mzlab_feat_wasm::run::{recipe_for, detect, read_files, align}`, `pipeline::analyse`, `recipe::Recipe` (text + FNV fingerprint), `mzlab-cli` (`docs/agenti/cli.md`), `TP_Mine/files/feat-worker.js` (already orchestrates steps in JS), the exports of 10 `.wasm` modules (2 to 32 functions each).
  For identification: `mzlab-lib/src/{search,scores,decoy}.rs`, `attributio/{ms2_engine,annotate,identity,tox}`, `mzlab-core/src/subformulas.rs`, `interp/ms2link.rs` (`MATRICE.md` §3, §5).
- **Data with truth**: Rosina corpus and `verita/1` (`docs/studi/ROSINA.md` §F; private), series D1 (MetaboLights, MAF) and D2 (synthetic) of the bench (`docs/agenti/mzfinder-benchmark.md`, V), timolol truth `timolol_truth.json` (private). For the prototype: `small.RAW` (LTQ, 48 spectra, 1.5 MB) from the ThermoRawFileParser tests (Apache-2.0), used only locally and never saved in the repository.

## 2. Standards before inventing
| Standard | What we reuse | What not, and why |
|---|---|---|
| mzML 1.1 (PSI) | the `mzml` type as is; `dataProcessingList/processingMethod` with a PSI-MS term for EVERY step that changes spectra | as a workflow or provenance format: only a linear list per file, no inputs or typed parameters (V) |
| PSI-MS (`psi-ms.obo` 4.2.6) | ~10 accessions in the `cv` field of a step (MS:1000544 conversion, MS:1000035 peak picking, MS:1000593 baseline reduction, MS:1000594 low intensity data point removal, MS:1000745 RT alignment; V) | shipping the OBO (1.9 MB) in the site |
| mzQC 1.0 | the model of `inputFiles` (name, format, properties with checksum) and `analysisSoftware`; a future `qc` export | as a provenance container: it has no step graph (V) |
| mzTab-M 2.x | a future table export, after reading the specification | as the internal table format: heavy (mandatory MTD, study/assay), identification oriented (I) |
| CWL v1.2 / Nextflow | the port shape (`type`, `doc`, `default`, `format`) and the idea of a step that declares inputs and outputs (V) | the engine: neither runs in the browser; students' files never leave the computer |
| W3C PROV-JSON | `entity`/`activity`/`agent`/`used`/`wasGeneratedBy`: this is `prov/1` | the rest (bundles, delegation, qualifications): not needed. It is a Member Submission, not a Recommendation (V) |
| RO-Crate 1.2 + Workflow Run Crate | later a zip «package» export (`CreateAction` with `instrument`/`object`/`result`) | now: verbose JSON-LD, no benefit for students (V) |
| JSON Schema 2020-12 | step parameters and form generation (`title`, `description`, `default`, `minimum`, `maximum`, `enum`; V); already used in `docs/schemi/` | a generic validator in Rust: dependencies; a reduced check is enough (type, limits, enum, parameters of the chosen algorithm) |
| Schymanski levels (2014) | the scale of `annot.level` (1, 2a, 2b, 3a, 3b, 4, 5), already in `livelli-1.json` (V) | a verdict without evidence: each level carries the evidence that reached it |

## 3. Matrix and gaps (detail: `workflow/MATRICE.md`)
- mzLab covers the whole raw → table → identification → TP stretch (V; part in open PRs), but as internal calls without descriptors.
- Real gaps: smoothing as a step, continuous-warping RT alignment (only anchors), library-assisted deconvolution of chimeric spectra, InChIKey computation, mzTab-M and mzQC export, TPs from structure (excluded, §9).
- Ours only: native reading in the browser, parameter estimation with proof (`params::estimate`), position of the substitution, levels with evidence, regulatory and hazard records next to the identification.

## 4. The contract (draft in `workflow/contract.schema.json`)
**Data types** (`dataType`, 19): `raw`, `mzml`, `mzidx`, `xic`, `peaks`, `rtmap`, `table`, `samples`, `spectra`, `library`, `formulas`, `candidates`, `annot`, `tp`, `structure`, `transform`, `comparison`, `qc`, `report`. `mzidx` is a type of its own because it is the real currency of the engine: every step after reading works on the index (V: `Index<R>` in the signatures of `pipeline::analyse`, `params::estimate`, `run::detect`).

**The step `step/1`** (one per kind and version, shipped with the module that runs it):
- `id` (`features.detect`), integer `version`: RAISED whenever the same inputs and parameters can give a different output (like mzmine `parameter_version`, but tied to the result);
- `title`, `doc`, `icon` (a `QICON` key), `cv` (PSI-MS term when one exists), `package` (palette family: `core`, `attributio`, `tabui`);
- `in`/`out`: ports with `type`, `many` (one per file of the series), `optional`, `accepts`, `requires`;
- `params`: JSON Schema with `x-unit`, `x-advanced` (behind «More», like OpenMS `advanced`), `x-group`, `x-estimate` (absent value = estimated by the step, which writes its proof, like `floor` today), `x-noresult` (does not change the result: memory, threads; left out of the fingerprint);
- `runs_in`: `.wasm` module with its ceiling, and/or `native`; `deterministic`; `errors`: the keys it may return, in the API shape `{error, error_key, params}`;
- `algorithms[]` and `default_algorithm`: interchangeable methods with the same ports (below); `presets` (each names its `algorithm`) and `default_preset`.

**The workflow `mzflow/2`**: `format`, `program` (`name`, `version`, `commit`), `inputs` (name, SHA-256, bytes, type) and `steps` (instances: unique `id`, `step` = `id@version`, `algorithm`, `preset`, `in` = references `file:NAME` or `STEP.OUTPUT`, `params` only where they differ from the defaults). It is an acyclic graph; the interface shows a chain while the graph has a single path. `operations` is the log of `mzflow/1`; the reader still opens old files and maps their Italian keys (`formato` → `format`, `programma` → `program`, `operazioni` → `operations`).

**Provenance `prov/1`**: `agent` (mzLab, version, commit, `wasm` or `native`), `plan_sha256`, `entity` (SHA-256, bytes, type, label), `activity` (step, `algorithm`, CANONICAL parameters with the estimates, `params_sha256`, `used`, `generated`, `ms`, `heap_peak_bytes`, `error`).

**Rules**:
1. Canonical parameters = JSON with sorted keys, every value present (defaults and estimates included), numbers in the shortest form that reads back equal; the fingerprint is the SHA-256 of that text, without the `x-noresult` keys.
2. Cache key = (input SHA-256, `id@version`, algorithm `id@version`, `params_sha256`); same key → same result, reused without recomputing.
3. Immutable inputs: a step never changes data, it produces new data (correction 7 of `PROVE_SU_CARTA.md`).
4. Explicit units: `mz_tol {da, ppm}`, RT in declared `min` or `s`; never sign codes.
5. Before starting, the whole plan is checked: port types, references, versions present, step available on the platform (`err.flow.unavailable`), parameters within limits.
6. Errors stop at the step: `prov/1` writes the activity with `error`, the outputs already made stay valid and cached.

**Alternatives per step** (detail and comparison: `COMUNICAZIONE.md` §4). A step has a default algorithm (`default_algorithm`: the best known for the typical case, justified by a bench measure) and others selectable with the SAME ports.
Each entry of `algorithms[]`: `id`, `version`, `title`, `doc`, `source` (DOI of the paper it is written from), `applicability` (when it applies: `resolution`, `polarity`, `ms_levels`, `spectrum_mode`, `peak_fwhm_s`, `min_files`, `needs` such as `internal_standards`), its own `params` (those common to all are in the step `params`), `status` (`available` or `planned`). The interface suggests the algorithms whose applicability matches the input facets.

| Step | Default | Alternatives |
|---|---|---|
| `features.detect` (peaks) | `masstrace` (`trace::detect`, available, V) | `centwave` (Tautenhahn 2008, HR, planned), `threshold` (threshold and derivative, LR, planned) |
| `series.rt_align` | `anchors` (`rtmap` with anchors, available, V) | `internal_standards` (needs `internal_standards`, planned), `linear`, `obiwarp` (Prince 2006, planned) |
| `spectra.match` (MS2) | `spectral_entropy` | `cosine`, `modified_cosine` (all three in `mzlab-lib/src/scores.rs`, available, V) |
| `spectra.chimeric` | `purity_flag` (`fill_purity`, available, V) | `intruders` (PR #1061), `library_deconvolution` (DecoID-like, planned) |

**Comparison** (`meta.compare@1`): a meta-step that runs the same step with 2 or more variants (algorithm, preset, parameters) on the same inputs and writes `comparison` data: matched features, only in A, only in B, median Δm/z (ppm) and ΔRT, |log2| of the area ratio, Spearman; for scores, rank correlation, top-1 agreement and changed levels. It serves to choose the default on evidence, to validate a new implementation against the old one and, in teaching, to show what a method changes.

**Generated interface** (I, to test): one form per step from the schema (number with unit and limits, menu from `enum`, groups from `x-group`, «More» for `x-advanced`, «Auto» for `x-estimate` with the proof beside it), English only, icons from `QICON`, transitions off under `prefers-reduced-motion`, keyboard and `aria-label` from the `title`s. Few clicks: a saved workflow is re-applied with «Open» → choose files → «Run».

## 4b. How modules talk (detail: `workflow/COMUNICAZIONE.md`)
A wire carries a descriptor (`type`, `sha256`, facets such as `hr`/`lr`), never bytes. An output can be wired to an input if type (or `accepts`), cardinality, facets (`requires`) and acyclicity hold; if a registered conversion (`converts`) is missing, the program PROPOSES it with one click.
Order (Kahn) and parallelism (independent branches, `per_item` per file; workers in the browser, threads in the app) are derived from what each step reads and writes; the only shared resource is memory (`memory_hint_bytes`). Each step has presets with `when` on the facets (e.g. `hr`, `lr`, as `run::recipe_for` already does, V) and alternative algorithms inside the same step, with the same ports. The fields are in the draft and the examples validate it (V).
Chemical structure is a port like any other (`structure`, format of the dedicated study `docs/studi/STRUTTURA_DATO*.md`, record `mol/1`): it enters from Drawing, from pasted text or from a result with one click, and leaves as a drawing. Parent → product transformations are a type of their own (`transform`: Δ, rule, atom mapping, sites with support), read by `tp.link`, by the positions and by the drawing; the readers of a structure are in `MATRICE.md` §4.

## 4c. Why JSON
- **JSON (chosen for contracts, descriptors, workflows and `prov/1`)**: it is already the language of `mzflow/1`, `feature_pipeline/1`, `campioni/2` and all `/api` routes (V); the engine already has its own reader and writer with no dependency (`mzlab_core::json`, used by the prototype, V) and the browser has `JSON.parse`; JSON Schema 2020-12 validates and generates the forms; the canonical form (sorted keys, shortest numbers, rule 1) gives a stable fingerprint. Files are small (`example.mzflow.json` 1 295 B, V) and read well in a diff.
- **YAML**: handier by hand and with comments, but a large specification, implicit types (`no` → false, `1.10` → 1.1), no reader in the engine: one more dependency in modules close to their ceiling (I: bytes not measured) and a hard canonical form. Workflows are saved by the program, not written by hand; notes go in `doc` and `label`.
- **TOML**: excellent for flat configuration (`Cargo.toml`), awkward for a graph of instances with nested `in` and `params`; no reader in the engine and no standard schema (it validates only after conversion to JSON).
- **MessagePack, CBOR, protobuf**: compact and fast, but unreadable and without git diffs for files of a few KB, where speed does not matter; protobuf needs a compiled schema. CBOR has a deterministic encoding (RFC 8949 §4.2): useful only to sign binary records, not needed today.
- **Large data**: in standard or binary formats, never inside a JSON: mzML (`indexedmzML`, numpress), `.mzidx`, `.mzfeat`/`.mzfeatf`, `.mzlib`. Workflows and `prov/1` cite them by SHA-256 and type.

## 4d. Identification and the Attributio and Tabui families (detail: `workflow/MATRICE.md` §3 and §5)
- **Chain** (all `requires: {"resolution": ["hr"]}`; the LR world stops at peaks, principle 1): `features.ms2_link` → `spectra.chimeric` (purity, intruders, deconvolution) → `formula.ms1` (exact mass + isotopes) and `formula.ms2` (fragment subformulas, neutral losses, H shifts) → `spectra.match` (library: cosine, modified cosine, spectral entropy; decoys and q-values) and `spectra.insilico` (MetFrag-like candidates) → `rt.predict` → `annot.level` (Schymanski) → `annot.regulatory`, `annot.hazard` → `export.attributio`.
- **Scores aware of chimeric spectra**: `spectra.chimeric` writes purity and intruders into `spectra`; `spectra.match` and `formula.ms2` read them (peaks explained by an intruder do not count for the precursor; low purity caps the level at 3), so the evidence of `annot.level` names the purity.
- **Families**: `package: attributio` for identification, regulatory lists and ecotox; `package: tabui` for TP generation, linking, positions, reactivity and drawing; `package: core` for the rest. On the palette a family is a group of modules; a whole Attributio or Tabui session is just a saved `mzflow/2`.
- **Fusion through ports**: Tabui gives Attributio `transform` and `structure` (a TP becomes a suspect, with `modified_cosine` at the Δ of the transformation); Attributio gives Tabui an identified parent (`annot` level 1-2), RT and formula constraints, and regulatory or hazard records on the TP row. Both write `candidates` and `annot` with the same fields (one identity record).

## 5. Paper test (detail: `workflow/PROVE_SU_CARTA.md`)
Four real workflows (mzmine FBMN, OpenMS FFM, XCMS vignette, patRoon TP) rewritten in the format. All of them are expressed along detection → alignment → filling; the algorithms we do not have (centWave, Obiwarp, standalone smoothing) and external programs (BioTransformer, CTS, MetFrag) are not.
Eleven corrections, already in the draft: `id` per instance, compound tolerances, `algorithms[]` and the instance `algorithm` for the choice of method, explicit references, `x-advanced`, declared conversions, immutable data, `samples` as a type, `err.flow.unavailable`, user lists as inputs with a hash, a graph from day one.

## 6. Minimal prototype: raw → mzML → features
`crates/mzlab-cli/examples/flow.rs` (prototype, not in the site): reads an `mzflow/2` (English keys, steps `convert.mzml@1` → `features.table@1`) as a linear chain, runs the steps with the engine code (`mzlab_lettori::apri`, `lettori_mzml::scrivi`, `mzlab_cli::features`, made public to pass the work folder: WASI has no temporary folder) and writes `prov.json` with SHA-256, time and heap peak of each step (counting allocator, as in `examples/mzml_memoria.rs`).
```
cargo run --release -p mzlab-cli --example flow -- FLOW.json WORKDIR                        # native
cargo build --release --target wasm32-wasip1 -p mzlab-cli --example flow                    # WASM (WASI)
node wasi.mjs target/wasm32-wasip1/release/examples/flow.wasm DIR /d/FLOW.json /d/OUT       # Node 22, node:wasi, DIR mounted on /d
```

### 6.1 Measures (V, cloud container x86_64, 4 CPU, `small.RAW` LTQ 1 504 354 B, 48 spectra)
| Run | conversion (ms) | features (ms) | conversion heap peak | features heap peak | process |
|---|---|---|---|---|---|
| native, run 1 / 2 | 52.1 / 51.1 | 49.6 / 49.0 | 2 231 848 B | 6 426 896 B | 0.11 s, RSS 11.1 MB |
| WASM (Node 22 WASI), run 1 / 2 | 124 / 119.4 | 118.1 / 110.3 | 2 156 824 B | 6 102 097 B | linear memory 7.1 MB; compile 10 ms |
- **Identical hashes** on four runs: `conv.mzML` (1 829 427 B) SHA-256 `92dd492951bfec2c…`, `feat.csv` `4bd43e415156e608…`, equal between native and WASM, and equal after the switch to English keys; the `prov.json` of both validates `prov1` of the draft (V). The heap peak differs (32-bit pointers in WASM), the result does not.
- **Direct path = mzML path**: `mzlab-cli features small.RAW` and `features conv.mzML` give the same 3 rows and the same areas (V).
- **Scale**: the file is small (3 features); time and memory on real files are to be measured with the same example on the Rosina and timolol files (F8 of Rosina, on the Mac). WASI is not the site: in the browser the steps run in the existing modules (§9).

### 6.2 What the prototype showed
1. **Two mzML writers**: `raw::to_mzml` (example `raw2mzml`, software «opentfraw 0.1.0», 1 846 674 B) and `lettori_mzml::scrivi` (software «mzlab 1.0.1», 1 829 427 B). Features from the first have areas 0.014 % different (e.g. 2 853 741.7 against 2 853 340.2; same heights; cause not investigated, I). In the contract `convert.mzml` is ONE step and its version names the writer.
2. **The written mzML is not `indexedmzML`** (no `<offset>`), declares CV 4.1.163 (current 4.2.6) and a single `processingMethod` (conversion). Test F6 of Rosina and §7.2 need the index, the updated CV and one `processingMethod` per step that touches spectra.
3. **CSV column names come from the file name** (`datafile:small_1:area`): two identical files with different names give different CSVs. In the contract identity is the SHA-256, the name is a label; hash comparisons are made with equal names.

### 6.3 What the code showed
- `Recipe::file_text` is the `{:?}` (`Debug`) text of the Rust structs with a 128-bit FNV fingerprint (V): renaming a field or changing an `f64` format changes the fingerprint without changing the result. Fine as an internal cache key; the contract needs the canonical form of rule 1 (§4).
- `Recipe::row_window` uses `ppm < 0` to mean «Da» (V): allowed inside the engine, forbidden in the contract (rule 4).

## 7. External tests (designed, not run)
1. **Same table as mzmine and OpenMS**: same mzML file, `tools/bench/` (V: `corri_mzmine.py` headless batch 3.9.0, `corri_openms.py` FeatureFinderMetabo + KD linking, `da_csv` adapters for mzmine, XCMS, OpenMS, patRoon). A row matches at HR |Δm/z| ≤ 5 ppm and |ΔRT| ≤ 0.1 min; at LR |Δm/z| ≤ 0.5 Da and |ΔRT| ≤ 0.1 min.
   Outcomes declared beforehand: recall of the oracle rows above 10 times the noise ≥ 0.9; median |log2(mzLab area / oracle area)| ≤ 0.15; Spearman of the areas ≥ 0.9. Agreement is measured, not equality (the oracle is not the truth); the truth is in D1, D2 and the timolol series.
2. **Validated mzML**: XSD `mzML1.1.1.xsd` and the `idx` schema of the PSI page (name to re-check), the HUPO-PSI Java semantic validator with `ms-mapping.xml` offline, OpenMS `FileInfo -v`; reading back = reader (F6 of Rosina). It would fail on the index today (§6.2 point 2: I, not run).
3. **Re-run with identical hash**: every step, two runs and three platforms (Linux x86_64, Mac arm64, WASM in Chromium); the first piece is done (§6.1). In CI: a `cargo test` of a small synthetic `mzflow/2` that compares `generated.sha256` with written values.
4. **The timolol task with fewer clicks than mzmine**: from the 13 `.raw` to the TP table; clicks and keys counted with Playwright (mzLab) and with a screen recording (mzmine 4.10 on the Mac, loaded batch against built batch). Criterion: mzLab ≤ half the clicks of mzmine with the same result within the tolerances of test 1.
5. **Library match with external truth**: compounds present in two public libraries (MassBank against GNPS or MoNA; licences per record to read, I): the spectra of one library are the queries, the other is the library, with the decoys of `decoy.rs`. Measured per score (`cosine`, `modified_cosine`, `spectral_entropy`): rank-1 rate, q-value at 1 % FDR. Declared beforehand: spectral entropy ≥ cosine on rank-1 (the claim of Li et al. 2021, checked on our code), every score deterministic between native and WASM.
6. **Formula from MS2**: MassBank records and CASMI 2016/2017 challenges with known formula (I: licences to read). Declared beforehand: with MS2 subformulas the true formula ranks first more often than with MS1 alone; the gain is reported with its confidence interval, not assumed.
7. **Chimeric spectra**: pairs of MassBank spectra of known compounds mixed in silico (ratios 1:1 to 1:10, precursors 0.3-1.5 Da apart, window 1 Da), the truth being the mix. Declared beforehand: the purity written by `spectra.chimeric` is within 0.05 of the true share; with intruders discounted the true compound ranks first more often than without; deconvolution is measured only when implemented.
8. **Levels**: fixed fixtures, one per Schymanski level, with exactly the evidence of the 2014 paper (standard, library, diagnostic fragments, formula only, mass only); `annot.level` must return the level and name the evidence. On the timolol series the levels are compared with the private truth (`timolol_truth.json`), never shipped.

## 8. First flagship workflow and work plan
Chain: `import.raw` (the 13 timolol `.raw`) → `convert.mzml` (optional, for export) → `index.mzidx` → `features.estimate` → `features.detect` → `features.align` → `features.gapfill` → `table.filter` (`campioni/2`) → `features.ms2_link` → `spectra.chimeric` → `formula.ms1`/`formula.ms2` → `spectra.match` → `annot.level` → `tp.link` (`interp/1`, `tp/1`) → `tp.position` → Drawing («Open in Draw»). All steps exist as code (V, `MATRICE.md`; some in open PRs); descriptors, canonical form, `prov/1` and the generated form are missing.

| DEV chat (Sonnet) | Scope | Test |
|---|---|---|
| W-1 descriptors | `step/1` of the core steps of the chain as Rust data next to the code, exported to JSON; no change of result | descriptors validate the schema; step outputs are byte for byte today's |
| W-2 canonical form and `prov/1` | canonical parameters, `params_sha256`, `prov/1` in `mzlab-core`; `mzflow/2` read and checked (rule 5) | re-run with identical hash, native and WASM (test 3) |
| W-3 one mzML | `convert.mzml` = `lettori_mzml::scrivi` with `indexedmzML`, CV 4.2.6, one `processingMethod` per step; `raw::to_mzml` only behind it | test 2 on the Rosina files (F6) |
| W-4 generated form | form from the schema in the Pipeline panel (English, `QICON`, `prefers-reduced-motion`, keyboard, axe) | `e2e_pipeline` and `e2e_axe_grafici` green; tour valid |
| W-5 oracles | test 1 on the D1 and timolol files with `tools/bench/` | numbers in the private repository, outcomes against the declared thresholds |
| W-6 clicks | test 4 | count written in `misure.md` |
| W-7 identification descriptors | `step/1` (`package: attributio`) of `spectra.chimeric`, `formula.ms1`, `formula.ms2`, `spectra.match`, `spectra.insilico`, `rt.predict`, `annot.level`, `annot.regulatory`, `annot.hazard`; the shared `candidates` and `annot` records | descriptors validate; outputs unchanged against the parity data of each package |
| W-8 Tabui steps and fusion | `step/1` (`package: tabui`) of `tp.generate`, `tp.link`, `tp.position`, `structure.reactivity`, `structure.render`; adapters `interp/1` → `tp`, sites → `transform.sites`; a TP as a suspect of `spectra.match` | the timolol TPs found with and without the fusion, levels compared with the private truth |
| W-9 identification truth | tests 5-8 with public data in `tools/bench/`, results in the private repository | outcomes against the thresholds declared in §7 |
Order: W-1 and W-3 in parallel; W-2 after W-1; W-4 after W-2; W-5 and W-6 after W-4. W-7 after W-1 and after the MS PRs below are merged; W-8 after W-7; W-9 can start with W-7 (it only needs the existing entry points). Local (Mac): mzmine 4.10, ThermoRawFileParser, F8 measures.

**Open PRs: what changes, so that no work is redone** (state read on GitHub on 19/10/2026):
- **#1059, #1060, #1061, #1065 (MS-02..05: in silico, rearrange, RT prediction and chimeric intruders, `.wasm` and admin panel)**: nothing to rewrite. They become the algorithms and steps `spectra.insilico`, `formula.ms2`, `rt.predict`, `spectra.chimeric` (`intruders`).
  Only asks: entry points stay pure functions of typed inputs (no I/O, deterministic, as their parity tests already require); the worker op names of #1065 stay stable, because the Flow planner will call them; the new `.wasm` keeps its own measured ceiling (no existing ceiling raised). Descriptors come later from W-7, next to the code.
- **#1064, #1070 (RG-02, RG-03: hazard records T1-T5, regulatory lists, amenability)**: the versioned snapshots are already what correction 10 asks (inputs with a hash); they become `annot.hazard` and `annot.regulatory`. Ask: keep the snapshot readers separate from the row logic, as `tox/snapshot.rs` already does (V).
- **#1091 (TT-R2, merged)**: `chem/posizioni`, `localize`, `interp/sito.rs` and `interp/disegno.rs` are `tp.position` and `structure.render`; the `site` and `drawing` keys of `interp/1` map to `transform.sites` and to the drawing. Nothing to redo.

## 9. Constraints
- **`.wasm` ceilings may not be raised** (V: `tools/build_feat_wasm.sh`, `pages.yml`): finder 512 000 B, `sciex` 327 680, `chem` 569 344, engine 1 703 936. The descriptor and the canonical form are small but do not fit in a module already near its ceiling: in the browser steps stay in the modules that have them; plan checking and `prov/1` go into a NEW module (ceiling fixed at the first measure, I) or into the engine only if the measure allows it. Orchestration stays in the JS worker as `feat-worker.js` already does (V): it calls modules, it does not compute.
- **Licences**: mzLab MIT; patRoon GPL-3 → algorithms rewritten from the original papers, never copied or translated code (`docs/studi/patRoon_TP.md` §5). mzmine, OpenMS, XCMS: licences to read before reusing anything (I: OpenMS BSD-3, XCMS GPL, mzmine 4 with its own terms); same limits, parameter files are read, not code. External data only with a licence read (MassBank, GNPS, CASMI, RepoRT: I).
- **Module language**: English for every name, id, description, label and error (paragraph «Language» at the top), m/z, RT, XIC, TIC, MS2 as always; Teoria and Pratica do not change.
- **Principle 1**: in the LR world a workflow produces no TPs, no formulas and no calibration lines: the `tp.*`, `formula.*`, `spectra.match`, `spectra.insilico` and `annot.*` steps require `hr`, or run in mzFinder.

## 10. What NOT to do (and why)
1. Do not adopt CWL, Nextflow, Galaxy or KNIME as the engine: they do not run in the browser and data must not leave (V for CWL/Nextflow).
2. Do not use mzTab-M or mzML as the workflow or internal table format: no graph and no typed parameters (V for mzML; I for mzTab-M).
3. Do not call external programs from a step (R, Java, BioTransformer, CTS, MetFrag, SIRIUS): heavy, network, licences.
4. Do not copy or translate GPL code (patRoon, XCMS): only ideas and papers.
5. Do not invent a second session format: `mzflow/2` extends `mzflow/1`, which stays readable.
6. Do not make data identity depend on the file name, nor the fingerprint on the `Debug` text (§6.2-6.3).
7. Do not keep two seemingly equivalent mzML converters (§6.2 point 1).
8. Do not put the workflow engine in the main module near its ceiling; do not raise ceilings.
9. Do not promise equivalence with mzmine or OpenMS parameters: a batch import is a declared translation (`PROVE_SU_CARTA.md` B).
10. Do not build the «Flow» canvas before the plan check in Rust: the canvas draws, compatibility and order are decided by the engine (COMUNICAZIONE.md §5), otherwise the rules are written twice.
11. Do not make each algorithm a step of its own: alternatives live in `algorithms[]` of the same step, so they swap without rewiring and the comparison has the same ports.
12. Do not put Italian in the modules (fields, ids, texts, errors) or in new studies: a second language in the contracts means two names for the same thing and maps to maintain.
13. Do not write workflows in YAML or TOML, nor in binary (§4c).
14. Do not keep Attributio and Tabui as apps that read each other's files: they meet only through typed ports (`structure`, `transform`, `candidates`, `annot`), otherwise every change breaks the other side.
15. Do not score a chimeric spectrum as if it were pure: purity and intruders travel with the spectrum and cap the level (§4d).

## 11. Decisions to take (recommendation)
1. **Name and place**: `mzflow/2` with `steps`; `step/1` and `prov/1` in `docs/schemi/` when W-1 and W-2 implement them (drafts here today).
2. **Where the plan check runs in the browser**: a new small module (recommended) against the engine (only if the measure allows it).
3. **Single mzML writer**: `lettori_mzml::scrivi` (recommended: it is the one of the registry and gives the same areas as the direct path).
4. **mzmine batch import**: only after W-4, as a translation that warns about untranslated parameters (like `parametri_non_letti` of `tools/bench`).
5. **Default MS2 score**: `spectral_entropy` until test 5 says otherwise (recommended on the published evidence; to confirm on ours).

## 12. Publishing the libraries (detail: `workflow/PUBBLICAZIONE.md`)
Proposed order: first the numpress codec split from `mzlab-core` (small, external truth in the specification), then `mzlab-lib`, then the readers; npm afterwards. MIT licence; dependency licences are already checked on every PR by `cargo deny` (V). Missing: an English and stable API, rustdoc, `publish = false` on all crates until Federico's go-ahead. No outward action without his go-ahead.
