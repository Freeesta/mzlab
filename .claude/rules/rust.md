---
paths:
  - "crates/**"
  - "tools/build_*_wasm.sh"
---
# Rust engine rules
- New modules get a CLI (`--set INSTANCE.NAME=VALUE` per step parameter; checklist in `docs/agenti/cli.md`) and a wasm export; the byte-for-byte parity test covers site, app and CLI.
- **Wasm caps**: a `.wasm` may grow by up to +10 % per new function; record before/after in `docs/agenti/wasm-tetti-misure.md`. Never raise a cap by editing a workflow or build script.
- `unsafe` only in small isolated modules with tests; ported code cites its source and has a parity test; GPL/AGPL code is only an idea, never copied.
- A damaged unit (block, scan, channel) is dropped, counted and reported, never silent; a sample is refused only when nothing usable is left.
