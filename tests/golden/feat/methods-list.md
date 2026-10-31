# Methods

Data were processed with mzlab 0.9.1 (commit 0123456789abcdef).

- `features.detect@1` (instance `det`): algorithm centwave@1; preset hr; polarity 1; axis ppm; floor 20000 (estimated from the data); min_height 200000; min_width 0.05; min_snr 3; min_points 5
- `features.align@1` (instance `ali`): mode family; ppm 5; rt_gap 0.1; rt_warp off
- `vendor.custom@2` (instance `x`): depth 1 to 4; strict on
- `spectra.match@1` (instance `m`): algorithm modified_cosine@1; tol_da 0.02; did not complete

Plan SHA-256: `aa11bb22`.
