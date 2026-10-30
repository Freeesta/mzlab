# Methods

Data were processed with mzlab 0.9.1 (commit 0123456789abcdef).

Features were detected with the centWave algorithm (polarity 1, mass axis ppm, intensity floor 20000 (estimated from the data), minimum height 200000, minimum width 0.05, minimum S/N 3, minimum points 5). (`features.detect`, step version 1)

Features were aligned across the files in family mode (m/z tolerance 5 ppm, RT gap 0.1 min, RT warping off). (`features.align`, step version 1)

The step vendor.custom was run with depth 1 to 4; strict on. (`vendor.custom`, step version 2)

Spectra were matched against the library with the modified_cosine score (fragment tolerance 0.02 Da). This step did not complete. (`spectra.match`, step version 1)

Plan SHA-256: `aa11bb22`.
