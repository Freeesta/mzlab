# W-3 checks on the Rosina files (local Mac session only)

The Rosina files are private; CI covers the same properties on synthetic data (`docs/agenti/rust-step.md`). Run from the repository root with `MZLAB_DATI` pointing at `mzlab-dati`; `F` is a Rosina `.raw` (or `.wiff`, `.D`) file and `W` an empty folder.

1. Convert with the flow runner (indexed form): write `W/p.json` = `{"format":"mzflow/2","inputs":[{"name":"F","type":"raw"}],"steps":[{"id":"c","step":"convert.mzml@1","in":{"raw":"file:F"}}]}`, copy `F` next to it, then `cargo run --release -p mzlab-cli --example flow -- W/p.json W/out 1`. Expect `W/out/c-0.mzML` starting `<?xml`, then `<indexedmzML`, and `W/out/prov.json` without `error`.
2. Same bytes twice: run the command again into `W/out2`; `sha256sum W/out/c-0.mzML W/out2/c-0.mzML` must be equal.
3. Offsets: every `<offset idRef="..">N</offset>` must point at `<spectrum ` or `<chromatogram `: `python3 -I - W/out/c-0.mzML` with a script that reads the file as bytes and checks `b[N:].startswith(...)` for each offset, and that `b[indexListOffset:]` starts with `<indexList`.
4. Checksum: `head -c $(( $(grep -aob '<fileChecksum>' c-0.mzML | cut -d: -f1) + 14 )) c-0.mzML | sha1sum` must equal the text inside `<fileChecksum>`.
5. Schema: validate `c-0.mzML` with `mzML1.1.2_idx.xsd` (`xmllint --noout --schema mzML1.1.2_idx.xsd c-0.mzML`) and, where installed, OpenMS `FileInfo -v -in c-0.mzML` and the HUPO-PSI semantic validator.
6. Read back: `cargo run --release -p mzlab-cli -- features W/out/c-0.mzML` against `... -- features F` (the direct path): same rows and areas.
7. Same chain against the old writer: record the area differences of study §6.2 point 1 (`raw::to_mzml` against `scrivi`), and send the numbers.
