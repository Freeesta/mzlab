# Identification levels (Tabui, Attributio, untargeted flow)

Definitions chosen by Federico. They apply to Tabui (low-resolution MS2), Attributio, the "What is it?" view, "Identify all MS2" and the mzFinder untargeted flow. Rules in `mzlab/web/livelli-1.js` (data), cascade in `mzlab/web/livelli.js`, Rust copy in `crates/mzlab-feat/data/livelli-1.json` (`python3 tools/export_levels.py`).

## Schymanski scale (identification level)
- **L1**: reference standard marked by the chemist (mass, RT, MS2).
- **L2a**: match of the high-resolution MS2 spectrum against an experimental library (mzCloud, MassBank, MoNA). An in silico library or an MS2 without a library never gives 2a (`evid/graph.rs`: `LibHit.experimental`, `max_level`).
- **L2b**: probable structure supported by diagnostic fragments and in silico spectra (MetFrag, CFM-ID, SIRIUS/CSI:FingerID), or by unambiguous chemical-context evidence without a library standard. **The chemist assigns 2b; the program writes it by itself only with `[levels] auto_2b = true` (off by default, marked "auto-2b", counted separately, with a log)**: as a rule the program proposes the evidence and the chemist can always change what is written automatically; the level lives in "decided" (`LIVELLI.decide`: from 2a, 3a, 3b or 4; rejected on 5 and 0).
- **L3a**: few positional/isomeric candidates among homologues (proposed by the program for a candidate that fails confirmation, provisional, conflicting names, indistinct isomers, RT isomer, expert review, CSI:FingerID).
- **L3b**: generic classes with many plausible isomers (proposed for the analogue only; partial structures of the Drawing `parz/1`, `level: "3b"`).
- **L4** unique formula; **L5** exact mass, **L0** detected without annotation.
- A CANOPUS class (ClassyFire, NPClassifier) is a separate annotation (columns `classes`/`class_probs`, threshold `canopus_min`): it neither assigns nor raises any level, in any profile.
- At unit resolution (profile `lr`, `tetto` "4" in `livelli-1.json`) a nominal mass is a candidate and an MS2 does not raise the level: the profile currently emits only 4 (the nominal formula is enough to explain the ion) or 5.
- **Tabui** (unit resolution): the program proposes no Schymanski level (`level` null, "Lvl" column empty) and shows only the rule's "Prev. N"; the chemist chooses and changes the level in the open row (`TPDEC`, 2b stays rejected without a proposed level). `level_text` says what the data support: "Proposed formula" needs a real chromatographic peak, the decaying precursor, a single step and a hypothesis class above chance (`legacy/lr/summary.rs`, `docs/agenti/mzfinder-lr.md`). A matched MS2 changes nothing (D6 = no).

## Levels reachable by resolution
Principle: the program never shows a level beyond the profile's `tetto`; in low resolution (LR) the cap is 4 (Tabui decision D6), so L1-L3 are not emitted. The rule stays `livelli.js` / `livelli-1.json` (parity `tests/livelli_casi.json`).

| Level | Needs | Emitted in LR |
|---|---|---|
| L1 | standard: mass, RT, MS2 | no (profile `lr` without `standard`, above the cap) |
| L2a | experimental high-resolution MS2 + library | no (HR spectrum) |
| L2b | diagnostic fragments / in silico / context, decided by the chemist | no (above the cap) |
| L3a/L3b | positional candidates or class + MS2 or library | no (above the cap) |
| L4 | unique formula | yes: the nominal formula is enough to explain the ion |
| L5 | exact mass | yes (without evidence) |
| L0 | detected, no annotation | yes |

Tabui emits `level` null even where the table says yes: the chemist chooses the level (`TPDEC`).

## Two scales, two labels
- Schymanski scale: label "L" + level, sublevel in lowercase ("L2a", "L3b"); in JS `LIVELLI.tag(lv)`, colour classes `.lv-2a`, `.lv-3a`, `.lv-3b`... (variables `--lv*` in `app.css`). In exports (CSV, JSON) the values stay without "L" (`2a`, `3b`).
- Prediction scale of Löffler et al., ES&T 59:19095 (2025): label "Prev. N" (grades 1-4), "Prev." column of the mzFinder products table (`.lvb.pd-N`).
- They are different scales and are never confused in the interface: the same digit with different labels ("L3a" versus "Prev. 3") is normal.
- The program proposes a level (in HR; in Tabui LR none) and shows the evidence; the decided level stays empty until the chemist decides. In Attributio each row carries the origin (`origin`: `auto`, `auto-2b` or `user`) and a decision by the chemist, who can change the automatic choice, is left in the log; the page's "Automatic 2b" checkbox turns on the `auto_2b` option (`docs/agenti/attributio.md`).
- The mzFinder HR products table (`TPTAB`, Flow) shows the proposed level 5/4 without sublevel as "L5/L4" (`TPTAB.level`, "Lvl" column, quality goes from diagnostic evidence to L4, L5; a matched MS2 does not give L3, it is stated as "Prev. N") and the prediction as "Prev. N" (`TPTAB.pred`); the export headers are "Level (Schymanski)" and "Prediction (Löffler)".
- Test `tests/test_livelli_schymanski.py`: one fixed fixture per evidence class of the 2014 paper (standard, library, weak match, analogue, unique formula, exact mass, unit resolution) must give the level and name its evidence; 2b only through the chemist's decision.
- The decoy q-value of the `spectra.match` step (`decoys=spectrum`, `docs/agenti/rust-flow.md`) enters as `q` of the candidate and `LIVELLI.assess` writes it as the last line of the reason ("decoy q 0.03 (lower bound)"): an evidence line, the level does not change.

## Score terms (`score_terms`)
- Every score opens as a weighted sum of named terms: record `{name, value, terms:[{name, value, weight}], formula_ref}` of `mzlab-core::score_terms` (`ScoreTerms`, `sum`, `mean`, `over_pairs`); `value = Σ weight · value` within 1e-12 (`residual()`), `formula_ref` is the stable key of the formula (`score/entropy-similarity`, `score/cosine`, `score/modified-cosine`, `score/composition`, `score/fragmenter`, `score/rt-residual`) and the formula text lives in the Theory, by Federico.
- The record is written next to the value only with the `score_terms` option (off: outputs are identical).
- Already giving their terms, without weights: `composition::compose` (`score{mass, iso, frag, total}`, weighted mean of `isoscore::combined`), `formula_id/1` (`terms{mass, isotopes}`), the RT-logP check (`resid`, `window`). Not giving them: library matching (entropy, cosine, modified cosine), the in silico score, the RT-logP check as a sum.
- With the record: `spectra.match` (`score_terms`: one term per pair, the 6 strongest with the peak m/z and the rest in "n other pairs"), `formula_id/1` (`score_terms` in the options: mass and isotopes with the `compose` weights, 1 and 2), `spectra.insilico` (`score_terms`: share of each explained peak), `rt.predict` (setting `rt_logp_terms`: observed, intercept and one term per descriptor with the negated coefficient). Proofs: `mzlab-lib/tests/score_terms.rs`, `mzlab-cli` `flow_ident_tests`, `tests/untargeted_formula_id.rs`, `score_terms_insilico.rs`, `score_terms_rt.rs`.

- The `export.annotations` step (schema `docs/schemi/annotations-1.json`) writes a CSV with, for each feature, the candidates of every method (standard, library, in silico, suspect, formula) side by side, each with its own `score_name`, and `level`/`level_why` of `annot/1` repeated on every row: it neither assigns nor raises any level; with `min_level` it writes only the features at that level or better.

## Flow: level 1 from standards and suspect fragments (`annot.level@1`)
- The `annot.level` step gives L1 only with a user `standards` file (CSV: `name`, `inchikey`, `rt` in min, optional `mz` and `adduct`; `,`, `;` or tab): the best hit of `candidates/1` whose first InChIKey block is that of a standard counts, with the `standard` gates of `livelli-1.json` (RT, score, ppm), checked before the library as in `livelli.js`. The score is `ent` (entropy); a hit with cosine only uses the same threshold on `cos` and the gate is called `stdscore_cos`.
- The query RT comes from the MGF of the optional `spectra` port (`SPECTRUMID`, `ACCESSION`, `FEATURE_ID` or `TITLE`; mean of the scans): without it, the row carries `standard.ran=false` and is never L1 (no silent pass).
- The Flow thresholds are parameters that take the place of the values of the `mzlab` profile (which does not change): `std_rt_max` 0.2 min, `std_score_min` 0.7, `std_ppm_max` 5, `suspect_fragments_min` 2.
- The `candidates` port accepts several files: a `screen/1` from `screen.suspects` (recognised by `format`) enters here. The join is exact on the id: `query` of `candidates/1` (the `SPECTRUMID` of `spectra.ms2` with the wired table, or the `ACCESSION` of `table.match`) is the row id of the table, equal to `feature` of `screen/1`; with `spectra.ms2` without a table the ids are titles and never join (no approximate join).
- Suspect fragments: without a library hit at 2a or a standard, a `screen/1` candidate with at least `suspect_fragments_min` fragments found gives `3a` (`why` `suspect_fragments`, with name and m/z); never above 3, never in profile `lr`; a mass-only hit does not change the level. Cases shared by page and Rust in `tests/livelli_casi.json` (field `thresholds`).

## Measurements of the MS2 gates (`crates/mzlab-cli/src/flow_level_ms2_tests.rs`)
- MEASURE livelli_ms2_piccoli (50 compounds m/z < 200, 3 true fragments against a library entry with 3 strong peaks and 5 at 1-2 %, library of 300 entries, `spectral_entropy`, thresholds of the `mzlab` profile): median entropy 0.90; with 1 scan per feature all 50 stay at 3a (failing gates: `scans` 50, `shared` 50); with 3 files (3 scans) all 150 stay at 3a and only `shared` fails (3 shared peaks against `picchi_min` 6: `nlib` counts the 5 weak peaks, so the reduced threshold of 3 does not trigger).
- MEASURE livelli_ms2_chimerici (50 co-isolated compounds, isobaric interferent at 40 %, purity 0.60): median entropy 0.75 against 0.999 for the same pure targets (drop 0.25). With 1 scan 3a for all (`scans` 50, `score` 27); with 3 scans 2a for 70 of 150 and 3a for 80 (only `score` fails); the pure targets at 3 scans are all 2a.
- MEASURE livelli_ms2_esche (`decoys=spectrum`): median and maximum q 0 for the best hits of the small and chimeric sets: decoys do not separate chimeric hits from clean ones (lower bound, `lib.fdr.note`). `scansioni_min` 2 counts scans with the same best hit: a compound seen in one file with one scan never reaches 2a.
- The JS cascade (`mzlab/web/livelli-1.js`) reads the same data and the same thresholds; parity stays in `tests/livelli_casi.json`.
