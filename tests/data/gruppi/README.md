# Synthetic Multi-Condition Dataset (Groups and Kinetics)

Synthetic test dataset for the reaction-condition comparison module of Tabui,
designed to validate kinetic differentiation and the attribution of photolytic
and photocatalytic pathways from low-resolution LC-MS data (Q1 full scan, nominal mass, ESI+).

The data are entirely synthetic: no real, proprietary or confidential data are present.
The parent compound is the public standard Bentazone.

## 1. File naming and structure

The dataset is made of exactly **38 mzML files**, laid out as:
- **Kinetic runs (36 files)**: `<Condition>_<time>_<replicate>.mzML`
  - 6 experimental conditions:
    1. `Air`: photocatalysis in an air atmosphere
    2. `O2`: photocatalysis in pure oxygen
    3. `N2`: photolysis in an inert nitrogen atmosphere
    4. `Ar`: photolysis in an inert argon atmosphere
    5. `He`: photolysis in an inert helium atmosphere
    6. `Reference`: stability control in the dark
  - 3 time points: `t0` (time zero), `t5` (5 minutes), `t10` (10 minutes)
  - 2 replicates per time: `r1`, `r2` (deterministic experimental variability +/- 3-5%)
- **Analytical controls (2 files)**:
  - `bianco.mzML`: solvent blank (eluent / matrix)
  - `standard.mzML`: analytical standard of the parent compound alone at time t0

Each file is recorded over the chromatographic range 1.0 - 5.0 minutes with 80 zlib-compressed
centroid scans and is about 59-60 KB, for a total dataset size
of about 2.2 MB, well under the 3 MB limit.

## 2. Compound profile

| Compound | Analytical role | Nominal m/z | Theoretical RT (min) | FWHM (min) | Kinetic behaviour |
|---|---|---|---|---|---|
| **Parent** | Bentazone (substrate) | 241 | 3.00 | 0.14 | Decreases in the active conditions (100% -> 50% -> 15%); stable in Reference (100% -> 98% -> 96%); absent in the blank. |
| **TP1** | Mono-hydroxylated intermediate | 257 | 2.20 | 0.13 | Bell-shaped kinetics in all active conditions (t0=0, t5=max ~2.0e5, t10=drop ~6.0e4); weak trace in Reference; absent in blank and standard. |
| **TP2** | Photo-oxygenation | 273 | 1.80 | 0.12 | Present ONLY in O2 and Air (t0=0, t5=1.0-1.8e5, t10=1.5-2.5e5); totally absent in the inert conditions (N2, Ar, He), Reference and blank. |
| **TP3** | Anaerobic photolysis | 225 | 2.60 | 0.13 | Present ONLY in N2, Ar, He (t0=0, t5=1.2e5, t10=2.0e5); totally absent in the presence of oxygen (O2, Air), Reference and blank. |
| **TP4** | Contaminant / plasticiser | 279 | 4.20 | 0.15 | Present at constant area (~1.2e5) in ALL files including blank and standard. Must be discarded by the blank filter. |
| **TP5** | In-Source Fragment (ISF) | 197 | 3.00 | 0.14 | Fragment formed in the ESI source. Always co-eluting with the parent (identical RT) at a height of ~18% of the parent. Absent in the blank. |

## 3. Validation criteria for Tabui

The dataset lets Tabui check the following algorithms:

1. **Blank filter (blank subtraction)**:
   - TP4 (m/z 279, RT 4.20) is detected both in the kinetic samples and in `bianco.mzML`.
   - The blank filter must exclude TP4 from the list of real transformation products.
2. **Deconvolution of In-Source Fragments (ISF)**:
   - TP5 (m/z 197) co-elutes 100% with the Parent (m/z 241, RT 3.00) in all runs and
     keeps a constant area ratio (~18%).
   - The grouping algorithm by ion families (`mode: family`) assigns it to the
     parent's family without treating it as an independent degradation product.
3. **Classification by environmental condition**:
   - Aerobic conditions (`Air`, `O2`): selective formation of TP2 (m/z 273). The yield in pure O2
     is significantly higher than in Air.
   - Anaerobic / inert conditions (`N2`, `Ar`, `He`): selective formation of TP3 (m/z 225).
     No trace of TP2.
   - Dark reference condition (`Reference`): parent degradation rate below
     5% at t10, confirming that the disappearance of the parent in the other conditions is photochemical.
4. **Formation and consumption kinetics**:
   - TP1 (m/z 257) shows the bell-shaped course typical of a consecutive reaction
     intermediate (Parent -> TP1 -> secondary products/mineralisation).

## 4. Ground truth: attesi.json

The file `attesi.json` holds the structured analytical truth for automatic checking:
- `metadata`: chemical parameters of the parent, list of conditions, times, replicates and controls.
- `compounds`: definition of each compound (formula, m/z, RT, analytical role, description).
- `files`: dictionary indexed by file name with experimental metadata and, for each
  compound, the presence state (`present: true/false`), expected retention time, height
  and area (in counts * s).

## 5. Check with mzlab-cli

To extract the feature table from one or more files of the dataset:

```bash
# Feature extraction in family mode (default: groups TP5 with the parent)
cargo run --release -p mzlab-cli -- features tests/data/gruppi/standard.mzML --out /tmp/feat.csv

# Single-ion feature extraction (shows both the parent 241 and the ISF 197 separately)
echo '{"mode":"ion"}' > /tmp/params.json
cargo run --release -p mzlab-cli -- features tests/data/gruppi/standard.mzML --params /tmp/params.json --out /tmp/feat_ion.csv
```
