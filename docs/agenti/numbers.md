# Numbers (`mzlab/web/numeri.js`)
Loaded before `grafici.js`; used by `picchi.js`, `liste.js`, `banco.js`, `conversione.js`. Tests: `tests/test_perf_web10.py`, `e2e_spettro_grande`.

- `maxOf(a, init)`, `minOf(a, init)`, `maxAbs(a, init)`: extremes of an array (typed arrays too) without spread; a NaN gives NaN like `Math.max`; `init` is the value for an empty array. `Math.max(...a)` on ~125 000 elements throws RangeError: use these helpers instead.
- `lowerIdx(a, v)`: first index with `a[i] >= v` in an ascending array.
- `spanOf(a, lo, hi)`: the interval `[i0, i1)` that contains every index with `lo <= a[i] <= hi`: binary search if `a` is ascending (checked once per array, in a WeakMap), otherwise the whole array. Calling loops keep their own exact tests, so there is a single path and the pixels do not change.
- `b64F32(str, n)`: the float32 values of a base64 payload of the map routes (`getMap` in `grafici.js`); a payload that is not base64, whose length is not a multiple of 4, or with a number of values different from `n` (nrt×nmz) throws a clear `Error` instead of a RangeError.
