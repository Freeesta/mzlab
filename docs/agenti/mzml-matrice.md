# mzML conformance matrix

Fixtures: `tools/mzml_variants.py OUT_DIR` (20 synthetic files, invented numbers); test: `crates/mzlab-cli/src/mzml_variants_tests.rs` (`canali`, `features`, `validate` on every variant; skipped without python3 + numpy). Numpress fixtures are written with `MzmlWriter` (`Opzioni::numpress`), not from Python. Reading = `mzlab-core` (`scantable` table) for plain mzML, the `mzml-numpress` reader (`aperti`) for Numpress: `lettori-aperti.md`.

| Variant | Fixture | Outcome (CLI) |
|---|---|---|
| plain mzML, indexed (`indexedmzML`), absurd offsets | `plain_*`, `indexed`, `bigoffsets` | opens; the scan table reads the heads at the index offsets (an index that does not match the file is ignored and the file is walked), the engine (`mzml::read`) reads sequentially |
| float 32/64 bit × uncompressed/zlib | `plain_32_*`, `plain_64_*` | opens |
| `defaultArrayLength` `0` or missing with non-empty zlib arrays | `robustness_mzml.rs` | before inflating, the declared points for the bytes kept must fit in `budget::TARGET_BYTES`; without a length the array is inflated at most up to `budget::undeclared_inflate` (32 times the compressed bytes, at least 1 MB): beyond that it is `error.memoryBudget` (the size, not damage); a broken or incomplete zlib is `err.file.zlib` |
| CRLF, UTF-8 BOM | `crlf`, `bom` | opens |
| MS2, MS3, MS4 | `ms3_4`, `ms3_only_idx` | opens; the type proposed by `canali` is `lc-ms2-mirato` |
| profile / centroids | `profile` | opens |
| ion mobility (`MS:1002893` array) | `mobility` | opens; the extra array is not used |
| `referenceableParamGroup`, `scanSettings` | `rpg`, `scansettings` | opens; the scan table rereads the group parameters where the reference sits (`spechead`) |
| TIC, BPC, SRM, SIM, absorption chromatograms | `chroms_all` | opens; proposed type `misto` |
| MS-Numpress linear/pic/slof, with and without zlib | `Opzioni::numpress`, `open_numpress*.rs` | opens from the `aperti` reader (site, app, CLI) |
| empty file, not XML | `empty`, `notxml` | `err.file.notMzml` |
| broken base64 | `badb64` | `err.file.base64` |
| gzip (`.mzML.gz`) | `gz.mzML.gz`, `open_gz.rs` | opens from the `mzml-gz` reader (`open/mzml_gz.rs`): decompresses in memory up to `Limiti::byte` (`error.fixedLimit` beyond), then `mzml-numpress` reads it; a gzip truncated after a few spectra opens with the ones it has (`mancanti` = `declared` minus the rows), a broken gzip or one truncated before the first byte is `err.file.zlib`; on the site it is inflated as a stream on OPFS (`gz-flusso.js`, `lettori-aperti.md`) |
| truncated file | `truncated` | opens with the spectra it has; `Table::declared` (the `<spectrumList>` count, read even beyond the run userParams (`metodo/1`, `head_read::run_end`); tests `mzml_run_window.rs`, `test_parziali_parita.py`) minus the rows read gives `stats.missing` and the loading row says "N scans missing" (`conv.row.missing`); the CLI writes `warn.file.truncated` on stderr (also for files read by `aperti`: `.mzML.gz`, Numpress; on the site the module returns the count with the conversion and `lr.js` adds it to the index one); `validate` flags it INVALID |
| mzMLb, imzML | – | not read |

Not tested here: the site `.wasm` engine and the desktop app on the same fixtures (same core code, but no e2e per variant).

## Measurements (large synthetic file, 700 scans × 100 000 points, profile)
`tools/mzml_grande.py` writes the zlib file (893 MB); `mzlab-cli convert x.mzML.gz --compression smallest|standard` makes its twins; `mzlab-cli canali FILE` builds the index (native CLI, release, 4 CPU). Times and memory peak: Standard 1003 MB, 20.5 s, 109 MB; Smallest (zlib + Numpress) 180 MB, 26.9 s, 67 MB; writing Standard 43 s / 940 MB, Smallest 35 s / 916 MB (the `.gz` is inflated in memory). Numpress is lossy; the data is synthetic (invented noise and peaks), the real ratio depends on the file.

Head of a spectrum or chromatogram (`spechead`): a single reader, used by the scan table and by the engine (`mzml::parse`, `mzml/build.rs`), gives the terms in file order (spectrum, `scanList`, scans), the precursors in file order (window, first selected ion, activation), the products (never a precursor) and the array terms; a `referenceableParamGroupRef` repeats the parameters of the group where it sits, the first term of an accession wins, names have no namespace prefix, CDATA counts as text, time in any unit (`UO`) becomes minutes.
Engine (`mzml::parse`): an array with data but no numeric type is `err.file.mzmlPrecision`, two arrays of different lengths `err.file.mzmlLengths`, lists that declare items without showing any `err.file.mzmlNoItems`; the precursor is the immediate one (`spechead::immediate`), Q1/Q3 of an SRM come from the windows. `Run.losses` (`parse_counted`) gives spectra declared by `spectrumList` and read: truncated file `warn.file.truncated`, wrong count in a closed file `warn.file.mzmlFewer`. Tests: `mzml/tests_head.rs`.
Limits of `spechead`: a list holds at most 4096 terms and an element 64 stages (a hostile file does not fill memory). Tests: `spechead/tests.rs`, `scantable/tests_variants.rs`.

Writing (`mzmlwrite/text.rs`): every text read from a file goes through `esc` (`& < > "` and characters outside the XML 1.0 `Char` production become U+FFFD), also the CV term names.
A generic term with a text from the file (`GENERIC`: `MS:1000031` and the vendor models, `MS:1000531` software, `MS:1000008`/`MS:1000443`/`MS:1000026` source type, analyzer, detector) keeps the CV name; the text goes into a `userParam` (`instrument model` for the models, otherwise the CV name), reread by `mzmlhead/params.rs`, `scantable/analyzer.rs` and `open/mzml_header.rs`.
`tests/test_mzml_secondo_parser.py` reads with lxml (libxml2, no recovery) the `convert` mzML of every synthetic family, in Standard and Smallest.

Validator (`mzml_validate`): first a strict XML 1.0 pass (`wellformed.rs`: characters outside `Char`, malformed or repeated attributes, `<` in a value, unknown entities, content outside the root; it stops at the first, code `xml`). Every mzML written by a conversion goes through `check_written`: an error is `err.file.mzmlInvalid` and the file is not kept (`convert.mzml` of the CLI, `nat2mzml` of the app, "Download .mzML" of the site with the `valida` module: `CONV.valid`).
