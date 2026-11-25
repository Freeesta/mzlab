Read docs/coda/BRIEF.md.

# SUM-FILL-TIME: ions of a summed spectrum without `--fill-ms`

**Goal.** `spectrum get --sum` reports `ions` (cps x fill time) and `fill_time.source` `file` when the file states a fill time per scan, so an agent no longer has to pass `--fill-ms`.

**Why.** Today `evidence.sum.ions` is `null` unless the user states `--fill-ms`, because the `.mzidx` has no fill-time column (`idxread::Scan`). mzML files from Thermo carry it per scan (`MS:1000927`, ion injection time); the agent's question "is this MS2 spectrum credible: 24 or 490 ions?" is then answered by one command.

**Read first.** `crates/mzlab-core/src/specsum.rs` (the route; `fill_time` and `ions` members), its tests `crates/mzlab-core/src/specsum_tests.rs`, `crates/mzlab-core/src/idxread.rs` (`Scan`), the index writer (`grep -n 'struct Scan' crates/mzlab-core/src/*.rs`), the mzML reader for `MS:1000927`. Start with `grep -n 'spectrum\|raw-access' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** `/api/specsum` answers the same bytes in the site (`rawxic` `spec_sum`), the app and the CLI; `fill_time.note` is `note.spec.sum.noFill` and `--fill-ms T` states a fill time for every scan.

**Done means.** A per-scan fill time read from the file (index column, version bump, old indexes still read and rebuilt on demand), used by `specsum.rs` per scan so `by_scan_type[].ions` is the sum of counts x time per scan; `--fill-ms` still overrides and says so (`source: user`); a file without it keeps `ions: null` with the note. Parity CLI = app = wasm (test `tests/test_spec_sum_wasm.py`); synthetic file written with `MzmlWriter` carrying the term. The `rawxic` growth is stated in the PR body (raw bytes before, after, %).

**Out of scope.** Readers of vendor formats that do not expose the fill time; background subtraction (MS2-PEAK-AUTO).

**Follow-ups.** Closing step of the brief.
