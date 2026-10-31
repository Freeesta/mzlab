# Mass calibration check from ubiquitous contaminants

Study behind the TIC menu entry «Check calibration (contaminants)…» (`mzlab/web/masscal.js`, route `api/masscal`, engine `mzlab_feat::massacc::check`). Numbers only: no verdict, the threshold only colours them.

## What is reused (nothing written twice)
- **List**: `mzlab/web/contaminanti.json`, 1272 ions (Keller 2008 background list: PEG/PPG/PTFE-like series, siloxanes, phthalates, plasticisers, solvent clusters, with `pol`). Read by `massacc::refs::background`; the exact m/z of a singly charged ion comes from the engine (`elements::ion_mz`), the listed one only where the adduct table has none.
- **Matcher**: `massacc::measure` (window `search_ppm` = 20, intensity floor, a listed background ion must stay in at least half of the MS1 scans of the file, so an analyte or a one-off peak is not taken for background).
- **Estimates**: `massacc::analyse` (Huber offset and scale; fits `linear_mz`, `linear_mz_rt` over the scans of each ion). The panel calls it with `model = linear_mz_rt`, so the slopes per 100 m/z and per minute come from the same fit that corrects a table in Flow.
- Browser: `mass_accuracy` of the `.wasm` `proofs` (no new export, no size change except the additive field `points[].scans` of `massacc/1`); desktop: op `masscal` (`rpc_masscal.rs`), the same call.

## Ideas weighed
| Idea | Verdict |
|---|---|
| Error against m/z (slope = error of the mass scale) | In: chart and `slope_ppm_per_100_mz`. |
| Error against RT (drift of the calibration along the run) | In: chart (apex RT of each ion) and `slope_ppm_per_min`. The per-ion chart is only informative when contaminants peak at different times; the scan-level slope does not depend on that. |
| Lock-mass style series (polysiloxane 445.12003, 519.13882, 593.15761; DEHP [M+H]+ 391.28429, [M+Na]+ 413.26623) | Already in the list; they are used like any other ion. A method that applies a lock mass is read from the index (`lock_mass` in the report), never applied. |
| Error against intensity (space charge) | Not in this version: needs the scan-level points in the report (today one point per ion). Follow-up: add `intensity` as a regressor to `fit.rs` and a third chart. |
| First minute against last minute | Covered by the RT slope. |
| Suggest a recalibration offset | Report only: the offset is the number in the summary line. Applying a ppm correction is the Flow step `qc.mass_accuracy` with `model` ≠ `none` (corrects a feature table, never the raw data). |

## Limits
- Positive or negative ions follow the polarity of the file (the larger MS1 group of a mixed file); a unit-resolution file is not checked (a ppm error means nothing on a 0.1 Da axis).
- A contaminant has to be in the file: a clean method with a very pure solvent can have fewer than three persistent ions, then the panel lists them and gives no offset.
- Overlapping peaks bend one ion; the Huber fit down-weights it, the table shows it. Mass accuracy of an Orbitrap is also intensity-dependent: set the lowest intensity (default 1e4 counts) for the instrument at hand.
- Native samples of a maker's file (the ones read without an mzML) are not checked from the panel yet.
