# W-3 checks on the Rosina files (local Mac session only)

The Rosina files are private; CI covers the same properties on synthetic data (`docs/agenti/rust-step.md`). Run from the repository root with `MZLAB_DATI` pointing at `mzlab-dati`; `F` is a Rosina `.raw` (or `.wiff`, `.D`) file and `W` an empty folder.

0. Traps: the input name in the plan must carry the extension (`F.raw`, not `F`: otherwise `err.file.notAccepted`), and `W/out` must already exist (otherwise `err.flow.io`). `xmllint` needs `--huge` on files with a very long instrument-method userParam.
1. Convert with the flow runner (indexed form): write `W/p.json` = `{"format":"mzflow/2","inputs":[{"name":"F","type":"raw"}],"steps":[{"id":"c","step":"convert.mzml@1","in":{"raw":"file:F"}}]}`, copy `F` next to it, then `cargo run --release -p mzlab-cli --example flow -- W/p.json W/out 1`. Expect `W/out/c-0.mzML` starting `<?xml`, then `<indexedmzML`, and `W/out/prov.json` without `error`.
2. Same bytes twice: run the command again into `W/out2`; `sha256sum W/out/c-0.mzML W/out2/c-0.mzML` must be equal.
3. Offsets: every `<offset idRef="..">N</offset>` must point at `<spectrum ` or `<chromatogram `: `python3 -I - W/out/c-0.mzML` with a script that reads the file as bytes and checks `b[N:].startswith(...)` for each offset, and that `b[indexListOffset:]` starts with `<indexList`.
4. Checksum: `head -c $(( $(grep -aob '<fileChecksum>' c-0.mzML | cut -d: -f1) + 14 )) c-0.mzML | sha1sum` must equal the text inside `<fileChecksum>`.
5. Schema: CI runs this on a synthetic file (`mzmlwrite/tests_xsd.rs`). On a real file validate `c-0.mzML` with `mzML1.1.2_idx.xsd` (`xmllint --noout --schema mzML1.1.2_idx.xsd c-0.mzML`) and, where installed, OpenMS `FileInfo -v -in c-0.mzML` and the HUPO-PSI semantic validator.
6. Read back: `cargo run --release -p mzlab-cli -- features W/out/c-0.mzML` against `... -- features F` (the direct path): same rows and areas.
7. Same chain against the old writer: record the area differences of study §6.2 point 1 (`raw::to_mzml` against `scrivi`), and send the numbers.

## Result on a real Thermo Q Exactive file (200616_AC_05_BR_1.raw, 52.9 MB)
convert.mzml@1: 25 962 591 B, `<indexedmzML`, no error, 0.8 s, RSS 177 MB; two runs give the same SHA-256; 3362 offsets, 0 wrong; `indexListOffset` and SHA-1 checksum correct; features on the mzML and on the raw file: 1301 rows each with the same columns. XSD, OpenMS `FileInfo` and the PSI validator were not available on the Mac: not run. Against `raw::to_mzml`: 1299 against 1301 rows, areas identical on the 1299 matched, 2 extra rows and none lost (cause not investigated: both rows sit within 5e-5 s of the detector threshold; m/z 262.87762 RT 0.0443 and m/z 275.06121 RT 1.9561).

## Schema and semantic checks on synthetic files (cloud)
- XSD `mzML1.1.2_idx.xsd`: CI (`mzmlwrite/tests_xsd.rs`, `xmllint`) validates an indexed file with an empty header and with a full one, with and without processing steps. An empty `sourceFileList` or `componentList` was invalid; the writer now leaves them out.
- OpenMS semantic check (`pyopenms` 3.6.0, `MzMLFile().isSemanticallyValid`, installed in a scratch folder, not in CI): `base peak intensity` (MS:1000505) had no unit, now `MS:1000131 number of detector counts`. Still reported on the synthetic header only: no instrument model term in `instrumentConfiguration`, and a `sourceFile` without the three required terms (file format, native ID format, SHA-1). The readers fill these in; to confirm on the real file, run step 5 with `isSemanticallyValid` on `c-0.mzML`.
- Real file (Mac, `c-0.mzML`, 26 227 952 B, `isSemanticallyValid`), before: not valid, 3361 messages: 3359 × `MS:1000512 filter string` under `/mzML/run/spectrumList/spectrum`, 2 × `MS:1000813` named «total wavelength chromatogram».
- After (cloud, synthetic file; to confirm on the Mac): `filter string` is written inside `<scan>`, and the UV chromatograms use `MS:1000812 absorption chromatogram` (the PSI-MS OBO defines MS:1000813 as `emission chromatogram`, light emitted; a UV/PDA trace is light absorbed), so 0 messages are expected; the readers accept both accessions. Test: `filter_string_sits_in_the_scan_and_uv_is_an_absorption_chromatogram`, and the XSD file now has a filter string and a UV chromatogram. `.wasm` sciex before wasm-opt: +21 B (1086796 → 1086817), limit 328192 unchanged.
- HUPO-PSI Java semantic validator: not run (the release download is blocked from the cloud).

## The 2 extra feature rows (Mac session; no Thermo file in the cloud)
`raw::to_mzml` (`opentfraw::write_mzml`, vendor centroids) and `scrivi_indicizzato` (the reader's scans) are two writers, so the spectra they write can differ before any feature is detected. With `F.raw` the Q Exactive file, `W` an empty folder and the converted file from step 1 (`W/out/c-0.mzML`):
1. `cargo run --release -p mzlab-core --example raw2mzml -- F.raw W/old.mzML`; then `cargo run --release -p mzlab-cli -- features W/old.mzML --out W/old.csv` and the same on `W/out/c-0.mzML` (`new.csv`): expect 1299 and 1301 rows.
2. Join the two CSV on (m/z within 5 ppm, RT within 0.05 min) and send the 2 rows of `new.csv` with no partner: m/z, RT, area, height, number of points.
3. For each of the 2 rows, list the MS1 scans of the window (RT ± 0.2 min) in both mzML: scan count, number of peaks per scan, and the intensity of the peak nearest to the row's m/z. Expect either extra or missing peaks near the intensity threshold, or a different scan count; say which.
