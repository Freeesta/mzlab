#!/usr/bin/env bash
# The .wasm of mzFinder (crates/mzlab-feat-wasm: detect, proofs, untargeted (the pipeline after mzmine 3.9.0, loaded only when the admin mode picks the mzmine-like engine in the panel Pipeline), mrm, loaded only for a series of MRM files, and finder, loaded by the worker of mzFinder for classify, formula_info, identity_graph, polarity_pairs and the Dig of a high-resolution series, and finder_msn, loaded only when a file of the series is MSn (tree, library, localisation), and attributio, the table of levels of a campaign folder in administrator mode; fetched only when a series is analysed, never committed), each under 500 KB after wasm-opt -Oz,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static>
set -eu
# Limit of every other .wasm after wasm-opt -Oz, bytes (the rule of the project: it does not go up; measured: detect 418400, proofs 450711, finder 436274, attributio 432347, finder_msn 296579, mrm 226834, grafo 199697).
limit=512000
limit_rev=942080   # attributio-rev carries the table of levels AND the review of one feature (libraries, spectra, formulas, isotopes); measured 912955 (+3 %)
limit_export=659456   # attributio-export carries the cascade AND the four spreadsheet exports (Schymanski folder, publication table, quantification, gradient); measured 636757 (+3 %)
limit_lr=741376   # finder-lr carries the structure graph (chem) for the supposed tree and the MS2 matching; measured 717874 (+3 %)
limit_chem=569344   # chem: ETKDG geometry and GFN2-xTB of the parent; measured 551415 (+3 %)
out=${1:-site/static}/wasm-feat
R=target/wasm32-unknown-unknown/release
# the wasm-bindgen + wasm-opt of each module run in the background while cargo builds the next one (tools/wasm_paralleli.sh); wasm_wait below joins them
. tools/wasm_paralleli.sh
for which in detect proofs mrm finder finder_edge finder_msn untargeted; do
  cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features $which
  wasm_pack "$out/$([ $which = detect ] && echo feat || echo $which | tr _ -)" $R/mzlab_feat_wasm.wasm $limit "mzFinder $which"
done
# the third one, loaded only when a table is shown or exported (views, report, CSV, MGF, .ms): crates/mzlab-feat-table-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-table-wasm
wasm_pack "$out/table" $R/mzlab_feat_table_wasm.wasm $limit "mzFinder table"
# the structure graph, loaded only when the user gives a SMILES: crates/mzlab-feat-grafo-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-grafo-wasm
wasm_pack "$out/grafo" $R/mzlab_feat_grafo_wasm.wasm $limit "mzFinder grafo"
# the reactivity of the parent (crates/mzlab-chem-wasm: reatt/1), loaded only in administrator mode
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-chem-wasm
wasm_pack "$out/chem" $R/mzlab_chem_wasm.wasm $limit_chem "mzFinder chem"
# the readers of the tables of other programs (mzMine, FBMN/GNPS2, MS-DIAL), loaded only when the user opens one: crates/mzlab-feat-import-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-import-wasm
wasm_pack "$out/import" $R/mzlab_feat_import_wasm.wasm $limit "mzFinder import"
# the table of levels of Attributio (administrator mode), loaded only when a campaign folder is chosen: crates/mzlab-attributio-wasm
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-attributio-wasm
wasm_pack "$out/attributio" $R/mzlab_attributio_wasm.wasm $limit "mzFinder attributio"
# the Banco 2b of Attributio (review of one feature, saving of a decision), loaded only when the Banco opens: the same crate with the feature rev
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-attributio-wasm --features rev
wasm_pack "$out/attributio-rev" $R/mzlab_attributio_wasm.wasm $limit_rev "mzFinder attributio-rev"
# the exports of Attributio (Schymanski folder, publication table, quantification, gradient), loaded only when the Export menu is used: the same crate with the feature export
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-attributio-wasm --features export
wasm_pack "$out/attributio-export" $R/mzlab_attributio_wasm.wasm $limit_export "mzFinder attributio-export"
# the low-resolution Dig of mzFinder (crates/mzlab-finder-lr-wasm), loaded by the worker of mzFinder only for a series of full-scan unit-resolution files
cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-finder-lr-wasm
wasm_pack "$out/finder-lr" $R/mzlab_finder_lr_wasm.wasm $limit_lr "mzFinder finder-lr"
# the readers module (crates/mzlab-lettori-wasm), loaded only when a file needs it: tools/build_lettori_wasm.sh (it has its own jobs: it overlaps with the ones above)
tools/build_lettori_wasm.sh "${1:-site/static}"
wasm_wait
python3 - "$out" <<'P'
import hashlib, json, sys
o = sys.argv[1]
sha = lambda d: hashlib.sha256(open(f"{o}/{d}/mzlab_feat_wasm_bg.wasm", "rb").read()).hexdigest()
json.dump({"feat": {"js": "feat/mzlab_feat_wasm.js", "wasm": "feat/mzlab_feat_wasm_bg.wasm", "sha256": sha("feat")},
           "proofs": {"js": "proofs/mzlab_feat_wasm.js", "wasm": "proofs/mzlab_feat_wasm_bg.wasm", "sha256": sha("proofs")}},
          open(f"{o}/manifest.json", "w"), indent=1)
m = json.load(open(f"{o}/manifest.json"))
m["table"] = {"js": "table/mzlab_feat_table_wasm.js", "wasm": "table/mzlab_feat_table_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/table/mzlab_feat_table_wasm_bg.wasm", "rb").read()).hexdigest()}
m["mrm"] = {"js": "mrm/mzlab_feat_wasm.js", "wasm": "mrm/mzlab_feat_wasm_bg.wasm", "sha256": sha("mrm")}
m["finder"] = {"js": "finder/mzlab_feat_wasm.js", "wasm": "finder/mzlab_feat_wasm_bg.wasm", "sha256": sha("finder")}
m["finder_edge"] = {"js": "finder-edge/mzlab_feat_wasm.js", "wasm": "finder-edge/mzlab_feat_wasm_bg.wasm", "sha256": sha("finder-edge")}
m["finder_msn"] = {"js": "finder-msn/mzlab_feat_wasm.js", "wasm": "finder-msn/mzlab_feat_wasm_bg.wasm", "sha256": sha("finder-msn")}
m["untargeted"] = {"js": "untargeted/mzlab_feat_wasm.js", "wasm": "untargeted/mzlab_feat_wasm_bg.wasm", "sha256": sha("untargeted")}
m["grafo"] = {"js": "grafo/mzlab_feat_grafo_wasm.js", "wasm": "grafo/mzlab_feat_grafo_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/grafo/mzlab_feat_grafo_wasm_bg.wasm", "rb").read()).hexdigest()}
m["chem"] = {"js": "chem/mzlab_chem_wasm.js", "wasm": "chem/mzlab_chem_wasm_bg.wasm",
             "sha256": hashlib.sha256(open(f"{o}/chem/mzlab_chem_wasm_bg.wasm", "rb").read()).hexdigest()}
m["import"] = {"js": "import/mzlab_feat_import_wasm.js", "wasm": "import/mzlab_feat_import_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/import/mzlab_feat_import_wasm_bg.wasm", "rb").read()).hexdigest()}
m["attributio"] = {"js": "attributio/mzlab_attributio_wasm.js", "wasm": "attributio/mzlab_attributio_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/attributio/mzlab_attributio_wasm_bg.wasm", "rb").read()).hexdigest()}
m["attributio_rev"] = {"js": "attributio-rev/mzlab_attributio_wasm.js", "wasm": "attributio-rev/mzlab_attributio_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/attributio-rev/mzlab_attributio_wasm_bg.wasm", "rb").read()).hexdigest()}
m["attributio_export"] = {"js": "attributio-export/mzlab_attributio_wasm.js", "wasm": "attributio-export/mzlab_attributio_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/attributio-export/mzlab_attributio_wasm_bg.wasm", "rb").read()).hexdigest()}
m["finder_lr"] = {"js": "finder-lr/mzlab_finder_lr_wasm.js", "wasm": "finder-lr/mzlab_finder_lr_wasm_bg.wasm",
                 "sha256": hashlib.sha256(open(f"{o}/finder-lr/mzlab_finder_lr_wasm_bg.wasm", "rb").read()).hexdigest()}
json.dump(m, open(f"{o}/manifest.json", "w"), indent=1)
P
