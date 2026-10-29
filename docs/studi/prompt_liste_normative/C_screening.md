# C: Matching: `screen.suspects` with the regulatory lists, `annot.regulatory` executor, Library panel, report wording, user import of ECHA/OEHHA files

Prompt ready for a DEV chat: copy everything after the line "---".

---

Package C of the study `docs/studi/LISTE_NORMATIVE.md`. Needs B's package format. It can start with A's fixture packed by `normative pack`. If the NTS PR3 chat (`screen.suspects`) is open, ask the COO who owns the step file. Never two chats on the same `flow_*.rs`.
Area files besides the common ones: `docs/agenti/sospetti.md`, `docs/agenti/attributio.md` (`reglists/` B1, B2), `docs/agenti/rust-flow.md`, `docs/agenti/cli.md`, `docs/agenti/cli-passi.md`, `docs/agenti/rust-step.md`, `docs/agenti/librerie.md` (panel and masks).

## Goal
A feature table (HR) can be screened against the lists in force, both by mass/formula and by candidate structure. Every hit carries its memberships, isomer group and provenance, and no verdict. Site and app give the same bytes.

## To do
1. **Mass path, `screen.suspects`** (study §6.2):
   - **New step or own file:** if the step does not exist yet, create it in its own `flow_screen_suspects.rs` (only the registration line goes in `flow.rs`). Descriptor `step/1`, requires `hr`.
   - **Params** with `--set`: `ppm` (5), `adducts` (from a `liste/1` adducts list, default B2's five), `lists` (mask; default: status `in_force`), `include` (`proposal`, `draft`, `reference`; off), `suspects` (path of one or more `.mzsus`: the regulatory package, SusDat, a user `.mzsus`).
   - **Output `screen/1`:** per feature × adduct × formula, the `candidates[]` with skeleton, full key, name and `memberships[]` (list id, `entry_ref`, `form`, group flag), `delta_ppm`, and Level 5, or Level 4 when `formula.ms1` supports the formula.
   - **Isomers:** every listed structure with the formula appears, grouped. Never a single name when more than one exists.
   - **Schema:** write it as `docs/schemi/screen-1.json`. Remove the step from `BUCHI` only once its executor exists.
2. **Structure path, `annot.regulatory` / `list_membership`.** Give the step its native executor (it is in `BUCHI` today). It reads the identities' InChIKeys of the `annot/1` rows and the package's memberships through B1 `match_campaign` (full key → "InChIKey", first block → "first block", flagged). It writes the `regulatory` field of `records.rs`. `suspect_screen` and `ms2_confirm` stay as they are (Attributio campaigns).
3. **Provenance.** The `catalogo.json` version of every list used, its SHA-256 and the mask enter `params_sha256` and `prov/1` `lists` (type `regulatory`). `corsa/1` shows them.
4. **Web executors** for both steps in the `.wasm` that hosts the other NTS/Attributio executors. Measure before and after; the caps never rise. Parity test: native and web give identical `screen/1` and `annot/1` bytes on the same fixture.
5. **Library panel.** A "Regulatory lists" section with one row per list:
   - label, jurisdiction, status, version date, coverage ("70 entries: 61 screened, 6 groups, 3 inorganic"), checkbox;
   - proposals and drafts unchecked;
   - "Check for updates" (from B);
   - "Import an official file…" for the ECHA Candidate List (XLSX/CSV) and Prop 65 (CSV). The import runs the A readers `svhc` / `ca_prop65` **plus a local `resolve` against SusDat only**: the site has no PubChem files. Unresolved rows are counted and shown. The result is a private `.mzsus` with the `listings` extension, stored in OPFS (desktop: the app data folder), never uploaded, labelled "imported by you on <date> from <file name>, SHA-256 …".
   Accessibility per AGENTS.md principle 6: keyboard, `aria-label`, CSS variables, 24×24 targets. Tour: `tour.js` and `e2e_tour.py` stay valid.
6. **Wording** exactly as in study §7. Column "Listed in", the cell texts, the report header "Screened against: …" with version, retrieval date and SHA-256 per list, the coverage line, and the disclaimer in the report and in the panel footer. Forbidden words: "detected", "present", "non-compliant", "exceeds", "violation", "contaminated". Add a test that greps the new `lang/` keys and the report template for them. The legal limit text (`attr`) is shown as quoted law text and never compared with any measured value.
7. **Report.** The `report/1` section "Regulatory lists" (table + header + coverage + disclaimer), the same from the CLI and the page.

## Tests
- `crates/mzlab-cli/src/flow_screen_suspects_tests.rs`. Invented 6-row package: two isomers with the same formula, one salt-collapsed row, one row on 3 lists, one proposal-only row. Invented feature table. Assertions:
  - the isomer group is complete;
  - the proposal row is absent by default and present with `include=proposal`;
  - Δppm is right;
  - the provenance hash changes when the mask changes.
- `flow_rule_tests.rs` green with the steps removed from `BUCHI`.
- Byte parity web/native.
- `e2e_normative_screen.py`: panel, import of an invented ECHA-like XLSX, a run, the report, axe on the panel.
- Run `python3 tools/pre_pr.py` and `python3 tools/verifica.py --cambiati`.

## PR text
Title "Screen feature tables against the EU and US regulatory lists (candidates, never verdicts)". Body: the wording table, the sizes, the parity proof, and what stays for phase 2 (substructure test against structural definitions such as the PFAS definitions of REACH XVII entry 82, TSCA 40 CFR 705.3 and CCL 5).

## Common rules (valid for the whole chat)
- You are a DEV chat of mzLab (repository `Freeesta/mzlab`). AGENTS.md is in your context: follow it. English only in code, files, commits, PR and messages. COO = `session_01NawSmneBkv8sosEHpoC8k4`: at the end ONE `send_message` (≤ 5 lines: PR number, 40-char SHA at green CI, what remains). Blocked or a choice between two behaviours: PR open with the explanation, a question to the COO with your recommendation, no merge. PR events: skill `steward`.
- Read ONLY: `docs/studi/LISTE_NORMATIVE.md` (the study: §3 identity, §4 design, §5 build and packaging, §6 matching, §7 wording; read by sections with `grep -n '^## '`), the area files named in this prompt, and code found with `python3 tools/indice.py` + `grep -n` or the `cercatore` subagent.
- Principles: a list match is a candidate with its evidence, never a verdict (AGENTS.md principle 1); no "detected", "present", "non-compliant", "exceeds". Every calculation and decision in Rust (`crates/`), with its test; Python only as reference for parity (`tools/*_riferimento.py`); JS only interface. No new dependencies in the product, no `unsafe`, no network at analysis time (CSP `connect-src 'self'`), no CompTox/ECHA/PubChem API calls from the site or the app at run time.
- Site = app (AGENTS.md, rule of 27/10/2026): every function exists in the `.wasm` and in the native app/CLI with a byte-parity test. `.wasm` caps never rise: measure before and after (`docs/agenti/wasm-tetti.md`); lists are data, never code.
- No lab data, no secrets, no surname of `nomi_vietati.txt`. Tests use tiny INVENTED fixtures (invented names, invented CAS-shaped strings that fail the CAS checksum, invented InChIKey-shaped strings); never a real downloaded list in `tests/`.
- Do not touch: `mzlab/web/teoria/`, `TP_Mine/` (Tabui), `.github/workflows/` except where this prompt says so, the `.wasm` caps, the `.mzsus` v1 layout (the extension is additive, `reader_min` stays 1).
- Flow: branch `claude/<topic>` from `origin/main`, PR ready for review, never merged from the chat (the COO squashes at green CI). Before pushing: `python3 tools/pre_pr.py`. Documents describe the current state: one line per new function in the area file (`docs/agenti/sospetti.md`, `docs/agenti/attributio.md` or a new `docs/agenti/normative.md` registered in `docs/agenti/INDICE.md`).
- Context above 200k tokens at the end of the PR: hand-over block to the COO (AGENTS.md §2) and stop.
