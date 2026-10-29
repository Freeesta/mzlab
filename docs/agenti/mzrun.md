# The `mzrun/1` file (`*.mzrun.json`)

One analysis in one canonical JSON file, in two modes: a `record` replicates a run exactly, a `preset` keeps the parameters only and starts a new analysis. Schema `docs/schemi/mzrun-1.json` (JSON Schema 2020-12, `$id` `mzrun/1`), examples `tests/data/mzrun/` (synthetic, no lab data), test `tests/test_mzrun_schema.py`. The format is defined here and nothing reads or writes it yet: `mzlab_core::mzrun` (Rust) comes in later PRs, with the site and the app sharing the same bytes (parity rule of AGENTS.md).

## Canonical text and fingerprint
- The canonical text is the one of `flow::value_text` / `Resolved::text()` (`crates/mzlab-core/src/flow/canon.rs`): keys sorted, the shortest number form, UTF-8, LF line ends, no trailing spaces, no spaces between tokens. The number form is that of `canon.rs`; the Python check `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False)` agrees with it only for integers and strings, so the examples avoid floats with several representations (`5`, not `5.0`).
- `self_sha256` = SHA-256 of the canonical text of the whole object with the key `self_sha256` removed (64 lowercase hex). It is the fingerprint of the file. `plan_sha256` is the SHA-256 of the canonical text of `plan`.
- A file on disk is the canonical text plus one final LF.

## Fields (both modes unless marked)
- `schema` = `mzrun/1`; `kind` = `record|preset`; `min_reader` (integer, 1): the lowest schema version that reads the file.
- `engine`: `name`, `version`, `commit` (40 hex), `platform` (`wasm|native`); optional `target`, `rustc`, `cargo_lock_sha256`, `modules` (module name → SHA-256 of each `.wasm` or native binary). In a preset it is the origin of the parameters.
- `plan`: the `mzflow/2` object verbatim (contract `$defs/mzflow2`) and `plan_sha256`. In a preset the plan inputs are slots: `name` is the slot, `sha256` is 64 zeros and the steps read `file:SLOT`.
- `resolved`: instance → `step` (`id@version`), `algorithm` (`id@version`), `preset`, `params` (EVERY parameter, canonical: `mzflow/2` keeps only the differences from the defaults), `pin` (parameter → `fixed|estimate`), `estimates` (parameter → proof).
- `inputs` (record only): `alias` (`S01`, `BLK_P1`…; letters, digits, `_+.-`, no path, no URL), optional original `name`, `sha256`, `bytes`, `type` (data type of the contract), `role` (`tipo` of `campioni/2`), optional `tipo_bianco`, `polarity` (`+|-`), `injection_order`, `replicate`, `series`; optional `raw_sha256` (the vendor file) and `converter` `{name, version, image_digest}`.
- `slots` (preset only): `{slot (e.g. blank.procedure), type, role, polarity, min, max}`. `roles_sha256` (record): the `digest` of the `campioni/2` document (`campioni.md`).
- `resources`: `{kind (library|suspects|adducts|transformations|rules|other), id, version, licence, url, retrieved (YYYY-MM-DD), sha256, bytes, builtin}`. `seeds`: `instance.param` → integer.
- `decisions` (record): the manual acts of the chemist, in order: `{seq, kind (tpdec|feature|role|param), target, before, after, note_sha256, input_sha256}`; the `tpdec/1` decisions of the review are one kind.
- `revisions` (integer) and optional `plan_chain` (the `plan_sha256` of each revision): parameters changed in the middle of a run.
- `runs` (optional): sub-records of a run per polarity (ESI+ and ESI- are two templates merged at the end), each with its own plan, inputs, `prov` and `results`; `merge` `{instance, params, params_sha256}` describes the final step.
- `prov` (record only): the `prov/1` object verbatim. `results` (record only): `entities` (name → SHA-256) and `results_digest` (below).
- `limits`: `{code, detail}`, `code` ∈ `vendor.conversion.external`, `resource.online.unpinned`, `step.nondeterministic`, `engine.differs`, `llm.assisted`, `input.missing`, `preset.param.new`, `preset.param.migrated`.
- `compat`: the report of applying a preset: `{from_engine, changes[{instance, param, kind (new|migrated|removed), value, confirmed}]}`. `related`: `{doi, source_record_sha256}`.
- Mode rules (`if/then`): a record requires `inputs`, `prov`, `results`, `roles_sha256` and `decisions` and has no `slots`; a preset requires `slots` and has no `inputs`, `prov` or `results`.
- Privacy: no absolute path (a name starting with `/`, `\`, `~` or a drive letter such as `C:\` is refused), no URL in an alias. A file carries no lab data and no personal names.

## Record and preset
- `record` replicates a run: same plan, same resolved parameters, same inputs (by SHA-256), same resources, same seeds and decisions. Nothing is a verdict about the chemistry.
- `preset` is the pure function `to_preset(record)`: it keeps `engine`, `plan` (inputs turned into slots from `role`, `polarity` and the counts), `plan_sha256`, `resolved`, `resources`, `seeds`, `revisions`, `limits`, and drops `inputs` (aliases, hashes, bytes), `roles_sha256`, `decisions`, `prov`, `results`, `runs` and `related`. Same record, same bytes. It is uploaded to start a new analysis: the user fills each slot with files and the engine checks them against `type`, `role`, `polarity`, `min`, `max`.

## Applying a preset written by another version
- A parameter the running step has and the preset lacks (new): ALWAYS ask for confirmation, never silent; the answer goes in `compat.changes` (`confirmed`) and the limit `preset.param.new` is written.
- A parameter removed, or a new version of a step: a Rust migration (`preset.param.migrated`) or a refusal with `err.flow.step.version`.
- A schema above the reader (`min_reader` greater than the reader's): the file opens read-only with `error.newerFormat` (`formato-nuovo.md`), never rewritten.

## Comparing two records
- Verdicts come from `results_digest` only, the SHA-256 of the lines `name<TAB>sha256\n` sorted by name over `results.entities`. It leaves out `ms`, `heap_peak_bytes`, `cached` and the agent.
- `identical`: both digests equal. `differs`: same plan and inputs, digests differ (the entities that differ are listed). `not_comparable`: a different plan or other inputs, so the question has no answer.

## Decisions taken
- Storage: OPFS `mzlab/flow/records/`, with an export button; old records are kept.
- No signatures: the fingerprint is `self_sha256`; for a stable public reference the record is deposited and cited with a Zenodo DOI (`related.doi`).
- Raw vendor files are hashed in the background, with progress, so that `raw_sha256` costs no waiting.
- The methods paragraph is English only and generated in Rust from the record.
- Planned, later: RO-Crate (Provenance Run Crate) and mzQC exports.

## Later PRs
1. engine identity (commit, modules); 2. descriptor guard; 3. `mzrun` build/verify (`mzlab_core::mzrun`); 4. browser `prov` for all steps; 5. wasm and desktop routes; 6. preset apply; 7. Attributio decisions; 8. UI; 9. ESI+/ESI- merge; 10. exports; 11. e2e replicate.
