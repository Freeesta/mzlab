# C2b: importing official list files (ECHA Candidate List, Proposition 65) in the site and the app

Note for the COO and Federico, written while doing C2a (the «Regulatory lists» section of the Library window). Study: `LISTE_NORMATIVE.md`. Prompt: `prompt_liste_normative/C_screening.md`, item 5, «Import an official file…».

## What C2b has to do
The user picks the ECHA Candidate List (XLSX or CSV) or the OEHHA Proposition 65 list (CSV). The page runs the A readers `svhc` / `ca_prop65` and a local `resolve` against SusDat only, then packs the result into a private `.mzsus` with the `listings` extension. The site keeps that file in OPFS and the app in its folder of the libraries; it is never uploaded. It is labelled «imported by you on <date> from <file name>, SHA-256 …», and unresolved rows are counted and shown. Site and app must give the same bytes.

## Gap 1: SusDat by CAS is not in the browser
`resolve` finds a CAS number in SusDat (`resolve_src.rs`). The browser has no such table. The mzFinder suspect index (`.mzsus` v1, `tools/genera_sospetti.py`) is keyed by InChIKey-14 with mass and formula; it has no CAS column, and it is downloaded only by mzFinder.

**Recommendation: yes, a sidecar.** `genera_sospetti.py` (which already reads SusDat with a pinned SHA-256) writes a small CAS → InChIKey-14 table next to the index, as a downloadable package with its own `catalogo.json` (same format and checks as `normative/1`).
One row per CAS that maps to a single skeleton: the CAS as `u64` digits and the 14 letters. A CAS with two skeletons is left out, so it stays `unresolved`, as `resolve` already does. Size to measure; an estimate for about 100 k CAS numbers is 2 MB raw and about 1 MB compressed. It is downloaded only on the first import and is never part of the first page load (`tests/test_peso_avvio.py`).

## Gap 2: there is no XLSX reader in Rust
**Recommendation: CSV only first, XLSX later.** ECHA also offers the Candidate List as CSV, and Proposition 65 is CSV only. The page says «save it as CSV» when the file is an XLSX. A later XLSX reader needs only the zip directory, `xl/sharedStrings.xml` and the first sheet. `flate2` is already a dependency of `mzlab-core`, so no new crate is needed, but it is new code to keep small and test well; it belongs in its own PR.

## Gap 3: the readers, `resolve` and the packer are native-only
`crates/mzlab-cli/src/normative/` is `cfg(not(target_family = "wasm"))`: fetching, PubChem and the cache are native-only by design. The import needs only the parts that read bytes:
- `csvrd.rs`, `entry.rs`, and the `svhc` / `ca_prop65` branch of `readers.rs`;
- the SusDat step of `resolve.rs`, fed by the sidecar of gap 1 instead of the SusDat file;
- the row building of `build_rows.rs` and the writer of `pack.rs`.

**Recommendation:**
- Move these pure parts into `mzlab-feat` (a `normative` module without network, std only). `mzlab-cli` keeps fetching, PubChem, DSSTox and the cache, and calls the moved code, so the CLI output does not change; a byte-for-byte test pins that.
- Native: one entry point, `import_official(reader, csv_bytes, sidecar_bytes, label) -> .mzsus bytes`, called by the app through a new route `normative/import-official`.
- Site: the same function in a `.wasm`. The natural host is `attributio-all`, which already holds `flow_regulatory` and has about 52 KB of headroom (1 483 361 / 1 536 000). Measure the growth first.
- If the growth does not fit, the alternative is a lazy `normative` `.wasm` of its own. That needs a cap in `wasm-tetti.md` and a job in `.github/workflows/`, so it waits for Federico's explicit yes, as the COO decided.
- Parity test: the same invented CSV and the same invented sidecar give the same `.mzsus` bytes from `mzlab-cli`, the app route and the `.wasm` under node (the pattern of `tests/test_normative_screen_wasm.py`).
- Fixtures: invented CSVs shaped like the ECHA and OEHHA exports, CAS numbers that fail the checksum, invented InChIKeys; never a real downloaded list in the repository.

## After the import
The imported `.mzsus` carries a `normative/1` manifest entry like the packed lists (`id`, `tags.status`, `tags.version_date`, `tags.instrument`, `tags.citation`, `coverage`, `csv_sha256`, `retrieved`). The C2a panel then lists it, and the report shows it, with no change; the steps take it as one more file on their `suspects` / `lists` port.

## Decisions needed
1. The CAS → InChIKey-14 sidecar from SusDat: yes or no.
2. CSV only in C2b, XLSX in a later PR: yes or no.
3. Where the `.wasm` code goes: in `attributio-all` if it fits, otherwise a new lazy module (Federico).
