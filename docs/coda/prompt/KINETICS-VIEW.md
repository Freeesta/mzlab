Read docs/coda/BRIEF.md.

# KINETICS-VIEW: The decay plot of `quant.kinetics`

**Goal.** A view of the `kinetics-series/1` document: per ion and series the normalised points against time, the first-order curve with its 95 % band, k and half-life in the title, the UV factor of each file as a small strip, and the flagged points (`uv_off`, skipped, forced zero) drawn differently with their reason on hover.

**Why.** `quant.kinetics` prints the numbers and the choices; the owner judges a fit by looking at it. `quant.curve` has the same gap (calibration.md, "Not yet").

**Read first.** `docs/agenti/kinetics-series.md`, `docs/agenti/calibration.md` ("Not yet"), `TP_Mine/lazy/groups-view.js` (the conditions chart to reuse), `docs/agenti/value-states.md`, `docs/agenti/flow-web.md` (the Flow page).

**Already on main (checked 28 Nov, re-check before coding).** `quant.kinetics@1` writes `kinetics-series/1` with `files`, `points`, `fits`, `choices` and `warnings`; no view draws it.

**Done means.** 1. The view reads only the document (no new numbers computed in JS: the curve points come from `A`, `k` and the interval of `fits`; if the band needs more, add it to the Rust text with a parity test). 2. Absent and skipped points are visible, never dropped silently. 3. An e2e test on the committed series of `crates/mzlab-cli/tests/data/kinetics/`.

**Out of scope.** A new fit model; second-order and formation-decay (`KINETICS-FORMATION`).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
