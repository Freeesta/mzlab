Read docs/coda/BRIEF.md.

# SUM-CHANNEL: `spectrum get --sum --id CHANNEL`

**Goal.** `--sum` accepts `--id` (an `ms:<key>` or `dd:<level>` channel): the sum, its scan count, counts, ions and scan types are those of the channel's scans, the same as its mean spectrum.

**Why.** Today `--sum` with `--id` is refused (exit 2, `--sum is not for --id`). The channel filter (`select_scans`, `canali()`, the MSn tree) added about 65 KB raw to the lazy `rawxic` module in a trial (module +19 % against the 5 % rule), so `/api/specsum` selects scans by window, level and precursor only.

**Read first.** `crates/mzlab-core/src/specsum.rs` (`pick`), `crates/mzlab-core/src/channels/routes.rs` (`channel_filter`), `docs/agenti/wasm-caps.md`, `tools/moduli/rawxic.toml`. Start with `grep -n 'channels\|wasm' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** `--sum` without `--id` in the site, the app and the CLI; `by_scan_type` already separates the scan types of a window, which is the usual way to read one channel.

**Done means.** One of: the engine resolves the channel to a scan mask and the route takes it (`scans=` ranges), so `rawxic` does not link the channel code; or a new lazy module holds the channel filter. Parity CLI = app = wasm byte for byte; the refusal test of `spectrum_sum_tests.rs` becomes the agreement test (a channel sums the same scans as its spectrum, at both levels). Raw bytes of every module touched, before, after and %, in the PR body.

**Out of scope.** The `msn:` tree filters.

**Follow-ups.** Closing step of the brief.
