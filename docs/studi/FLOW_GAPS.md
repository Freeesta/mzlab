# mzFlow gaps: decided, not done

Open gaps only; everything not listed here is implemented. Each item: stage, size (S/M/L), target module.
Cap rule for all of them: never raise a `.wasm` cap; the `hr` and `table` modules are frozen (+0 B), growth goes into lazy modules (`flowtab`, `flowstat`, `untargeted`, `attributio-all`, `formula`).

## Open
| gap | stage | size | target | where it stands |
|---|---|---|---|---|
| PQN normalisation | 9 normalisation | M | `flowtab` (`crates/mzlab-feat/src/norm.rs`) | no `Pqn` in `norm.rs` |
| `annot.hazard` executor | 12 annotation | S | `attributio-all` | listed in `HOLES` (`flow_rule_tests.rs`) and `SOLO_APP`: "descriptor only" |
| `arpls` baseline and `LocalMax` mass detector as `features.detect@2` params | 1-4 detection | S-M | `untargeted` | `steps/detect.rs` exposes only `smooth_window` |
| Blank subtraction with `Matching::Batch` / `Matching::Matrix` | 8 blank filtering | M | `flowtab` (`filters_blank.rs`) | refused unless matching is `All` |
| `export.fbmn_bundle` | 15 export | S-M | export module | step absent |
| `formula.ms1` on cohorts above 256 MiB in the browser | 11 formulas | M-L | lazy `formula` | `MAX_MZML` in `flow/lazy/flow-web-ident.js`; needs range-sliced `.mzidx` reads |
| Browser executors for `spectra.chimeric` and `meta.compare` | 12 / 15 | M each | lazy modules | still in `SOLO_APP` (`tests/test_parita_passi.py`) |

## Rejected
- Merging adduct or in-source fragment rows silently: rows stay separate in `ionnet/1` families so MS2 and blank checks stay auditable.
- p-values with fewer than 3 replicates: the statistics steps refuse, the user sees the replicate count.
- Automatic level 2b: the level is chosen by the user from the shown evidence, never assigned by the program.
- Raising a `.wasm` cap, or fusing `formula` into `hr` / `untargeted`: caps are hard contracts (`docs/agenti/wasm-tetti.md`).

## MZmine comparison, conclusions
- Detection, ADAP/ADAP-like resolution, alignment and gap filling are at parity in method; the byte-for-byte parity across site, app and CLI is the advantage.
- Unique to mzFlow: non-destructive ion families, provenance records for every run, in-browser operation with files kept local.
- Missing relative to MZmine: PQN, batch-wise blank handling, FBMN export bundle, exposed baseline and local-maximum mass detector options.
- Everything else compared is either implemented or rejected above.

## Next step
PQN normalisation and the `annot.hazard` executor first.
