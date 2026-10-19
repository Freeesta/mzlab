# Study "Flow, the graphics": how the canvas of modules looks and is used

Goal (Federico, 19/10/2026): a "Flow" section with modules that are dragged and linked one after the other (vendor raw reading → mzML → peaks and chromatograms → TP/screening workflows → identification → drawing → theory), on the web or in the desktop app. This study decides **only the graphics**: how the canvas is drawn, how it is touched, how modules talk to each other ON SCREEN. Base: `main` at `2e64041`.

It builds on two parallel studies and does not repeat them:
- `docs/studi/WORKFLOW_MODULARI.md` (PR #1095): `passo/1` (typed ports, params as JSON Schema), `mzflow/2` (the plan), `prov/1`, and above all `workflow/COMUNICAZIONE.md` (link rule, proposed conversion, order, cache, canvas ↔ engine messages). **Here is only the way to show them.**
- `docs/studi/STRUTTURA_DATO.md` (PR #1098): the `mol/1` record behind the `structure` port and `trasf/1` behind `transform`.

Study files (`docs/studi/flow_grafica/`, probes: not shipped in the site, `tools/indice.py` skips `docs/studi/`):
- `prototipo.html` + `flow-proto-model.js` (rules, 153 lines) + `flow-proto.js` (canvas, 405 lines) + `flow-proto.css` (74 lines): a chain of fake modules with typed ports, plain JS, no dependency;
- `misura.py`: 7 acceptance checks + axe + smoothness measures with Playwright (§9).

**Marks**: **V** = verified (read in a cited source or measured here with the written command); **I** = hypothesis, or remembered and not re-read.

## 0. Verdict
1. **We write the canvas ourselves**, in plain JS: DOM for the nodes, one SVG for the wires. The prototype does everything needed in ~560 lines (13.6 KB gzip) and stays smooth with 200 nodes even with the CPU slowed 4× (V, §9). No existing library meets "no bundler", keyboard/screen-reader access and a permissive licence together (§2).
2. **Rules are not in the canvas**: compatibility, conversions, order, cache are decided by the engine (`plan.check` in Rust, COMUNICAZIONE.md §5). The canvas colours the engine's answer. In the prototype the rules live in a separate file to show that boundary.
3. **Every data type = colour + shape + letter**: no information rides on colour alone (Okabe-Ito has 8 colours, there are 15 types).
4. **While you drag, every input says whether it fits**: green "compatible", orange "⇄ needs a conversion", grey "no" with the reason; the missing step is **proposed** with one click, never inserted silently.
5. **Everything also works without dragging** (click-click, keyboard, a "Steps list" that is the data table of the graph), as WCAG 2.5.7 and the project's `TDATI` rule require.
6. **Flow is the home of every module we will ever write** (Federico, 19/10/2026), English only. **Tabui moves inside Flow, split into modules**: what is a program today (e.g. feature finding, the Pipeline) becomes one or more nodes; Attributio likewise. Nodes can open a preview body (structure, spectrum, confidence level) drawn by the existing renderers (§7).
7. **Where Flow lives**: a section of the main app (`#/flow`), English only, lazy-loaded, not on phones (§8).

## 1. What the best tools do, and what we take
| Tool | Graphic idea | Do we take it? | Source |
|---|---|---|---|
| KNIME | a "traffic light" under each node: red not configured, yellow ready, green executed, cross for error | yes, but with **shapes** besides colour (empty, dashed, filled circle, square) | KNIME user guide (V); state details from a UGR course (V, secondary) |
| KNIME | ports of different shape for data, models, variables | yes: shape per type family | KNIME "Node Ports" (I: page not read) |
| Unreal Blueprints | port colour per type; dragging a wire into empty space lists only the compatible nodes | yes, both (prototype: release in empty space → filtered palette) | Epic, "Nodes in Unreal Engine" (V) |
| Unreal Blueprints | conversion node wired in between (`AutowireConversionNode`) | yes, but **proposed** and confirmed with one click | UE 4.26 API (V); when the editor inserts it alone: not found (I) |
| Galaxy | "may this output feed this input?" lives in the terminal code (`ConnectionAcceptable`); implicit format conversions that do not chain (a→b→c no) | yes the per-port check; we chain (breadth-first, COMUNICAZIONE.md §2) | Galaxy commits and mailing list (V) |
| Orange | "channels" have a name and a type; when two widgets can be linked in several ways, a dialog asks which | yes: if several ports fit, ask | orange-widget-base, `Input`/`Output` (V); the dialog itself: remembered (I) |
| n8n | each node shows the input and output data of the last run; "pinned" data to test without recomputing | yes the preview; "pinned" = our cache key | remembered (I) |
| Nuke | thumbnail of the image on the node ("postage stamp") | yes for spectra, XIC, structures: small and on request (§7) | remembered (I) |
| Houdini | "dirty" nodes to recompute when an upstream parameter changes | yes: "needs run" state flowing to descendants | remembered (I) |
| Node-RED | palette on the left, deploy button, nodes grouped in subflows | yes palette and groups; "deploy" = our "Run" | remembered (I) |
| ComfyUI | canvas on `<canvas>` (LiteGraph): fast with many nodes, but nodes and ports are not page elements | **no**: a screen reader does not see them | README of `@comfyorg/litegraph` (V) |
| React Flow (xyflow) | nodes and wires reachable with Tab, Enter selects, arrows move, Escape deselects; configurable screen-reader texts (`ariaLabelConfig`), move announced in a live region | yes, **the whole keyboard model** (idea, not code) | reactflow.dev, "Accessibility" and "AriaLabelConfig" (V) |

## 2. Libraries: use or write?
| Library | Licence | Needs | Verdict |
|---|---|---|---|
| React Flow / Svelte Flow (xyflow) | MIT (I: file not re-read) | React or Svelte + bundler | no: framework and bundler are ruled out (principle 2) |
| Rete.js v2 | MIT (plugins V; core to re-check in its LICENSE) | TypeScript packages + a render plugin (React/Vue/Angular/Svelte or Lit) | no: bundler and a render framework |
| LiteGraph.js (and the ComfyUI fork) | MIT (V for the original; fork: to read) | no dependency, draws on canvas; ComfyUI copy ~506 kB unminified (V) | no: canvas = no keyboard nor screen readers; heavy |
| Drawflow | MIT (V) | plain JS, no dependency (V) | idea yes, code no: last push 19/10/2024 (V), no typed ports, no accessibility |
| Baklava.js, Litegraph.ts, JointJS | Baklava MIT, JointJS MPL-2.0 (I) | Vue / TS / MPL is not in the permissive list of `deny.toml` (I) | no |

**Why in house** (V from the prototype): the core (DOM nodes, SVG wires, drag, zoom, pinch) is ~250 lines; the rest is ours anyway (typed ports, proposed conversions, previews, states, mzflow/2). A library would add weight, a bundler and an accessibility model to fix. We take ideas (§1), not code.

## 3. How modules talk ON SCREEN
**3.1 The port.** A dot with an Okabe-Ito colour (CSS variable `--t-<type>`, light and dark), a **shape** and the port name; full `aria-label` ("Input index, Scan index, one per file, of Detect features"). Proposal for the 15 `dataType` values:

| Shape | Family | Types (colour) |
|---|---|---|
| square | raw and converted files | `raw` (orange #E69F00), `mzml` (light orange) |
| circle | indexes and traces | `mzidx` (blue #0072B2), `xic` (sky blue #56B4E9) |
| diamond | peaks and spectra per file | `peaks` (green #009E73), `spectra` (light green), `rtmap` (yellow #F0E442 with border) |
| triangle | tables | `table` (vermilion #D55E00), `annot`, `tp` (purple) |
| hexagon | chemistry | `structure` (pink #CC79A7), `transform` (pink with a bar) |
| hollow circle | end of chain | `samples`, `qc`, `report` (grey) |

Two types of the same family share the shape: colour and name tell them apart; who cannot see colours still has shape + name + hint text (V in the prototype: `glyph()` in `flow-proto.js`).

**3.2 While linking.** From the first move away from an output (or a click on it) every input gets one of three states, computed by `check()` (in the product: from the answer of `plan.check`):
- **ok**: dot enlarged, green bold text;
- **adapt**: orange text + "⇄", hint "Needs: Build index";
- **no**: port at 40 %, hint with the reason from its key (`err.flow.type`, `err.flow.cycle`, `err.flow.cardinality`).
Nodes with no useful input fade to 35 %; the status bar announces "Pick an input: 2 compatible, 1 through a conversion" (`aria-live` region). Releasing on an "adapt" input opens a bubble "Insert: Build index" with **Insert** and **Cancel** (V: check "adapter proposed, not inserted"). Releasing in empty space opens the palette with only the steps that accept that type (Unreal's idea).

**3.3 The wire.** The colour of the type it carries; reachable with Tab (`role="button"`, `aria-label` "Link Align samples table to Export CSV table, Feature table"); on hover or Enter it shows the **preview**: type, descriptor (number of files, MB, facets `hr`/`lr`, polarity, rows), cache key, and "Not computed yet" if the datum does not exist. Delete removes it. While the downstream step runs the wire "flows" (animated dash, off with `prefers-reduced-motion`).

**3.4 Errors.** Three places, always the same text from the catalogue of `err.flow.*` keys (never text written in the canvas code): border and red square on the node, the missing port, the status bar that moves focus to the wrong node (V: "Missing input “index” in Detect features."). A failed step stops only its descendants (COMUNICAZIONE.md §3): siblings stay green.

**3.5 Node states** (shape + colour + hidden text for screen readers): `idle` empty circle, `queued` dashed, `running` spinning (still with reduced motion) and a progress bar, `done` filled green, `cached` green ring, `stale` ("needs run") orange stripes, `error` red square. The progress bar uses the `step.progress {done,total}` messages of today's single bar (`progresso.js`, V in the code).

## 4. Few clicks, icons, motion
- **Clicks counted in the prototype** (V): link = one drag or two clicks; missing conversion = +1 click; run = 1 click or key R; add a step = A, two letters, Enter.
- **Icons**: in the product from `QICON` (`icons-modi.js`, `currentColor`, follows the theme; V), one per step family (`passo/1.icon`). The prototype uses Unicode characters so as not to touch site code.
- **Motion**: only `transform` and `opacity`, with the tokens `--t-fast` 120 ms and `--t-med` 180 ms of `transizioni.css`, which `prefers-reduced-motion` and `data-a11y-reduce` set to 0 (V in the code). Nodes and bubbles appear with the panels' animation; inserting an adapter slides with FLIP as in `transizioni.js` (I).
- **One form for parameters**: double click or Enter on a node opens the panel generated from the params schema (number with unit and limits, "Auto" for `x-estimate` with its proof beside it); changing a value marks the node and its descendants "needs run" (V in the prototype).

## 5. Touch, keyboard, performance
**Touch** (V in the prototype): Pointer Events (mouse, finger, pen alike), `touch-action:none` on the canvas, two-finger pinch = zoom, targets 28 px with a mouse and 40 px with `pointer:coarse`. On phones Flow is not downloaded (like Dati and Disegno, `telefono.js`); later at most a read-only list (I).

**Keyboard** (V in the prototype, check "keyboard only"): Tab through nodes, ports and wires; arrows move a node by 16 px (Shift 64); Enter on an output "arms" the link, Enter on an input closes it; Escape cancels; Delete removes; A opens the palette; R runs.
Missing in the prototype and required in the product: **"Steps list"**, the data table of the graph (one step per row: inputs as drop-downs listing only compatible outputs, state, actions). It is the full alternative to the canvas for who cannot use it, as `TDATI` is for charts (rule 6 of AGENTS.md).
WCAG 2.2 AA to meet: 2.5.7 dragging with an alternative, 2.5.8 targets ≥ 24 px, 2.4.11 focus not obscured (a bubble must not cover the focused node), 1.4.11 contrast 3:1 of ports and wires on the background (to measure for every colour, I).

**Performance** (rules used in the prototype, V): nodes are `div`s moved with `transform`; pan and zoom = one `transform` on the layer; port positions measured once per node, then only added; moves batched into one `requestAnimationFrame` that redraws only the wires of the moved node; all layout reads first, then all writes (this rule alone took the first draw of 506 nodes from 909 to 166-370 ms, V).
Beyond ~300 nodes, off-screen nodes must be hidden and simplified at low zoom (I); a real workflow has 10-30 steps (`PROVE_SU_CARTA.md`), closed groups (subflows) keep the canvas small.

## 6. Saving, running, cache
**Saving** (V in the prototype, check "round trip"): the file is `mzflow/2` with `passi` (the plan) and a separate **`ui`** block: `view {x,y,k}`, `nodes {id: {x,y}}`, later closed groups, expanded previews and notes. **Proposal to the WORKFLOW study**: add `ui` as an optional field of `mzflow/2`, outside `plan_sha256` (moving a node does not change the plan). Without `ui` the canvas lays nodes out by levels of the Kahn order (I). The canvas reads titles, icons, ports and params from the `passo/1` descriptors: no step is hard-coded in the graphics.

**Run and cache** (V in the prototype, check "second run from cache"): Kahn order, key per node = step + canonical params + keys of its inputs; on the second run every node is "from cache". In the product: the engine's SHA-256 key (COMUNICAZIONE.md §3), "Run" and "Run up to here" in the node menu, cancel per node, parallelism decided by the scheduler (the canvas shows several "running" nodes at once).

## 7. Attributio and Tabui as module families
Flow collects every module (Federico: "the tool that gathers all the modules we could ever invent"). **Tabui is split into Flow modules**: its passes become nodes ("1 · Files" → `import.*`/`index.mzidx`; Pipeline → `features.estimate`, `features.detect`, `features.align`, `features.gapfill`; feature table and QC → `table.filter` + table view; TP view, groups, interpretation, positions → the Tabui families below). **Attributio** gives the identification, MS2 and regulatory/ecotox families. Their outputs are chemistry, not numbers, so their nodes need a **preview body** under the ports.
**Migration (I)**: today's guided Tabui becomes a saved Flow (`tabui.mzflow.json`, a preset opened with one click) whose nodes call the same `.wasm` and open today's Tabui panels as their "Open in view"; the old entry stays until every pass exists as a node, then the panels live only behind nodes. Each pass moved is one slice (§10), with an e2e proving the same table byte for byte.

**7.1 Nodes and ports** (step ids indicative; the real ones come from the W-1 descriptors):

| Family | Example nodes | In → out |
|---|---|---|
| Attributio · identification | library match, formula candidates, level cascade | `table` + `spectra` → `annot` |
| Attributio · MS2 | in silico fragmentation, fragment assignment, RT prediction, chimeric check | `spectra` + `structure` → `annot` (fragments, intruders) |
| Attributio · regulatory/ecotox | regulatory lists, aquatic hazard records | `annot` / `structure` → `annot` + `report` |
| Tabui · TP | TP link, interpretation per row, groups/conditions | `table` + `structure` → `tp` |
| Tabui · transformations | parent → product rules, Δ, atom mapping | `structure` → `transform` |
| Tabui · positions | site of the substitution compatible with MS2 | `tp` + `transform` + `spectra` → `annot` |
| Drawing | parent structure, structure from a result | (Ketcher) → `structure`; `structure` → Disegno |

**7.2 Preview bodies** (all on request, collapsed by default; one click or Space on the node header toggles them; each has its data table, `TDATI`):
- **Structure** (`structure`, `transform`): 2D drawing by OpenChemLib (already in the site, V), 160×100 px, read only. For `transform`, parent and product side by side with the changed atoms or the uncertain region outlined (the `regions` of `mol/1`, with `support`); a hatch, not only a colour, marks "region, not a site".
- **Spectrum** (`spectra`, MS2 nodes): mini stick plot; for a match, a mirror plot (experimental up, reference or in silico down); **chimeric spectra**: intruder peaks drawn hollow and dashed, with a count "3 intruder peaks"; precursor m/z and RT as text under the plot.
- **Confidence level** (`annot`, `tp`): a badge with the level text ("L2a", "L3", "Prev. 2") and the existing level colours `--lv1 … --lv5` of `app.css` (Okabe-Ito, V in the code), plus the shape of the scale step. The badge shows the best level of the rows; the preview lists "12 rows: 1 × L2a, 4 × L3, 7 × L4". A level is a candidate with proofs, never a verdict (principle 1): the preview always states the evidence ("MS2 match 0.82, 9/14 fragments").
- **Regulatory/ecotox**: a short list of the lists hit and the hazard class with its source; no colour scale of "danger" without its text.
- **Node size**: header only (~200×40 px) by default; expanded ~200×160 px. Previews render only for nodes on screen and expanded, so 200 nodes do not mean 200 drawings (I, to measure in F-6).

**7.3 Rules for these families**: in the LR world (facet `lr`) the palette hides `tp.*`, `annot.*` and the identification family (principle 1, `in.requires` of COMUNICAZIONE.md §2); mzFinder stays hidden as today. Spectrum and structure previews reuse the existing renderers; if they cannot draw into a small given canvas, the preview is a button "Open in view" until they can (I, to check for `spettro.js`, `cromato.js`, `TP_Mine/js/46_tp_mol.js`).

**7.4 Drawing and views**: Ketcher stays the editor (`draw.js`, `sandbox` iframe, V); the "Parent structure" node has "Draw…", which opens Disegno with the current structure, and "Use in Flow", which brings back a `mol/1` record. Every output has "Open in view": `mzidx`/`xic` → Dati panels (TIC/XIC/spectrum), `table`/`tp` → Tabui table (`PIPET`), `annot` → Attributio panel, `structure` → Disegno. "?" on a node opens the related Teoria chapter (`#/teoria/N`, Italian as always; no new Teoria text).

## 8. Where Flow lives (recommendation)
**Decided by Federico: Flow holds all modules and is English only; Tabui lives inside it, split into modules.** Recommended placement: a section of the main app, route `#/flow`, with an entry in the header; the canvas in its own folder (`flow/` at the root), the modules' code where it is today (`TP_Mine/`, Attributio) until each pass is moved (§7); lazy-loaded when opened; not on phones.
Why:
1. **English only**: the modules are English (Tabui by rule, Attributio, the `passo/1` titles and the `err.flow.*` texts of the WORKFLOW study): one catalogue of descriptors, not two.
2. **The main app is where the files are**: OPFS, the engine worker, Dati and Disegno are already loaded there; "Open in view" needs no second page and no second engine.
3. **Students are not blocked**: the LR facet hides identification and TP families (§7.3), so a student can run raw → mzML → index → chromatograms in Flow without seeing answers. Flow is a research tool first; Dati stays the student's place.
4. **Cost**: nothing loads until `#/flow` is opened (as the Pipeline's `.wasm`, V), so the first paint and the tour do not change.
What it needs: `flow/` added to `SKIP_DIRS` of `tools/controlla_i18n.py` as `TP_Mine/` is (V, line 35), and the guard of `tests/test_tabui_inglese.py` (English only, never the word "formula" alone) extended to `flow/`. The only bilingual labels are the header button and the tour step, as `lang/` keys of the main app.

## 9. Probes and acceptance criteria
`python3 docs/studi/flow_grafica/misura.py [--cpu 4]` (Chromium 1194 of `/opt/pw-browsers`, window 1366×768, cloud container with 4 CPUs). Today: **7/7 functional checks green, axe WCAG 2.2 AA 0 violations in light and dark** (V). Measures are noisy (shared container): every row gives the range of 2-3 runs.

| Functional check (V) | Result |
|---|---|
| dragging `raw` onto the `index` input marks the port "adapt" | ok |
| on release the proposal appears and the graph does not change | ok |
| "Insert" adds 1 node and 2 wires | ok |
| the second "Run" takes everything from the cache | ok ("0 step(s) run, 7 from cache") |
| link made with Tab and Enter only | ok |
| right arrow moves exactly one node | ok |
| save → open → save gives the same `passi` | ok |

Smoothness (V; interval between frames over 90 moves, median / 95th percentile / frames over 50 ms):

| Nodes | CPU | first draw | drag a node | pan the canvas |
|---|---|---|---|---|
| 6 | ×1 | 3 ms | 16.7 / 16.7 / 0 | 16.7 / 16.7 / 0 |
| 206 | ×1 | 65-95 ms | 16.7 / 16.7-16.8 / 0 | 16.7 / 33.2-33.3 / 0 |
| 506 | ×1 | 166-370 ms | 16.7 / 16.7-16.8 / 0 | 16.7 / 33.4 / 0-1 |
| 206 | ×4 | 344-382 ms | 16.7 / 33.4 / 0-1 | 16.7 / 33.4 / 1-2 |
| 506 | ×4 | 1.3-1.8 s | 16.7 / 16.8 / 1 | **33.3 / 83.4 / 34-42** |

Reading: dragging a node stays at 60 frames per second even at 500 nodes; panning the whole canvas at 500 nodes with a slow CPU drops to ~30 (V). Shrinking the wires SVG from 20 000 px to 1 px with `overflow:visible` makes it **worse** (up to 89 frames over 50 ms at 206 nodes, V): the real cure is hiding off-screen nodes (I).

**Criteria for the product** (each slice inherits them in an e2e):
1. the 7 functional checks above, on the real code, in `tests_e2e/`;
2. axe WCAG 2.2 AA at 0 violations in light, dark and high contrast; `e2e_axe_grafici` green;
3. with 200 nodes and CPU ×4: median ≤ 20 ms and ≤ 2 frames over 50 ms for dragging and panning;
4. with `prefers-reduced-motion` no CSS animation running (`document.getAnimations().length == 0` after a "Run");
5. "Steps list" does everything the canvas does (same `mzflow/2` out);
6. no canvas text outside the message catalogue; never the word "formula" alone;
7. with 50 expanded preview nodes (structure + spectrum) the criteria of point 3 still hold;
8. tour: a "Flow" step finds its visible target (`e2e_tour`).

## 10. Plan in slices (small PRs, files ≤ 500 lines, functions ≤ 80, `// @ts-check`)
W-1 and W-2 of the WORKFLOW study come first (`passo/1` descriptors, `plan.check` in Rust): the canvas writes no rules (COMUNICAZIONE.md §5, "What NOT to do" 10).

| Slice | Scope | Estimated size (I) | Proof |
|---|---|---|---|
| F-1 read only | `#/flow` route, opens an `mzflow/2`, draws nodes and wires from descriptors, zoom/pan, "Steps list" | ~350 lines in 2 files | e2e open + axe |
| F-2 editing | move (mouse, touch, arrows), link (drag, click-click, keyboard), remove; states from `plan.check` | ~350 lines | checks 5-6 of §9 |
| F-3 conversions and palette | "Insert" proposal, palette filtered by type and by the `lr` facet | ~200 lines | checks 1-3 |
| F-4 run | states, progress, cache, "needs run", cancel, "Run up to here" | ~250 lines | check 4 + criterion 4 |
| F-5 wire previews | wire bubble, "Open in view" | ~200 lines | e2e preview |
| F-6 preview bodies | structure, spectrum (mirror, chimeric), level badge, `TDATI` for each | ~300 lines | criterion 7 |
| F-7 drawing | "Parent structure" node ↔ Disegno with `mol/1` | ~150 lines | e2e round trip |
| F-8 scale and touch | off-screen nodes hidden, closed groups, pinch | ~200 lines | criterion 3 |
| F-9 desktop app | same page in Tauri, native threads | check | desktop e2e |
| T-1…T-n Tabui into Flow | one Tabui pass per PR becomes nodes (Files, Pipeline, table/QC, TP, groups, positions); `tabui.mzflow.json` preset | ~150-250 lines each | same table byte for byte as today's Tabui |

Total ~2 000 lines of JS in ~7 files, ~45-50 KB gzip overall (I; the prototype is 13.6 KB gzip). Flow's JS downloads only when Flow is opened. No `.wasm` grows: the graphics do no computation.

## 11. Licences and rules
Ideas yes, other people's code no: no line copied from xyflow, Rete, LiteGraph, Drawflow, Node-RED, Galaxy (only docs and notes were read). mzLab MIT; no new dependency; no binaries in git (the prototype is text). Principle 1: in the LR world the TP, annotation and identification families do not appear in the palette (§7.3).

## 12. What NOT to do
1. Do not put compatibility rules in the canvas: the engine decides (§0.2).
2. Do not use `<canvas>` for the nodes: keyboard and screen readers are lost.
3. Do not carry information by colour alone; no colours outside CSS variables.
4. Do not insert conversions without asking.
5. Do not save positions in the plan (`passi`): they belong to `ui`, outside the hash.
6. Do not build a structure editor: Ketcher stays.
7. Do not draw a level or a hazard as a verdict: always with its evidence (§7.2).
8. Do not copy code from libraries or node tools, even MIT ones: adapting costs more than writing (V: the core is ~250 lines).

## 13. Decisions for Federico
1. **Placement**: §8 (main app, `#/flow`, lazy, not on phones); English only and Tabui inside Flow are decided. Confirm the migration path of §7 (Tabui as a saved Flow first, panels behind nodes after).
2. **Proposal to the WORKFLOW study**: optional `ui` field in `mzflow/2`, outside `plan_sha256`.
3. **Colours and shapes of §3.1**: confirm before F-1. Changing them later is easy: saved files hold the types, not the colours.

## Sources
- KNIME Analytics Platform User Guide: https://docs.knime.com/latest/analytics_platform_user_guide/ ; node states (secondary): https://abierta.ugr.es/pluginfile.php/83073/mod_resource/content/9/8.1_en.pdf
- Unreal Engine, nodes and pins: https://dev.epicgames.com/documentation/en-us/unreal-engine/nodes-in-unreal-engine ; `AutowireConversionNode`: https://docs.unrealengine.com/4.26/en-US/API/Editor/BlueprintGraph/UEdGraphSchema_K2/AutowireConversionNode/index.html
- Galaxy, implicit conversions: https://lists.galaxyproject.org/archives/list/galaxy-dev@lists.galaxyproject.org/thread/FQUD3SZFEXP4ZRDR2MOCX6SZMXSFZ3C6/ ; editor: https://galaxyproject.org/learn/advanced-workflow/basic-editing/
- Orange, widget signals: https://orange-widget-base.readthedocs.io/en/stable/widget.html
- React Flow, accessibility: https://reactflow.dev/learn/advanced-use/accessibility ; https://reactflow.dev/api-reference/types/aria-label-config
- LiteGraph (ComfyUI fork): https://cdn.jsdelivr.net/npm/@comfyorg/litegraph@0.17.2/README.md ; licence of the original: https://www.jsdelivr.com/package/npm/litegraph.js
- Drawflow: https://github.com/jerosoler/Drawflow (MIT, plain JS; last push from https://awesome.ecosyste.ms/projects/github.com%2Fjerosoler%2Fdrawflow)
- Rete.js plugins: https://classic.yarnpkg.com/en/package/rete-area-plugin ; https://classic.yarnpkg.com/en/package/rete-connection-plugin
- WCAG 2.2: https://www.w3.org/TR/WCAG22/ (2.5.7 Dragging Movements, 2.5.8 Target Size Minimum, 2.4.11 Focus Not Obscured, 1.4.11 Non-text Contrast)
- mzLab code read: `mzlab/web/transizioni.css`, `transizioni.js`, `app.css` (variables, `--lv*`), `icons-modi.js` (`QICON`), `tdati.js`, `draw.js`, `touch.js`, `explore.js` (Okabe-Ito), `TP_Mine/js/32_pipeline.js`, `tools/controlla.py`, `tools/controlla_i18n.py`, `tools/indice.py`.
