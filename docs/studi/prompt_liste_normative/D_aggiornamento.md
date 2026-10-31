# D: Update routine of the regulatory lists (`normative check`, monthly run, update PR)

Prompt ready for a DEV chat: copy everything after the line "---".

---

Package D of the study `docs/studi/LISTE_NORMATIVE.md`. Needs A and B merged.
Area files besides the common ones: `docs/agenti/normative.md` (written by A and B), `docs/agenti/cli.md`.

## Goal
A person or a scheduled agent learns, at least monthly, which lists changed upstream. One command rebuilds them, and the result is a reviewable PR. Nothing is updated at analysis time and nothing is updated silently.

## To do
1. **`mzlab-cli normative check --cache DIR`** compares `normative/sources.json` with what is current upstream. Per list it reports `same | changed | unreachable | new version available` with the evidence:
   - Cellar: newest consolidated CELEX and its date (SPARQL `https://publications.europa.eu/webapi/rdf/sparql`);
   - eCFR: newest version date of each part (`/api/versioner/v1/versions/title-NN.json`);
   - EU Pesticides API: record count and newest change date;
   - EPA CCL/UCMR pages: page date and table hash;
   - NORMAN SusDat: newest Zenodo version of the concept DOI;
   - watched events: a new Implementing Decision of the surface-water Watch List, a POPs delegated act, UCMR 6 / CCL 6 final, PFAS NPDWR final rules, a new Prop 65 list date (Prop 65 is user-import only: report it, do not build it).
   It writes `normative-check/1` JSON and a short Markdown summary. The exit code is 0 when nothing changed, 3 when something changed, 2 on error.
2. **`normative build --update-pins --only <changed ids>`** (from A), then `normative diff OLD NEW`: per list, entries added, removed and re-resolved, and the coverage before and after. The output is Markdown, ready to be the PR body.
3. **The routine, written in `docs/agenti/normative.md` § Update:**
   1. monthly, and on the event dates (Candidate List in January and June: tell Federico, since it is user-import only);
   2. run check;
   3. if anything changed: branch `claude/normative-update-<date>`, build, commit `normative/`, open a PR with the diff as body and the line "Only the official publications are authentic";
   4. a human reviews `overrides.tsv` and `unresolved.tsv` changes;
   5. the COO merges;
   6. the site picks the new package up at the next deploy, and the Library panel's "Check for updates" sees it.
   A list whose source vanished keeps its last good version with `stale: true` and a line in the PR.
4. **Status transitions** are data, never code:
   - a proposal that becomes law, e.g. UCMR 6 final: change `status` and the citation in `sources.json`;
   - a law repealed (e.g. Watch List 2025/439 when its successor arrives): keep the old list with `status: repealed` and the repeal citation, unchecked by default, so old reports stay reproducible.
5. **Optional, only if the COO approves it:** a scheduled task (fresh session, monthly) that runs the routine and opens the PR. It never merges.

## Tests
- `cargo test -p mzlab-cli normative_check`: a fake `Fetch` with an invented SPARQL answer, an eCFR versions JSON and an EPA page, in three states (same, changed, unreachable). Assertions: exit codes, the summary, and that `check` never writes `normative/`.
- `normative diff` on two invented builds.
- `python3 tools/pre_pr.py`.

## PR text
Title "Check and update the regulatory lists from their official sources". Body: the routine, the exit codes, and one example summary from the fixture.

## Common rules (valid for the whole chat)
- You are a DEV chat of mzLab (repository `Freeesta/mzlab`). AGENTS.md is in your context: follow it. English only in code, files, commits, PR and messages. COO = `session_01NawSmneBkv8sosEHpoC8k4`: at the end ONE `send_message` (≤ 5 lines: PR number, 40-char SHA at green CI, what remains). Blocked or a choice between two behaviours: PR open with the explanation, a question to the COO with your recommendation, no merge. PR events: skill `steward`.
- Read ONLY: `docs/studi/LISTE_NORMATIVE.md` (the study: §3 identity, §4 design, §5 build and packaging, §6 matching, §7 wording; read by sections with `grep -n '^## '`), the area files named in this prompt, and code found with `python3 tools/indice.py` + `grep -n` or the `cercatore` subagent.
- Principles: a list match is a candidate with its evidence, never a verdict (AGENTS.md principle 1); no "detected", "present", "non-compliant", "exceeds". Every calculation and decision in Rust (`crates/`), with its test; Python only as reference for parity (`tools/*_riferimento.py`); JS only interface. No new dependencies in the product, no `unsafe`, no network at analysis time (CSP `connect-src 'self'`), no CompTox/ECHA/PubChem API calls from the site or the app at run time.
- Site = app (AGENTS.md, rule of 27/10/2026): every function exists in the `.wasm` and in the native app/CLI with a byte-parity test. `.wasm` caps never rise: measure before and after (`docs/agenti/wasm-tetti.md`); lists are data, never code.
- No lab data, no secrets, no surname of `nomi_vietati.txt`. Tests use tiny INVENTED fixtures (invented names, invented CAS-shaped strings that fail the CAS checksum, invented InChIKey-shaped strings); never a real downloaded list in `tests/`.
- Do not touch: `mzlab/web/teoria/`, `TP_Mine/` (Tabui), `.github/workflows/` except where this prompt says so, the `.wasm` caps, the `.mzsus` v1 layout (the extension is additive, `reader_min` stays 1).
- Flow: branch `claude/<topic>` from `origin/main`, PR ready for review, never merged from the chat (the COO squashes at green CI). Before pushing: `python3 tools/pre_pr.py`. Documents describe the current state: one line per new function in the area file (`docs/agenti/sospetti.md`, `docs/agenti/attributio.md` or a new `docs/agenti/normative.md` registered in `docs/agenti/INDICE.md`).
- Context above 200k tokens at the end of the PR: hand-over block to the COO (AGENTS.md §2) and stop.
