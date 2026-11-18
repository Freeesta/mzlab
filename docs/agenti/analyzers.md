# Animated analysers of the startup veil (`#loading`)

- **Analysers off by default**: during loading only the base animation appears (`base` in `ldIon`: three ions 120° out of phase whose trails advance and interweave, dot with trail). The analyser screens stay in the code but are not included in the page, are not downloaded and have no entry in the interface. To re-enable them: `LD_ANALYSERS = true` in `avvio.js` (it loads them on demand from `LD_AN` and picks one at random).
- **Loading text**: while files are opening only the rotating phrases appear (`loading(true)` with no message). Hook by phase and %: `loadingStato(testo, pct)` in `shell.js` (`pct` null = indeterminate bar), `loadingStato(null)` returns to the phrases.
- Each analyser is in an `an-*.js` file that registers `window.LDAN.<name>(ctx)` → `{n, dt, T?, at(j, t), draw?(t), mid?}` (`an-util.js`: helpers). A new file is added to the name in `LD_AN` (`avvio.js`) and with a line here; the pytest tests find it on their own.
- `quad`: 4 rods, RF/DC, Mathieu secular oscillation.
- `paul`: 3D Paul trap, ring r²=r0²+2z² and end caps z²=z0²+r²/2 (z0=r0/√2), ω_z=2ω_r, micromotion at RF frequency; RF waveform in a box.
- `tof`: reflectron, same energy, same depth in the mirror, time ∝ √(m/z).
- `settore`: magnetic sector, radius r=√(2mU/q)/B: ions of different mass curve on different radii and only one passes the collector slit.
- `orbi`: spindle and barrel from the exact equipotentials of the quadro-logarithmic potential with the Lambert W (`radiusAt`/`zAt`); ω_z ∝ 1/√m, ω_φ and ω_r of Makarov; test in `tests/test_analizzatori.py`.
- `ims`: drift-tube ion mobility, v=K·E with K ∝ 1/CCS: more compact ions arrive first, collisions with the buffer gas.
- `esi`: Taylor cone, shrinking droplets.
- `maldi`: MALDI source with linear TOF: laser pulse, plume, acceleration at equal energy, flight time ∝ √m.
- `trappola`: linear ion trap. `fticr`: FT-ICR cell, ω_c ∝ z/m. `qqq`: triple quadrupole (Q1, q2 with gas, Q3).
- `ldIon(on, mode)` in `avvio.js` draws a dot with trail; with `LD_ANALYSERS` off it uses only `base`; `q.s` scales the size of an ion (droplets, fragments).
- Orbitrap: r1=6, r2=15, Rm=17 (arbitrary units). With Rm² = (r2²−r1²)/ln(r2²/r1²) the two electrodes would have zero voltage (`rmEqualPotential`): a larger Rm is needed; with Rm > r2 the outer barrel closes towards the tips, with Rm < r2 it opens (branch W₋₁).
