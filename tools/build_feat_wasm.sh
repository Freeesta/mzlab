#!/usr/bin/env bash
# The .wasm of mzFinder (crates/mzlab-feat-wasm: detect, proofs, untargeted (the pipeline after mzmine 3.9.0, loaded only when the admin mode picks the mzmine-like engine in the panel Pipeline), and finder, loaded by the worker of mzFinder for classify, formula_info, identity_graph, polarity_pairs and the Dig of a high-resolution series, and finder_msn, loaded only when a file of the series is MSn (tree, library, localisation), and normative, the import of an official regulatory file, and attributio, the table of levels of a campaign folder in administrator mode; fetched only when a series is analysed, never committed), each under 500 KB after wasm-opt,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static> [all|a|b|manifest]
# The optional part lets CI build the modules in two parallel jobs (a, b) and write the manifest afterwards (manifest, once both outputs are in place); no part = all, what pages.yml and release.yml run.
set -eu
part=${2:-all}
want() { [ "$part" = all ] || [ "$part" = "$1" ]; }
# What each module is built from (package, features, wasm-opt set, cap after wasm-opt) is in tools/moduli/<name>.toml, read by tools/moduli.py: the cap rule is in docs/agenti/wasm-tetti.md.
static=${1:-site/static}
out=$static/wasm-feat
R=target/wasm32-unknown-unknown/release
# the wasm-bindgen + wasm-opt of each module run in the background while cargo builds the next one (tools/wasm_paralleli.sh); wasm_wait below joins them
. tools/wasm_paralleli.sh
# one cargo build per module of the part; mzlab-feat-wasm is built several times with different features into the same cargo output, wasm_pack copies it at once
build_parte() {
  local n riga uscita pacchetto flags tetto opt
  for n in $(python3 tools/moduli.py --elenco "$1"); do
    riga=$(python3 tools/moduli.py --riga "$n")
    IFS='|' read -r uscita pacchetto flags tetto opt <<<"$riga"
    # shellcheck disable=SC2086
    tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p "$pacchetto" $flags </dev/null
    wasm_pack "$static/$uscita" "$R/${pacchetto//-/_}.wasm" "$tetto" "$n" "$opt"
  done
}
if want a; then build_parte a; fi
if want b; then
  build_parte b
  # attributio-all: the table of classes beside the fused .wasm (the manifest names it)
  cp crates/mzlab-feat/data/classi-1.json "$out/attributio-all/classi-1.json"
  # the readers module (crates/mzlab-lettori-wasm), loaded only when a file needs it: tools/build_lettori_wasm.sh (it has its own jobs: it overlaps with the ones above)
  # part b leaves the readers to the job of the engine in sito-rust.yml (it balances the three jobs); all (pages, release) builds them here
  [ "$part" = b ] || tools/build_lettori_wasm.sh "$static"
fi
wasm_wait
[ "$part" = all ] || [ "$part" = manifest ] || exit 0
python3 tools/moduli.py --manifest "$out"
