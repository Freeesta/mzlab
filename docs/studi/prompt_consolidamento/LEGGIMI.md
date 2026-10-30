# Consolidation prompts (study `docs/studi/DUPLICAZIONI_CODICE.md`)

Each file is a prompt ready for a DEV chat: copy everything after the «---» line. Order = rank of the study's top 10. Launch at most one prompt per zone at a time; a prompt waits for the PRs named in its «Gate» line.

| Prompt | Zone | Gate | Size effect (est.) |
|---|---|---|---|
| C1 | target screening (`liste*.js`, `mzlab-target`) | none for step 1; step 2 after #2218 | step 1: 0 start-up; step 2: −12.5 K lazy JS |
| C2 | wasm bridge glue, desktop protocol | none | ~0 |
| C3 | HR isotope pattern (`hr.js`, `isoscore.rs`, `mzlab-wasm`) | #2207 (`explore.js`, `lr.js`) | start-up −2…3 K, engine +1…3 K |
| C4 | sorts in `mzlab-lib` and `flow_screen_suspects` | #2218 | lib −12…60 K, table −10…50 K |
| C5 | JS chemistry tables (`tables.js`, `pfas-calc.js`, `draw.js`, `fair.js`) | #2220 (`picchi.js`) | start-up −3…5 K |
| C6 | mass constants in Rust | #2209 (`mzlab-core`), #2218 | ~0, goldens move |
| C7 | Rust numeric helpers (median, interp, FWHM, area) | #2209 | feat/core −5…30 K |
| C8 | Rust chemistry twins in `mzlab-feat` | #2209 | feat −3…8 K |
| C9 | JS signal code (SG, SNIP, integration) | #2220 (`picchi.js`) | start-up −2…4 K |
| C10 | JS UI helpers (escape, format, nearest, palette, CSV) | #2219, #2220, #2207 | start-up −2…3 K |
