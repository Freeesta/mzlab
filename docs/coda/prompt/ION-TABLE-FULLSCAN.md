Read docs/coda/BRIEF.md.

# ION-TABLE-FULLSCAN: `peak table FILE... --ions` for full-scan files

**Goal.** Let `peak table FILE... --ions` read a full-scan file (mzML, `.raw`, `.wiff`) directly: the scan type (EMS, EPI, Q1, other) and the method values come from the scans and `method/1` of the file, as `raw xic --ions` does.

**Why.** `peak table --ions` (PR of ION-COUNTS-PEAK-TABLE) takes the SIM/MRM channels of a file as scan type `sim` unless `--scan-type` names one. A full-scan file has to go through `trace xic --ions` and then `peak table --traces`, which carries the factor of each trace. One call per file is the agent-friendly route.

**Read first.** `crates/mzlab-cli/src/manual_table_ions.rs` (`of_file`, `of_traces`), `crates/mzlab-core/src/manint_ions.rs`, `crates/mzlab-core/src/rawxic_trace.rs` (`ion_scale_of`), `docs/agenti/raw-access.md` (the `--ions` lines).

**Done means.** 1. `peak table FILE... --ions` on a full-scan file gives the same factors as `trace xic --ions` then `--traces`, byte for byte. 2. Synthetic files of known factor for ems, epi, q1 and other. 3. Parity site (.wasm) = app = CLI, the wasm leg failing, not skipping, in CI. 4. Catalogs and `related.rs` words updated. No `rotte` growth.

**Out of scope.** More method labels than `ionfacts` knows (ION-COUNTS-LABELS).

**Follow-ups.** Closing step of the brief.
