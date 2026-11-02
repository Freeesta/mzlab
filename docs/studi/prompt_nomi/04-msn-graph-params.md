# N04 (GATED on D1): `spectra.msn_graph@2` with English parameters and `msngrafo/2`

Start only if the COO's prompt records Federico's **yes to D1** (`docs/studi/NOMI_TERMINALE.md` §5). Otherwise reply to the COO «D1 not approved» and stop.

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, `docs/agenti/rust-step.md`, `rust-flow.md`, `mzrun.md`, `msn-grafo.md` §3 and the study §1, §2.3. Branch `claude/msn-graph-v2` from `origin/main` after N02 is merged. You are the only owner of `docs/schemi/steps-cli-1.json` for this PR.

**Goal.** The 14 Italian parameters of `spectra.msn_graph` get the English names of N02's flags; saved plans and `mzrun/1` presets at `@1` keep running with the same output bytes.

**Do.**
1. First, the mechanism (none exists): in `mzlab_core::flow` a per-step migration table `(step, from_version, to_version, fn(params) -> params)` applied by `flow::check`/canonicalisation when a plan names `spectra.msn_graph@1` (or no version and old names); the canonical plan written after a run says `@2` with new names. `mzrun` presets record `preset.param.migrated` with `compat.changes`. Ask the COO before writing more than this one table.
2. `flow_msngraph.rs`: descriptor `version: 2`, new names; `--set g.<old>=` is accepted and mapped to the new name, never printed.
3. `msngrafo/2`: the `opzioni` keys in English (`msngrafo/json.rs`); readers (`cerca.rs`, `libreria.rs`, `rotta.rs`, the page JS that reads `opzioni`, the desktop) read `/1` and `/2`; `error.newerFormat` rule of `formato-nuovo.md`.
4. Regenerate `steps-cli-1.json` with `MZLAB_UPDATE_STEPS=1` (never by hand); `strumenti/1` with `MZLAB_UPDATE_LLM=1` if it changes.

**Do not touch.** Other steps, `steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `F.liveJobs`, the route query keys (page parity: change them only if the page sends step params, then in both site and app in this PR).

**Tests.** A saved `@1` plan (old names) runs and its data outputs are byte-identical to before; the migrated `plan.json` has `@2`; `mzrun` preset at `@1` applies with `preset.param.migrated`; `msngrafo/1` files still open in site and app (e2e that reads a `/1` fixture); `flow_rule_tests.rs` green; site/app parity test of `msngrafo` byte-equal. Golden files that change are only the ones recording the new format, each listed in the PR.

**PR.** Title «spectra.msn_graph@2: English parameters, msngrafo/2». Body starts with `VISIBLE: no` (no UI text changes), says which goldens changed and why, and the migration table.
