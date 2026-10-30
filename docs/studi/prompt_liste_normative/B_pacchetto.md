# B: Data release: first real build committed, `.mzsus` `listings` extension, the package in the site and in the app

Prompt ready for a DEV chat: copy everything after the line "---".

---

Package B of the study `docs/studi/LISTE_NORMATIVE.md`. Needs A merged and **Federico's decision D1** (commit the per-list CSVs to the public repository): ask the COO before starting if D1 is not recorded in the study §9.
Area files besides the common ones: `docs/agenti/sospetti.md`, `docs/agenti/librerie.md` (catalogue and downloads), `docs/agenti/wasm-tetti.md`, `docs/agenti/rust-desktop.md`.

## Goal
The lists reach the site and the app as one versioned package. The package is built in CI from the committed text with no network, downloaded on first use, and works offline after that.

## To do
1. **First real build.** Run `mzlab-cli normative build` with a full cache. If the PubChem files are needed and you are in the cloud, ask the COO: the local agent runs it. Commit:
   - `normative/lists/*.csv`;
   - `normative/provenance.json`;
   - `normative/unresolved.tsv`;
   - `normative/README.md`: licences and attributions per source, the sentence "Only the official publications are authentic", and how to rebuild.
   Excluded: ECHA Candidate List and Prop 65 rows (study §5.3). Add the NORMAN/NCBI/EU/EPA attributions to `LICENZE-TERZI.md`.
2. **`.mzsus` `listings` extension** (study §5.2, exactly that layout). In `crates/mzlab-feat/src/suspects.rs`:
   - `SRC_NORMATIVE = 8`;
   - `Suspects::listings() -> Option<&Listings>`, with lists, memberships per row, entries, full keys and names;
   - flag bit 0; `reader_min` stays 1.
   Old files and mzFinder are unaffected: a test opens a v1 file without the flag. Mirror it in the reference reader `tools/sospetti_riferimento.py` (reference only), and extend `crates/mzlab-feat/tests/suspects.rs` parity to the extension. Write the layout in `docs/agenti/sospetti.md` (rewrite the format section; no "now").
3. **Pack.** `mzlab-cli normative pack normative/ OUT/` writes:
   - `normative.mzsus`: the union of the screenable rows of every list, one row per skeleton, memberships sorted by catalogue order then `entry_ref`; deterministic bytes;
   - zlib pieces of 4 MiB with SHA-256;
   - `catalogo.json` (`normative/1`: lists with tags, version dates and coverage, raw size and SHA-256, attributions).
   Same pattern as `tools/genera_sospetti.py --pacchetto`, but native.
4. **Site.** In `pages.yml`, one step after the site build: `mzlab-cli normative pack normative/ site/static/normative/`, no network. Use a cache keyed on `hashFiles('normative/**')` if the build is slow. In the page, a small `normative-scarica.js` (`// @ts-check`, in a `tools/tipi/` project) downloads the pieces on first use into the Cache Storage `mzlab-libs`. It checks every SHA-256, decompresses with `DecompressionStream("deflate")` and hands the raw bytes to the engine. A second use is offline. "Check for updates" compares `catalogo.json`. Texts in `lang/it.js` and `lang/en.js`.
5. **App.** The desktop route reads the same package from the app data folder; first use fetches it from the site URL, or the user picks a folder. The CLI accepts the folder or the `.mzsus`. Parity test: the same package opened by the `.wasm` reader and the native reader gives the same `listings` JSON, byte for byte.
6. **Sizes.** Measure and write in the PR:
   - CSV total;
   - `.mzsus` raw and compressed;
   - the `.wasm` that hosts the reader, before and after.
   Caps never rise; a new module only if needed, with the COO's approval.

## Tests
- `cargo test -p mzlab-feat --test suspects` (extension + v1 compatibility);
- `cargo test -p mzlab-cli normative` (pack determinism on the invented fixture of A: two runs give the same bytes);
- `python3 tools/pre_pr.py`;
- one e2e `tests_e2e/e2e_normative_scarica.py`: the package from a built site, first download, reload offline, the SHA-256 error path with a corrupted piece; served by the test, never from the network. Run `python3 tools/build_site.py` first, and delete `site/` after.

## PR text
Title "Ship the regulatory lists as a versioned package for the site and the app". Body: the D1 decision referenced, the sizes, the list table with version dates and coverage, what is not shipped and why (ECHA, Prop 65, TSCA).

## Common rules (valid for the whole chat)
- You are a DEV chat of mzLab (repository `Freeesta/mzlab`). AGENTS.md is in your context: follow it. English only in code, files, commits, PR and messages. COO = `session_01NawSmneBkv8sosEHpoC8k4`: at the end ONE `send_message` (≤ 5 lines: PR number, 40-char SHA at green CI, what remains). Blocked or a choice between two behaviours: PR open with the explanation, a question to the COO with your recommendation, no merge. PR events: skill `steward`.
- Read ONLY: `docs/studi/LISTE_NORMATIVE.md` (the study: §3 identity, §4 design, §5 build and packaging, §6 matching, §7 wording; read by sections with `grep -n '^## '`), the area files named in this prompt, and code found with `python3 tools/indice.py` + `grep -n` or the `cercatore` subagent.
- Principles: a list match is a candidate with its evidence, never a verdict (AGENTS.md principle 1); no "detected", "present", "non-compliant", "exceeds". Every calculation and decision in Rust (`crates/`), with its test; Python only as reference for parity (`tools/*_riferimento.py`); JS only interface. No new dependencies in the product, no `unsafe`, no network at analysis time (CSP `connect-src 'self'`), no CompTox/ECHA/PubChem API calls from the site or the app at run time.
- Site = app (AGENTS.md, rule of 27/10/2026): every function exists in the `.wasm` and in the native app/CLI with a byte-parity test. `.wasm` caps never rise: measure before and after (`docs/agenti/wasm-tetti.md`); lists are data, never code.
- No lab data, no secrets, no surname of `nomi_vietati.txt`. Tests use tiny INVENTED fixtures (invented names, invented CAS-shaped strings that fail the CAS checksum, invented InChIKey-shaped strings); never a real downloaded list in `tests/`.
- Do not touch: `mzlab/web/teoria/`, `TP_Mine/` (Tabui), `.github/workflows/` except where this prompt says so, the `.wasm` caps, the `.mzsus` v1 layout (the extension is additive, `reader_min` stays 1).
- Flow: branch `claude/<topic>` from `origin/main`, PR ready for review, never merged from the chat (the COO squashes at green CI). Before pushing: `python3 tools/pre_pr.py`. Documents describe the current state: one line per new function in the area file (`docs/agenti/sospetti.md`, `docs/agenti/attributio.md` or a new `docs/agenti/normative.md` registered in `docs/agenti/INDICE.md`).
- Context above 200k tokens at the end of the PR: hand-over block to the COO (AGENTS.md §2) and stop.
