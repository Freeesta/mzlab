# `.mzsus` v1 suspect index (mzFinder, offline) and its `listings` extension (normative lists)
List of known compounds sorted by monoisotopic mass, for the H1 hypotheses of mzFinder as a LIST of suspects: a mass hit is a level 5/4 candidate with its evidence, never a name given to the feature. No network or external API at runtime. Never in the repository nor in the first load of the site: only mzFinder downloads it.

## Sources (all CC BY 4.0, versions and SHA-256 fixed in `tools/genera_sospetti.py`, `SOURCES`)
- NORMAN SusDat (Zenodo 15741861, NORMAN-SLE-S0.0.6.1): columns `StdInChIKey`, `Molecular_Formula`, `Monoiso_Mass`.
- PubChemLite for Exposomics (Zenodo 22953038, 3.3.0): `FirstBlock`, `MolecularFormula`, `MonoisotopicMass`, `PubMed_Count`, `Patent_Count`. The CSV (200 MB) is downloaded and discarded.
- Transformations in PubChem (Zenodo 19150892, 0.2.3): `PubChem_all_transformations.csv` + `Transformations_CID_Info_all.csv`.
- A file different from the fixed fingerprint stops the build. BioTransformer, enviPath and CTS never enter the index (only imported lists).

## Build
`python3 tools/genera_sospetti.py [--source DIR] --out FILE [--package DIR]`. Rows merged by InChIKey-14 (A-Z only): mass and formula from the first source (SusDat, PubChemLite, Transformations), sources in union, counts at the maximum, lowest reaction id. Rows without a valid key, mass or formula are discarded (counted in the manifest: `letti` / `usati`). Stable order (mass, InChIKey-14): same sources = same file byte for byte. With `--package` it also writes `DIR/cas/` (`mzlab-cas/1` sidecar of the SusDat CAS numbers and `catalogo.json`; format in `docs/agenti/normative.md`).
`--package`: the raw file compressed with zlib (level 9) in 4 MiB pieces, each with its SHA-256, plus `catalogo.json` (format `suspects/1`: sizes, SHA-256 of the raw file, sources with licence and DOI, attribution). The browser decompresses with `DecompressionStream("deflate")` and passes the raw bytes to the reader. The Pages CI puts it in `static/mzfinder/sospetti/`.

## Raw file (little-endian, every section starts at a multiple of 8)
Header (32 bytes): magic `MZSU`; u16 version (1); u16 minimum reader version (1); u32 rows `n`; u32 formulae `nf`; u32 reactions `nr`; u32 manifest bytes; u32 flags (bit 0 = `listings` extension, otherwise 0); u32 reserved (0). Then follow, each padded to 8:
| Section | Content |
|---|---|
| manifest | UTF-8 JSON: `format`, `rows`, `attribution`, `sources[]` (id, name, version, licence, doi, url, institution, bit, sha256 of the files, read, used) |
| masses | `n` × f64 (monoisotopic Da, increasing) |
| keys | `n` × 14 ASCII bytes, InChIKey-14 |
| formulae | `n` × u32, index into the formula table |
| sources | `n` × u8, bits: 1 SusDat, 2 PubChemLite, 4 Transformations, 8 normative lists |
| literature | `n` × u16 (PubMed, saturated at 65535) |
| patents | `n` × u16 (saturated) |
| reactions | `nr` × (u32 row index, u32 id), sorted by row; id = row (from 1) of the first reaction in the Transformations file in which the compound appears; absent = none |
| formula offsets | `nf+1` × u32 into the blob |
| formula blob | ASCII formulae, sorted, without separators |

## Reader
`read_suspects(path_or_bytes)` -> `Suspects` (`.manifest`, `.n`, `.hit(i)`, `.cerca(mass, ppm)`): window `|m − mass| ≤ mass·ppm·1e-6` with binary search, rows in order of mass and key. A file without the magic or newer than the reader knows is refused (`ValueError`). The test is `tests/test_sospetti.py` (synthetic mini-set of invented keys).

## Rust reader (`crates/mzlab-feat/src/suspects.rs`)
`Suspects::open(&[u8])` reads the raw file (already decompressed) into `Hit` rows sorted by mass (`mass`, `key` 14 bytes, `formula` index for `Suspects::formula`, `sources` `SRC_*` bits, `pubmed`, `patents`, `reaction`); `search(mass, ppm) -> &[Hit]` is a binary search with the same window as the Python reader. Errors: `Corrupt` and `Version`; no dependency beyond the crate.
The test `crates/mzlab-feat/tests/suspects.rs` compares the answers with `tools/suspects_reference.py` (skipped without python3; `MZLAB_GOLDEN_STRICT=1` fails) and measures the search on 10⁶ synthetic rows.
## Flow step `screen.suspects@1` (`crates/mzlab-feat/src/flow_screen_suspects/`)
Searches by neutral mass and adduct (the adducts and ppm of `attributio::reglists::screen_types`) the features of the HR alignment in one or more of the user's suspect files: `.mzsus` (`Suspects::search`) or CSV (`name`, `inchikey` or `smiles`, `formula` or `mass`, `rt`, `fragments` separated by `;`). Key = first block of the InChIKey; results with the same formula are a group of isomers, never a single name.
Output `screen/1` (`docs/schemi/screen-1.json`): `suspect_files` (name, SHA-256, manifest of the `.mzsus` or "user CSV"), `hits[]` per feature × adduct × formula with `candidates[]`, `memberships[]` (`list`, `entry_ref`, `form`, `group`, `attr`; `[]` without the `listings` extension), level 5 or 4 (formula in `formulas/1`). Same bytes in the CLI and in the `.wasm` `table` (`flow_screen_suspects`).
With a normative package (`.mzsus` with `listings`) the parameters `lists` (ids separated by commas; empty = all) and `include` (`proposal`, `draft`, `reference`; empty = only `in_force` and `adopted_not_applicable`) make the mask (`mzlab_feat::regulatory::Package`): a row that no kept list names is not a candidate; the name and the full key come from `listings`; `suspect_files[].lists` carries the manifest of each kept list (version, SHA-256, coverage), so the mask enters `params_sha256` and `prov/1`.
Imported lists (port `imported_lists`, `normative.md`) are additional sources after the suspect files, with an empty mask; their memberships are added to those of the package on the same candidate.
Test: `crates/mzlab-cli/src/flow_screen_suspects_reg_tests.rs` (invented package `crates/mzlab-cli/tests/data/normative_screen/`).

## `listings` extension (normative lists, flag bit 0; `reader_min` stays 1)
After the formula blob, each section padded to 8: header (16 bytes: u32 lists `nl`, u32 memberships `m`, u32 entries `ne`, u32 bytes of the entry blob); full keys (`n` × 27 ASCII bytes, the InChIKey of the representative listed form, spaces if missing); names (`n+1` × u32 offsets, then UTF-8 blob); membership offsets (`n+1` × u32); memberships (`m` × [u16 list index, u8 form, u8 bit, u32 entry index]); entries (`ne+1` × u32 offsets, then a blob of one compact JSON object per entry: `{"ref","name","cas","ec","attr"}`).
Form: 0 not stated, 1 `as_listed`, 2 `parent`, 3 `salt`, 4 `anion`. Bits: 1 group member, 2 isomer indicated, 4 the listed CAS is an anion or a salt. The list index points to `normative/1.lists` of the manifest (catalogue order); the lists are labelled there (`tags`, `coverage`, `csv_sha256`). The manifest also has `fonti` with the single source `normative` (bit 8).
`Suspects::listings() -> Option<&Listings>` (`None` without the flag; `list_count`, `full_key`, `name`, `memberships`, `entry`, `entry_count`) and `Suspects::listings_json()` (the whole extension as a single JSON: same bytes in the `.wasm` `normative_listings` and in `mzlab-cli db normative listings`). v1 files without the flag and mzFinder do not change. The Python reference reader is `tools/suspects_reference.py` (`write_listings`, `read_listings`); the test `crates/mzlab-feat/tests/suspects.rs` compares `listings_json` with its output.
The builder is `mzlab-cli db normative pack` (`docs/agenti/normative.md`): one row per skeleton, mass and formula from the CSV, memberships sorted by (list, `entry_ref`), deterministic bytes.
