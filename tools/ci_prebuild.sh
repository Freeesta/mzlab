#!/usr/bin/env bash
# Builds once, in one sequence, the cargo artifacts that the pytest tests ask for with the same commands (same -p set, features and target, so the
# fingerprints match). Used by the pytest jobs of test.yml: the tests then find everything up to date instead of queueing on the cargo build lock.
# A failing command only prints a warning: the test that needs the artifact reports its own failure or skip.
cd "$(dirname "$0")/.." || exit 1
t0=$SECONDS
step() {
  local s=$SECONDS
  echo "::group::cargo $*"
  cargo "$@" || echo "::warning::prebuild failed: cargo $*"
  echo "::endgroup::"
  echo "prebuild $((SECONDS - s)) s: cargo $*"
}
W=wasm32-unknown-unknown
# native examples driven by tools/valida/ and the parity tests
step build --release --locked -q -p mzlab-feat-wasm --example nativo
step build --release --locked -q -p mzlab-feat-wasm --example nativo_mrm
step build --release --locked -q -p mzlab-feat-wasm --features untargeted --example untargeted
step build --release --locked -q -p mzlab-feat-table-wasm --example tabella
step build --release --locked -q -p mzlab-feat --example anteprima
step build --release -p mzlab-cli --example flow --bin mzlab-cli
step build --release --locked -q -p mzlab-core --example mzidx
step build --release --locked -q -p mzlab-lib --example fdr_parity
step build --release --locked -q -p mzlab-lib --example valida
step build --release --locked -q -p mzlab-lib --example build
step test --release -p mzlab-feat --features interp --test posizioni_riferimento --test interp_dati --no-run
step test -p mzlab-cli --lib flow_json --no-run
# .wasm of the tests (wasm-bindgen of the pipeline contract, parity with the CLI)
step build --release --locked --target $W -p mzlab-feat-wasm
step build --release --locked --target $W -p mzlab-feat-wasm -p mzlab-wasm
step build --release --locked --target $W -p mzlab-feat-import-wasm
step build --release --locked --target $W -p mzlab-feat-grafo-wasm
step build --release --locked --target $W -p mzlab-feat-table-wasm
step build --release --locked --target $W -p mzlab-msngrafo-wasm
step build --release --locked --target $W -p mzlab-libro-wasm
step build --release --locked --target $W -p mzlab-lib-wasm
echo "prebuild total $((SECONDS - t0)) s"
exit 0
