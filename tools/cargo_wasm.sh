#!/usr/bin/env bash
# `cargo` for the .wasm of the SITE: tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p <crate> ...
# Adds `-Zlocation-detail=none` (no file:line in the panic payload of the .wasm, ~40-80 KB per module; the message stays for the crash hook) on the pinned stable rustc through RUSTC_BOOTSTRAP.
# The variable lives only here: .cargo/config.toml and every native build (CLI, desktop, tests) never see it (tests/test_guardie_regole.py).
set -eu
RUSTC_BOOTSTRAP=1 exec cargo --config 'target.wasm32-unknown-unknown.rustflags=["-Zlocation-detail=none"]' "$@"
