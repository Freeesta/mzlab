# Broad-peak bench (mzLab and MZmine)

Measures how much mzLab and MZmine lose on broad chromatographic peaks (FWHM 60-240 s) compared with narrow peaks. Tools in `tools/` only: no product changes. The metrics and the `bench/1` format are in `mzfinder-benchmark.md`; here the batch, the configurations and how to read the table.

## Synthetic batch
- **Generate**: `python3 tools/valida/sintetici_mzml.py FOLDER --seme 11` writes `blank_01.mzML`, `camp_01..03.mzML` and `verita.json` (37 rows, 4 files, ~70 MB; deterministic at a fixed seed, same bytes on every machine).
- **Compounds**: broad (a: Gaussian or EMG; b: double plateau with a saddle at 60 %; c: 150 s band with 1-2 narrow 8 s peaks within 3 ppm) on a gradient background over ~200 m/z channels; the files are ordered blank, then the samples.
- **Truth** (`verita.json`): per compound `ruolo` (broad/narrow), `rt`, `fwhm_s`, `rt_min`/`rt_max` (the central 99 % of the area) and `area_attesa` (list per file in intensity·min, 0 on the blank).
- The batch contains no laboratory data.

## MZmine 3.9.0 configurations
Two files in `tools/bench/parametri/mzmine/`, chosen with `--config`:
- `mzmine_default`: the documented flow (centroid mass, ADAP, "minimum search" resolver with chromatographic threshold 0.85, width up to 1 min, join aligner, gap filling), equal to `predefinita`.
- `mzmine_wide`: same flow, only the resolver changes: chromatographic threshold 0.60, search interval 0.5 min, width up to 8 min.
MZmine 4.10.90 reads the same two files through `oro_mzmine410.batch_xml`. 4.10.90 has no separate wavelet resolver (the modules are minimum search, noise amplitude, Savitzky-Golay): no wavelet variant. `tests/test_bench_mzmine_larghi.py` checks only the shape (JSON, XML, resolver values); MZmine does not run in the cloud.

## Run
mzLab, anywhere (needs cargo):
`python3 tools/bench/corri_mzlab.py BATCH_FOLDER OUTPUT --dataset SPAL`
MZmine on Mac (the program downloads only Linux JDKs: on Mac MZmine and Java are given by hand):
```
mkdir -p ~/mzmine39 && cd ~/mzmine39
curl -L -o mzmine.tar.bz2 https://conda.anaconda.org/bioconda/noarch/mzmine-3.9.0-hdfd78af_0.tar.bz2
shasum -a 256 mzmine.tar.bz2    # d596078684c7e23af7c7e350dded437ecbf6b2851aadc856c318c3cac2645504
mkdir mzmine && tar -xjf mzmine.tar.bz2 -C mzmine && chmod +x mzmine/bin/MZmine
curl -L -o jdk.tar.gz "https://api.adoptium.net/v3/binary/latest/20/ga/mac/aarch64/jdk/hotspot/normal/eclipse"   # Intel: x64
mkdir jdk && tar -xzf jdk.tar.gz -C jdk
cd -   # repository root
J=$(ls -d ~/mzmine39/jdk/*/Contents/Home)
for C in mzmine_default mzmine_wide; do
  python3 tools/bench/corri_mzmine.py BATCH_FOLDER OUTPUT --dataset SPAL --config $C --mzmine ~/mzmine39/mzmine --java-home $J
done
```
They write `OUTPUT/mzmine_SPAL_mzmine_default.json` and `..._mzmine_wide.json` (`bench/1`).

## Reading the table
`python3 tools/bench/confronto_larghi.py --verita BATCH_FOLDER/verita.json --tabelle "MZmine default=OUTPUT/mzmine_SPAL_mzmine_default.json" "MZmine wide=OUTPUT/mzmine_SPAL_mzmine_wide.json" "mzLab default=OUTPUT/mzlab_SPAL_predefinita.json"` prints one column per tool:
- **P1_narrow / P1_broad**: recall of the narrow and broad compounds (a broad band is found if |ΔRT| ≤ max(tolerance, 0.35·FWHM) or if the IoU of the intervals is ≥ 0.40). The gap between the two rows is the loss on smeared peaks.
- **P2_broad**: median relative area error on the broad compounds found (0 = exact; 1 = double or zero; above 1 = strong overestimate).
- **P3_split**: rows per band (mean) and bands split into more than one row: a narrow resolver cuts the band into fragments.
- **P5_fp**: rows that match no compound, as a number and as a fraction; with the background gradient a high value in a column says how much noise becomes a "peak".
Expected comparison: `mzmine_wide` should raise P1_broad and lower P3_split relative to `mzmine_default`, at the cost of more P5_fp; a column that does not move is a result, not an error.
