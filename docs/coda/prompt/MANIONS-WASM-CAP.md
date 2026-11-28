Read docs/coda/BRIEF.md.

# MANIONS-WASM-CAP: size and cap of the `manions` wasm module

**Goal.** Record the measured size and the cap of the lazy `manions` module (`mzlab-manions-wasm`, `peak table --ions` on the site) once the CI size step has measured it after `wasm-opt`.

**Why.** No `wasm-opt` was available in the DEV chat, so `tools/moduli/manions.toml` holds a placeholder `tetto` and `misura = 0`. The raw module is 704,490 B.

**Read first.** `docs/agenti/wasm-caps.md` (rules and the `manions` line), `tools/moduli/manions.toml`, `python3 tools/moduli.py --verify`.

**Done means.** 1. `misura` is the CI measurement and `tetto` is the measurement x 1.05 rounded up to 4 KiB, as for the other new modules. 2. The `manions` line of `docs/agenti/wasm-caps.md` says the size. 3. `python3 tools/moduli.py --verify` and `python3 tools/controlla.py` pass. 4. If the measurement is far above the raw-size expectation, say so and propose splitting the module.

**Out of scope.** Changing the module code; raising any other cap.

**Follow-ups.** Closing step of the brief.
