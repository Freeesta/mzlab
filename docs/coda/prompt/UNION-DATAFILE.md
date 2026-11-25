Read docs/coda/BRIEF.md.

# UNION-DATAFILE: features.union reads aligned tables with datafile: states

**Goal.** `features.union` and `quant union` accept aligned tables that carry `datafile:` state columns.

**Why.** Which states count as a row of a series is a choice that needs its own log (follow-up of #3710).

**Read first.** `docs/agenti/quant-targets.md`, `crates/mzlab-core/src/series_union/` (a test already uses a `datafile:a:area` header), `crates/mzlab-feat/src/union.rs`. Start with `grep -n 'quant targets\|union' docs/agenti/INDICE.md`, then `grep -rn 'datafile' crates/mzlab-core/src/series_union --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** `features.union@1` and `quant union` read series CSVs of `features` (merged).

**Done means.** 1. A parameter naming the accepted states (default detected and filled), logged in `union/1`. 2. Synthetic aligned table with forced and absent cells. Parity site=app=CLI; `flowtab` and `rotte` sizes per the brief.

**Out of scope.** A page node.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
