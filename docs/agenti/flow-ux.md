# Flow: cards, views and names of the "MSn infusion" model

Current state of names, groups and view cards of the web Flow. Flow texts in English only; logic in Rust, JS connects.

## Cards and views

- **Names and groups** (`flow-names.js`): `F.NAMES` gives an English name to every step (the id is never shown); `F.GROUPS` are Simple groups outside the base (Export = `msn.library_export` + `msn.table_export`); `F.INFUSION_STEPS` are the steps that read the Data infusion without `file:` in the plan and receive the edge from Raw files (`F.canvasModel`, `flow-fix.js`).
- **View card** (`F.STEP_VIEWS`): a `kind: "view"` card (key `view:<instance>`) next to the step, outside the plan (`mzflow/2` unchanged); it is off until `S.results` has the result of its type, then it opens `F.openResult` with the focus in the view. With ≤ 5 cards `F.layout` uses W 250, H 88.
- File data in the nodes (`flow-samples.js`): `F.fileNature()` reads from `E.files` the polarity (if all equal) and the resolution (`prof1.hr`); `F.defaults` shows the files' `polarity` and `nominal` as defaults (`F.fromFiles`), `F.addStep` writes the preset that matches the resolution (`inheritPreset`), `F.runPlanFromData` writes into the plan to run what the plan did not choose (`inheritNature`); the manual choice wins. Test: `tests/flow_natura_file.js`.
- **Plans with old-version steps**: `F.migrated` (`flow-inputs.js`, in the core) sends to the `flow/line` route (`F.lineCall`) every plan read from a file or proposed by the assistant that names an older version; the engine (`flow::migrate`) brings it to the current one, the page has no list of its own. Tests: `flow_command_tests.rs` (route = command line, byte for byte), `tests/test_flow_grafico.py`, `tests_e2e/e2e_flow_migrate.py`.

## Flow graphics (canvas rules)
- **Canvas**: written by us in plain JS (DOM for the nodes, SVG for the wires, never `<canvas>`, which loses keyboard and screen readers); no node library and no line copied from node tools, not even MIT. Compatibility, conversions and order are decided by the plan check (`flow::check` in Rust, `flow-check.js` identical on the test cases), the canvas colors the answer; a missing conversion is proposed with one click, never inserted by itself.
- **Ports** (`F.TYPES` of `flow/lazy/flow-registry.js`): every data type = Okabe-Ito color from a CSS variable + family shape + letter, never the color alone; saved files carry the types, not the colors, so changing them is easy. Everything works without dragging too (click-click, keyboard, step list). Goal of a chain (`F.GOAL_STEPS` of `flow-plan.js`, easy to change): the step that grades an identification, `annot.level`.

## Open: a simpler Flow for TPs
- **Not built**: five cards from left to right (Parent → Data → Features → Candidate TPs → Evidence & decisions, plus Export), the sample list is not a card but "Advanced" inside Data; the "Find transformation products" model that picks `tp.dig_lr` on unit-resolution files (it uses `tp.link`, HR only); times read from names and unconfirmed as warnings, not errors.
- **Open questions**: card names; warnings or errors (severity of `valida` in Rust); `tp.filter`, `tp.network`, `tp.position` on in the HR model; Evidence card in the LR world; report files.
- **Margins and first draw** (`e2e_flow_layout`): the `.flw.flx` page keeps the side margin of `.flw` (`10px 12px`, that of the header): title, status line, canvas and cards do not touch the edge at 1000, 1440 and 390 px. At first draw nothing moves: the cards of the first pass do not have the `fl-new` entrance, the stage (`.fl-stage`) stays `visibility:hidden` until the first `fit(false)` (no transition) and there is no layout jump from the click on the tab to the settled page.
