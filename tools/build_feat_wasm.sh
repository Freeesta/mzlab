#!/usr/bin/env bash
# The .wasm of mzFinder (crates/mzlab-feat-wasm: detect, proofs, untargeted (the pipeline after mzmine 3.9.0, loaded only when the admin mode picks the mzmine-like engine in the panel Pipeline), and finder, loaded by the worker of mzFinder for classify, formula_info, identity_graph, polarity_pairs and the Dig of a high-resolution series, and finder_msn, loaded only when a file of the series is MSn (tree, library, localisation), and normative, the import of an official regulatory file, and attributio, the table of levels of a campaign folder in administrator mode; fetched only when a series is analysed, never committed), each under 500 KB after wasm-opt,
# and site/static/wasm-feat/manifest.json with their SHA-256 (the worker checks it): tools/build_feat_wasm.sh <site/static> [all|a|b|manifest]
# The optional part lets CI build the modules in two parallel jobs (a, b) and write the manifest afterwards (manifest, once both outputs are in place); no part = all, what pages.yml and release.yml run.
set -eu
part=${2:-all}
want() { [ "$part" = all ] || [ "$part" = "$1" ]; }
# Limit of every other .wasm after wasm-opt, bytes (the rule of the project: it does not go up; measured: detect 418400, proofs 450711, finder 436274, attributio 432347, finder_msn 296579, grafo 199697).
limit=512000
limit_untargeted=585728   # untargeted: +2.4 % over 512000 for the broad-band mark and the blank rule (measured 512259), decision of the Coordinator
limit_table=737280   # table: tabcheck/1 with all fifteen checks (Kendrick analysis and PCA included); cap +20 % over 512000, decision of Federico, 22 Oct 2026 (docs/agenti/wasm-tetti.md)
limit_hr=1605632   # hr: detect, proofs, finder, finder_edge and finder_msn in ONE .wasm (measured 1384565 with flow_job_meta, the estimates of the record of a run in the browser; before 1302880); the only cap of the group (docs/agenti/wasm-tetti.md)
limit_attributio=1536000   # attributio fused: the table, the review (Banco 2b), the exports and the MS2 engine in ONE .wasm (measured 1395661 (+10 %)); the only cap of the group (docs/agenti/misure.md)
limit_lr=892928   # zlib-rs +30 KB (was 741376), mzlab-picchi at O3 +3.6 KB (peaks 3.1x); finder-lr carries the structure graph (chem) for the supposed tree and the MS2 matching; measured 717874 (+3 %)
limit_interp=623200   # interp (cap +10 % over the 566463 measured with tp.review, rounded up to 100 B): the interpretation of the rows (interp/1) with the MSn tree of the infusion; above 470000 the tree is built by finder_msn (docs/agenti/interp.md)
limit_chem=569344   # chem: ETKDG geometry and GFN2-xTB of the parent; measured 551415 (+3 %)
limit_msnregole=196608   # msnregole: SMILES -> the expectations of the structure rules (mzlab-feat chem::regole), loaded by rust-msngrafo.js only when the page asks for the rules with a SMILES; measured 163480 (+3 %)
limit_msngrafo=790528   # zlib-rs +30 KB (was 575000), mzlab-picchi at O3 +1.1 KB (peaks 3.1x); msngrafo: the fragmentation graph of an MSn infusion (msngrafo/1), loaded by the worker of the engine on the first request of the graph; measured 537129 (with the structure cleavages of mzlab-feat, feature chem only, and the table fragrules/1; the structure rules of the table are not in it: 539551)
limit_rotte=884736   # rotte: the statistics, molecules, campioni (Data) and Flow command/line/scheduler routes, out of the engine; loaded by the worker of the engine on the first request of one of them (docs/agenti/wasm-tetti.md)
limit_llm=397312   # llm: requests, normalized streams, recordings, the loop of the assistant (Agent), the context builder, the charts (grafico/1: views, validation, reduction), the readers of the data assistant (Assist: requests, lettura/1 and param-scelte/1 validation against the step registry) and the facts without a model (inspect_build, params_propose), loaded for the facts in the profile «ricerca» and for the assistant after the user turned it on and agreed; measured 358897 (x 1.10, multiple of 4 KiB); 305994 before the facts, the plan checker is the page's
out=${1:-site/static}/wasm-feat
R=target/wasm32-unknown-unknown/release
# the wasm-bindgen + wasm-opt of each module run in the background while cargo builds the next one (tools/wasm_paralleli.sh); wasm_wait below joins them
. tools/wasm_paralleli.sh
# HR group (detect, proofs, finder, finder_edge, finder_msn): ONE .wasm "hr" (the manifest keys of the five name it; the workers load it once).
if want a; then
  tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features detect,proofs,finder,finder_edge,finder_msn,flow_ms2
  wasm_pack "$out/hr" $R/mzlab_feat_wasm.wasm $limit_hr "mzFinder hr" @motore
fi
for which in untargeted interp; do
  want b || continue
  tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features $which
  wasm_pack "$out/$which" $R/mzlab_feat_wasm.wasm $(case $which in interp) echo $limit_interp;; *) echo $limit_untargeted;; esac) "mzFinder $which" @motore
done
if want a; then
# formula.ms1 of the Flow (flow_ms1_rows, flow_ms1_join), loaded only by its executor in flow-web-ident.js; a module of its own, with the standard limit
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features flow_formula
wasm_pack "$out/formula" $R/mzlab_feat_wasm.wasm $limit "mzFinder formula" @motore
# table.drift of the Flow (flow_drift), loaded only by its executor in flow-web-drift.js; the home of the table steps that no longer fit `table`, with the standard limit
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features flow_tab
wasm_pack "$out/flowtab" $R/mzlab_feat_wasm.wasm $limit "mzFinder flowtab" @motore
# the third one, loaded only when a table is shown or exported (views, report, CSV, MGF, .ms): crates/mzlab-feat-table-wasm
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-table-wasm
wasm_pack "$out/table" $R/mzlab_feat_table_wasm.wasm $limit_table "mzFinder table" @motore
# the structure graph, loaded only when the user gives a SMILES: crates/mzlab-feat-grafo-wasm
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-grafo-wasm
wasm_pack "$out/grafo" $R/mzlab_feat_grafo_wasm.wasm $limit "mzFinder grafo" @motore
# the reactivity of the parent (crates/mzlab-chem-wasm: reatt/1), loaded only in administrator mode
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-chem-wasm
wasm_pack "$out/chem" $R/mzlab_chem_wasm.wasm $limit_chem "mzFinder chem" @motore
# the fragmentation graph of an MSn infusion (crates/mzlab-msngrafo-wasm: msngrafo/1), loaded by the worker of the engine on the first request of /api/msn_grafo
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-msngrafo-wasm
wasm_pack "$out/msngrafo" $R/mzlab_msngrafo_wasm.wasm $limit_msngrafo "msngrafo" @motore
# the structure rules of that graph (crates/mzlab-msnregole-wasm: SMILES -> attese), loaded by rust-msngrafo.js only when the page asks for the rules with a SMILES
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-msnregole-wasm
wasm_pack "$out/msnregole" $R/mzlab_msnregole_wasm.wasm $limit_msnregole "msnregole" @motore
# the statistics, molecules, Data and Flow routes (crates/mzlab-rotte-wasm), loaded by the worker of the engine on the first request of one of them
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-rotte-wasm
wasm_pack "$out/rotte" $R/mzlab_rotte_wasm.wasm $limit_rotte "rotte" @motore
# the assistant (crates/mzlab-llm-wasm: llm-provider/1 requests, answer streams, recordings), loaded by mzlab/web/llm/llm-worker.js for the facts of the Flow (profile «ricerca») or after the opt-in; never fused with another module
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-llm-wasm
wasm_pack "$out/llm" $R/mzlab_llm_wasm.wasm $limit_llm "assistant" @motore
# the import of an official regulatory file (ECHA Candidate List, Proposition 65) into a private .mzsus, loaded only by the Library window when the user imports one: crates/mzlab-normative-wasm
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-normative-wasm
wasm_pack "$out/normative" $R/mzlab_normative_wasm.wasm $limit "normative import" @motore
# the readers of the tables of other programs (mzMine, FBMN/GNPS2, MS-DIAL), loaded only when the user opens one: crates/mzlab-feat-import-wasm
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-feat-import-wasm
wasm_pack "$out/import" $R/mzlab_feat_import_wasm.wasm $limit "mzFinder import" @motore
fi
if want b; then
# Attributio (administrator mode): ONE .wasm "attributio-all" with the table of levels, the Banco 2b, the exports and the MS2 engine (the manifest keys of the four point to it; the workers load it once).
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-attributio-wasm --features all
wasm_pack "$out/attributio-all" $R/mzlab_attributio_wasm.wasm $limit_attributio "mzFinder attributio" @motore
mkdir -p "$out/attributio-all" && cp crates/mzlab-feat/data/classi-1.json "$out/attributio-all/classi-1.json"
# the low-resolution Dig of mzFinder (crates/mzlab-finder-lr-wasm), loaded by the worker of mzFinder only for a series of full-scan unit-resolution files
tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-finder-lr-wasm
wasm_pack "$out/finder-lr" $R/mzlab_finder_lr_wasm.wasm $limit_lr "mzFinder finder-lr" @motore
# the readers module (crates/mzlab-lettori-wasm), loaded only when a file needs it: tools/build_lettori_wasm.sh (it has its own jobs: it overlaps with the ones above)
# part b leaves the readers to the job of the engine in sito-rust.yml (it balances the three jobs); all (pages, release) builds them here
[ "$part" = b ] || tools/build_lettori_wasm.sh "${1:-site/static}"
fi
wasm_wait
[ "$part" = all ] || [ "$part" = manifest ] || exit 0
python3 - "$out" <<'P'
import hashlib, json, os, sys
o = sys.argv[1]
sha = lambda d: hashlib.sha256(open(f"{o}/{d}/mzlab_feat_wasm_bg.wasm", "rb").read()).hexdigest()
# the five modules of the HR group: one entry each; fused, they all name the .wasm "hr" (same file and SHA-256, the workers load it once)
fused = os.path.isdir(f"{o}/hr")
dirs = {"feat": "feat", "proofs": "proofs", "finder": "finder", "finder_edge": "finder-edge", "finder_msn": "finder-msn"}
hr = {k: {"js": f"{d}/mzlab_feat_wasm.js", "wasm": f"{d}/mzlab_feat_wasm_bg.wasm", "sha256": sha(d)} for k, d in ((k, "hr" if fused else v) for k, v in dirs.items())}
json.dump({"feat": hr["feat"], "proofs": hr["proofs"]}, open(f"{o}/manifest.json", "w"), indent=1)
m = json.load(open(f"{o}/manifest.json"))
m["table"] = {"js": "table/mzlab_feat_table_wasm.js", "wasm": "table/mzlab_feat_table_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/table/mzlab_feat_table_wasm_bg.wasm", "rb").read()).hexdigest()}
m["finder"] = hr["finder"]
m["finder_edge"] = hr["finder_edge"]
m["finder_msn"] = hr["finder_msn"]
m["formula"] = {"js": "formula/mzlab_feat_wasm.js", "wasm": "formula/mzlab_feat_wasm_bg.wasm", "sha256": sha("formula")}
m["flowtab"] = {"js": "flowtab/mzlab_feat_wasm.js", "wasm": "flowtab/mzlab_feat_wasm_bg.wasm", "sha256": sha("flowtab")}
m["interp"] = {"js": "interp/mzlab_feat_wasm.js", "wasm": "interp/mzlab_feat_wasm_bg.wasm", "sha256": sha("interp")}
m["untargeted"] = {"js": "untargeted/mzlab_feat_wasm.js", "wasm": "untargeted/mzlab_feat_wasm_bg.wasm", "sha256": sha("untargeted")}
m["grafo"] = {"js": "grafo/mzlab_feat_grafo_wasm.js", "wasm": "grafo/mzlab_feat_grafo_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/grafo/mzlab_feat_grafo_wasm_bg.wasm", "rb").read()).hexdigest()}
m["chem"] = {"js": "chem/mzlab_chem_wasm.js", "wasm": "chem/mzlab_chem_wasm_bg.wasm",
             "sha256": hashlib.sha256(open(f"{o}/chem/mzlab_chem_wasm_bg.wasm", "rb").read()).hexdigest()}
m["llm"] = {"js": "llm/mzlab_llm_wasm.js", "wasm": "llm/mzlab_llm_wasm_bg.wasm",
            "sha256": hashlib.sha256(open(f"{o}/llm/mzlab_llm_wasm_bg.wasm", "rb").read()).hexdigest()}
m["msngrafo"] = {"js": "msngrafo/mzlab_msngrafo_wasm.js", "wasm": "msngrafo/mzlab_msngrafo_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/msngrafo/mzlab_msngrafo_wasm_bg.wasm", "rb").read()).hexdigest()}
m["msnregole"] = {"js": "msnregole/mzlab_msnregole_wasm.js", "wasm": "msnregole/mzlab_msnregole_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/msnregole/mzlab_msnregole_wasm_bg.wasm", "rb").read()).hexdigest()}
m["rotte"] = {"js": "rotte/mzlab_rotte_wasm.js", "wasm": "rotte/mzlab_rotte_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/rotte/mzlab_rotte_wasm_bg.wasm", "rb").read()).hexdigest()}
m["normative"] = {"js": "normative/mzlab_normative_wasm.js", "wasm": "normative/mzlab_normative_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/normative/mzlab_normative_wasm_bg.wasm", "rb").read()).hexdigest()}
m["import"] = {"js": "import/mzlab_feat_import_wasm.js", "wasm": "import/mzlab_feat_import_wasm_bg.wasm",
              "sha256": hashlib.sha256(open(f"{o}/import/mzlab_feat_import_wasm_bg.wasm", "rb").read()).hexdigest()}
def attributio(dirname):
    return {"js": f"{dirname}/mzlab_attributio_wasm.js", "wasm": f"{dirname}/mzlab_attributio_wasm_bg.wasm",
            "sha256": hashlib.sha256(open(f"{o}/{dirname}/mzlab_attributio_wasm_bg.wasm", "rb").read()).hexdigest()}
if os.path.isdir(f"{o}/attributio-all"):
    # fused: the four keys name the same file; the table of classes is beside it
    for k in ("attributio", "attributio_rev", "attributio_export", "attributio_ms2"):
        m[k] = attributio("attributio-all")
    m["attributio_rev"]["classes"] = "attributio-all/classi-1.json"
else:
    for k, dirname in (("attributio", "attributio"), ("attributio_rev", "attributio-rev"), ("attributio_export", "attributio-export"), ("attributio_ms2", "attributio-ms2")):
        m[k] = attributio(dirname)
    m["attributio_rev"]["classes"] = "attributio-rev/classi-1.json"
m["finder_lr"] = {"js": "finder-lr/mzlab_finder_lr_wasm.js", "wasm": "finder-lr/mzlab_finder_lr_wasm_bg.wasm",
                 "sha256": hashlib.sha256(open(f"{o}/finder-lr/mzlab_finder_lr_wasm_bg.wasm", "rb").read()).hexdigest()}
json.dump(m, open(f"{o}/manifest.json", "w"), indent=1)
P
