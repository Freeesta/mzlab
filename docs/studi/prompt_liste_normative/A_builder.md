# A: Builder of the regulatory lists (`mzlab-cli normative build`): official sources → per-list A1+ CSV + provenance

Prompt ready for a DEV chat: copy everything after the line "---".

---

Package A of the study "Current EU and US regulatory chemical lists in mzLab" (`docs/studi/LISTE_NORMATIVE.md`). It is **RG-04 plus the list half of RG-06** of `docs/studi/ATTRIBUTIO.md` §6: tell the COO at the start that you own them, so that no other chat opens them.
Area files besides the common ones: `docs/agenti/attributio.md` (the `reglists/` lines), `docs/agenti/sospetti.md`, `docs/agenti/cli.md`.

## Goal
A native, deterministic, offline-reproducible builder. It reads the official downloads of the lists in §2 of the study and writes:
- one CSV per list (schema A1 + the five columns of study §4);
- `normative/provenance.json`.
Nothing runs at analysis time.

## To do
1. **`Fetch` trait** in `mzlab-cli` (study §5.1; RG-06 design):
   - disk cache `DIR/<sha256 of URL>` plus a sidecar with URL, retrieval date and SHA-256;
   - 5 requests/s per host, exponential retry;
   - a fake `Fetch` for the tests.
   With a warm cache the build makes no request and is byte-identical. The clock is read only when a file is first fetched, and it is stored in the sidecar.
2. **Pins**: a committed `normative/sources.json`. Per list it holds:
   - id, URLs (Cellar consolidated CELEX, eCFR versioner URL with its date, EPA page, Pesticides API v3.0, NORMAN Zenodo 15741861);
   - the expected SHA-256 once fetched (`null` the first time; `--update-pins` fills it);
   - the tags of study §4: jurisdiction, instrument, citation, annex, status, in_force_from, applies_from, version_date, licence, attribution.
3. **Readers**, one per source shape. They are the RG-04 names `eu_water`, `eu_chem`, `eu_ppp`, `voci`, plus new ones: `us_ecfr` (eCFR XML tables), `us_epa_html` (CCL 5/6 and UCMR 6 HTML tables), and `svhc` and `ca_prop65` for the user-import route of package C. They turn their source into entries `{entry_ref, name, cas, ec, dtxsid, attr, group_entry, kind}`, with `kind` ∈ structure | group | uvcb | inorganic.
   - **Lists of v1:** every list of study §2 marked in force or proposal/draft with an official XHTML/XML/JSON/HTML form.
   - **Phase 2, not built here:** the lists marked phase 2, TSCA, CosIng, 37/2010 Table 1 if its XHTML table is not parseable (then `voci`, a hand-transcribed entries file with the act cited per row, reviewed in the PR).
   - **No ECHA site downloads** (ECHA terms, study §5.3): REACH XIV/XVII and CLP Annex VI are read from the OJ consolidated texts on Cellar. The `svhc` reader exists only for the user's own file (package C).
4. **`resolve`**, in the order of study §3:
   1. `law`;
   2. `susdat`: the pinned SusDat CSV, StdInChIKey and MS_Ready_InChIKey;
   3. `pubchem`: `CID-Identifiers.tsv.gz` + `CID-Parent.gz`, streamed and filtered to the listed CAS, then PUG REST properties for the CIDs found, all through the `Fetch` cache (never `CID-InChI-Key.gz`);
   4. `dsstox`: only where a list gives a DTXSID;
   5. `curated` (`normative/overrides.tsv`: `list_id;entry_ref;cas;inchikey;reason`);
   6. otherwise `unresolved`.
   A CAS that maps to two skeletons is `unresolved` with both in the log. The match key is the MS-ready first block (`inchikey_skeleton`). `form` is salt | anion | parent | as_listed.
   - **Large PubChem files:** do not download them from the cloud without telling the COO (> 90 MB). The local agent or a CI job may.
5. **Output**:
   - `normative/lists/<LIST_ID>.csv`: A1 + `entry_ref;ec;dtxsid;form;status`. Canonical: rows sorted by (`entry_ref`, `inchikey_skeleton`, `name`); `read_list` of `reglists/schema.rs` must read it unchanged.
   - `normative/provenance.json` (`normative-prov/1`): per list the tags, the input URLs with SHA-256 and retrieval date, and the coverage `{entries, screenable, groups, uvcb, inorganic, unresolved}`.
   - `normative/unresolved.tsv`: what a curator must look at.
6. **Catalogue**: add the new ids and the groups `US federal` and `US state` to `reglists/catalog.rs` (and its reference in `tools/attributio_tox_riferimento.py` if the parity test compares the catalogue). Keep the existing ids and order.
7. **CLI**: `mzlab-cli normative build --cache DIR [--only ID…] [--update-pins] --out normative/`. Exit codes and `err.normative.*` keys as in `cli.md`. A source that fails keeps the last good CSV and marks the list `stale: true` in the provenance. A list is never emptied silently.
8. **Data commit:** do NOT commit `normative/lists/` in this PR. Package B commits the first real build after Federico's decision D1. Here only the code, `sources.json`, an empty `overrides.tsv` with its header, and the fixtures.

## Tests (no network in CI)
- `crates/mzlab-cli/tests/normative_build.rs` with a fake `Fetch` serving tiny invented fixtures under `crates/mzlab-cli/tests/data/normative/`:
  - a 4-row Cellar-like XHTML table (one group entry, one anion CAS);
  - a 3-section eCFR-like XML;
  - a 2-page Pesticides-like JSON;
  - a 5-row SusDat-like CSV;
  - a PubChem-like `CID-Identifiers`/`CID-Parent` pair.
- Assertions: canonical CSV bytes; anion → parent collapse; group entries counted and not screenable; disagreement → `unresolved`; a warm-cache rebuild is byte-identical; a changed fixture without `--update-pins` stops with the pin error; `read_list` reads the output.
- `cargo test -p mzlab-cli normative`, `cargo test -p mzlab-feat --features attributio --test attributio_reglists` (unchanged), `python3 tools/pre_pr.py`.

## PR text
Title "Build the EU and US regulatory lists from the official sources (normative build)". Body: what is built, the source table with status per list, what is phase 2, and "RG-04 + list part of RG-06". Doc: new `docs/agenti/normative.md` (builder section), registered in `docs/agenti/INDICE.md`; one line in `attributio.md` pointing to it; the RG-04 row of `ATTRIBUTIO.md` set to its new state.

## Common rules (valid for the whole chat)
- You are a DEV chat of mzLab (repository `Freeesta/mzlab`). AGENTS.md is in your context: follow it. English only in code, files, commits, PR and messages. COO = `session_01NawSmneBkv8sosEHpoC8k4`: at the end ONE `send_message` (≤ 5 lines: PR number, 40-char SHA at green CI, what remains). Blocked or a choice between two behaviours: PR open with the explanation, a question to the COO with your recommendation, no merge. PR events: skill `steward`.
- Read ONLY: `docs/studi/LISTE_NORMATIVE.md` (the study: §3 identity, §4 design, §5 build and packaging, §6 matching, §7 wording; read by sections with `grep -n '^## '`), the area files named in this prompt, and code found with `python3 tools/indice.py` + `grep -n` or the `cercatore` subagent.
- Principles: a list match is a candidate with its evidence, never a verdict (AGENTS.md principle 1); no "detected", "present", "non-compliant", "exceeds". Every calculation and decision in Rust (`crates/`), with its test; Python only as reference for parity (`tools/*_riferimento.py`); JS only interface. No new dependencies in the product, no `unsafe`, no network at analysis time (CSP `connect-src 'self'`), no CompTox/ECHA/PubChem API calls from the site or the app at run time.
- Site = app (AGENTS.md, rule of 27/10/2026): every function exists in the `.wasm` and in the native app/CLI with a byte-parity test. `.wasm` caps never rise: measure before and after (`docs/agenti/wasm-tetti.md`); lists are data, never code.
- No lab data, no secrets, no surname of `nomi_vietati.txt`. Tests use tiny INVENTED fixtures (invented names, invented CAS-shaped strings that fail the CAS checksum, invented InChIKey-shaped strings); never a real downloaded list in `tests/`.
- Do not touch: `mzlab/web/teoria/`, `TP_Mine/` (Tabui), `.github/workflows/` except where this prompt says so, the `.wasm` caps, the `.mzsus` v1 layout (the extension is additive, `reader_min` stays 1).
- Flow: branch `claude/<topic>` from `origin/main`, PR ready for review, never merged from the chat (the COO squashes at green CI). Before pushing: `python3 tools/pre_pr.py`. Documents describe the current state: one line per new function in the area file (`docs/agenti/sospetti.md`, `docs/agenti/attributio.md` or a new `docs/agenti/normative.md` registered in `docs/agenti/INDICE.md`).
- Context above 200k tokens at the end of the PR: hand-over block to the COO (AGENTS.md §2) and stop.
