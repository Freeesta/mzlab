# Speed measurements of the Thermo `.raw` reader

One row per speed change of the `.raw` reader (open, decode, reads of the readers module), oldest first; same columns and rules as `misure-velocita.md`, which holds the other measurements and takes no new rows. Abandoned routes go in `ottimizzazioni-tentate.md`.

| Date | PR | Step | Before | After | % |
|---|---|---|---|---|---|
| 7/11 | #2811 | Synthetic HR `.raw` (304 MB and 1.8 GB, 3000 peaks per scan): adaptive read window of `Stream`, `con_blocco` and the `BlockCache` blocks no longer zero-filled; scans phase of `build_mzidx_native` on the site (Node, `thermo` module from `cargo_wasm.sh`, 4 vCPU, N=10, median [min-max]); `main` `c2f2c7829eb47a2bf5fae6e24c34e9bcdd41f36c` | 304 MB: 431 ms [418-444], 671 MB read in 1695 calls; 1.8 GB: 2415 ms [2338-2619], 4.03 GB | 265 ms [255-284], 549 MB in 1229; 1394 ms [1351-1459], 3.29 GB | −38 %; −42 % |
| 7/11 | #2812 | Synthetic HR `.raw` profile (30 000 points per scan, chunks of 10 bins) decoded into m/z and intensity without the `ProfileChunk`s: native `scansione_profilo` of 1400 scans (304 MB, `cli` profile, 4 vCPU, N=10, median [min-max]); wasm `opt-level z` in Node, ms per scan, N=10; `main` `c2f2c7829eb47a2bf5fae6e24c34e9bcdd41f36c` | native 892 ms [873-987]; wasm 1.12 ms [1.07-1.24] | 355 ms [343-407]; 0.56 [0.55-0.59] | −60 %; −50 % |
