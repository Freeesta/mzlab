# Pairing samples with a table for the TP steps (`order_for_roles`, `samples/roles_pair.rs`)

Companion of `samples.md` (model, `order_like`, `import`). Written for the roles-in-TP fix: a standard or a dark control in the plan no longer stops `tp.dig_lr`.

- `tp.dig_lr`, `tp.link`, `tp.network` use `order_for_roles` in place of the strict `order_like` (every other step keeps the strict one, conformance expects `error.badSamples`). A sample whose file the table lacks is left out (`left_out`); a table file with no sample takes the role its name proposes (`proposed`; evidence = the word, the minutes for a t0, "" for a plain sample). Unknown fields stay. `error.badSamples` on a step's samples adds a hint naming `sample table`, `sample import`, `sample pair`.
- Each choice comes back as `{action,file,role,evidence}` (`notes_json`, `with_pairing`): the `pairing` key of the `tp.dig_lr` result (built in `run_dig::dig_inputs`, so site, app and CLI write the same bytes) and of `rete/1` (`tp.network`). `tp.link` writes a binary `.mzfeat`, so its notes are only stderr lines (`err.cli.pair.*`): technical limit, follow-up row `TP-LINK-NOTES` in `docs/coda/CODA.md`.
- `mzlab-cli sample pair SAMPLES.json [FILE... | --table T] [--json]` (`samples_pair.rs`, schema `docs/schemi/sample-pair-1.json`) gives the same notes without a run. Exit 0 all paired, 1 choices made, 2 usage, 3 not a `samples/1`, 4 missing input.
- Tests: `crates/mzlab-cli/src/flow_lr_roles_tests.rs`, `samples_pair_tests.rs`, `tests_e2e/e2e_flow_dig_lr.py` (browser leg, byte for byte).
