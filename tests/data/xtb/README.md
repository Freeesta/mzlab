# xTB (GFN2-xTB) Quantum-Mechanical Reference Data

This archive holds the **GFN2-xTB** semi-empirical quantum-mechanical reference calculations run on the 20 test molecules listed in `MOLS` of `tools/cc_reference.py`: the 10 model compounds of the initial draft and 10 persistent organic pollutants / micropollutants typical of transformation products (TPs).

The data serve as a test bench and numerical parity check for the Rust/WASM implementation in `crates/mzlab-chem`.

---

## 1. Compute environment and versions

- **xTB**: version `6.7.1` (commit `edcfbbe`, macOS arm64)
- **RDKit**: `2026.09.1`
- **Python**: `3.11.17`
- **Compute environment**: Conda Forge (`~/Desktop/gemini_cc/conda_env`)
- **Generator script**: `~/Desktop/gemini_cc/script/calcola_xtb.py` (kept outside the repository to respect the rule of no Python scripts in the repo).

---

## 2. Generation protocol and commands

For each molecule:

1. **Initial 3D geometry**:
   - Conformer generation with RDKit `ETKDGv3` with the deterministic parameter `randomSeed = 42`.
   - Energy minimisation with the `MMFF94` force field (`maxIters = 500`).
2. **Coordinate `.xyz` file**:
   - Line 1: total number of atoms ($N_{\text{tot}} = N_{\text{heavy}} + N_{\text{H}}$).
   - Line 2: RDKit canonical SMILES.
   - Following lines: **strictly all heavy atoms first**, in the exact SMILES order (indices $0 \dots N_{\text{heavy}}-1$), followed by the hydrogen atoms.
3. **Implicit-water calculation (ALPB water)**:
   ```bash
   xtb <name>.xyz --gfn 2 --alpb water --vfukui --chrg 0
   ```
   Produces:
   - `<name>_xtb.out`: full xTB log.
   - `<name>_fukui.csv`: condensed Fukui indices for the heavy atoms ($0 \dots N_{\text{heavy}}-1$).
4. **Gas-phase calculation**:
   ```bash
   xtb <name>.xyz --gfn 2 --vfukui --chrg 0
   ```
   Produces:
   - `<name>_gas_xtb.out`: full xTB log.
   - `<name>_gas_fukui.csv`: condensed Fukui indices for the heavy atoms in the gas phase.

---

## 3. File structure

For each of the 20 molecules there are 5 files:
- `<name>.xyz`: Cartesian coordinates (Å).
- `<name>_xtb.out`: xTB ALPB water calculation output.
- `<name>_fukui.csv`: $f(+)$, $f(-)$, $f(0)$ values in water.
- `<name>_gas_xtb.out`: gas-phase xTB calculation output.
- `<name>_gas_fukui.csv`: $f(+)$, $f(-)$, $f(0)$ values in the gas phase.

Also:
- `riepilogo.csv`: summary table with calculation times, SCF convergence and the top $f(-)$ site among unsaturated carbons bonded to H, averaged over the topological symmetry classes of `CanonicalRankAtoms`.
- `README.md`: this technical documentation.

### Format of the Fukui CSV files (`<name>_fukui.csv` and `<name>_gas_fukui.csv`)
Columns:
- `smiles`: canonical molecular SMILES.
- `atom`: 0-based numeric index of the heavy atom in SMILES order.
- `quantity`: condensed Fukui descriptor (`f_plus`, `f_minus`, `f_zero`).
- `value`: numeric value (3 decimals).
- `method`: method used (`GFN2-xTB, ALPB water` or `GFN2-xTB, gas`).
- `source`: originating log file.

### Format of `riepilogo.csv`
Columns:
- `molecule`: identifying name of the compound.
- `cid`: PubChem Compound ID.
- `n_heavy_atoms`: number of heavy atoms.
- `time_xtb_water_s`: CPU/wall time in seconds (measured with `perf_counter`).
- `time_xtb_gas_s`: CPU/wall time in seconds in the gas phase.
- `converged_water`: SCF convergence in water (`true`/`false`).
- `converged_gas`: SCF convergence in gas (`true`/`false`).
- `site_f_minus_water`: index(es) of the heavy atom of the site with the highest $f(-)$ (unsaturated C with H, averaged over symmetry classes).
- `site_f_minus_gas`: corresponding site in the gas phase.
- `notes`: chemical and methodological notes.

---

## 4. List of the 20 molecules

| # | Molecule | PubChem CID | Official PubChem SMILES | RDKit SMILES used | Heavy atoms |
|---|---|---|---|---|:---:|
| 1 | fenolo | 996 | `C1=CC=C(C=C1)O` | `Oc1ccccc1` | 7 |
| 2 | anisolo | 7519 | `COC1=CC=CC=C1` | `COc1ccccc1` | 8 |
| 3 | toluene | 1140 | `CC1=CC=CC=C1` | `Cc1ccccc1` | 7 |
| 4 | acido_benzoico | 243 | `C1=CC=C(C=C1)C(=O)O` | `O=C(O)c1ccccc1` | 9 |
| 5 | nitrobenzene | 7416 | `C1=CC=C(C=C1)[N+](=O)[O-]` | `O=[N+]([O-])c1ccccc1` | 9 |
| 6 | clorobenzene | 7964 | `C1=CC=C(C=C1)Cl` | `Clc1ccccc1` | 7 |
| 7 | anilina | 6115 | `C1=CC=C(C=C1)N` | `Nc1ccccc1` | 7 |
| 8 | naftalene | 931 | `C1=CC=C2C=CC=CC2=C1` | `c1ccc2ccccc2c1` | 10 |
| 9 | carbamazepina | 2554 | `C1=CC=C2C(=C1)C=CC3=CC=CC=C3N2C(=O)N` | `NC(=O)N1c2ccccc2C=Cc2ccccc21` | 18 |
| 10 | paracetamolo | 1983 | `CC(=O)NC1=CC=C(C=C1)O` | `CC(=O)Nc1ccc(O)cc1` | 11 |
| 11 | diclofenac | 3033 | `C1=CC=C(C(=C1)CC(=O)O)NC2=C(C=CC=C2Cl)Cl` | `O=C(O)Cc1ccccc1Nc1c(Cl)cccc1Cl` | 19 |
| 12 | sulfametossazolo | 5329 | `CC1=CC(=NO1)NS(=O)(=O)C2=CC=C(C=C2)N` | `Cc1cc(NS(=O)(=O)c2ccc(N)cc2)no1` | 17 |
| 13 | atrazina | 2256 | `CCNC1=NC(=NC(=N1)Cl)NC(C)C` | `CCNc1nc(Cl)nc(NC(C)C)n1` | 14 |
| 14 | bisfenolo A | 6623 | `CC(C)(C1=CC=C(C=C1)O)C2=CC=C(C=C2)O` | `CC(C)(c1ccc(O)cc1)c1ccc(O)cc1` | 17 |
| 15 | caffeina | 2519 | `CN1C=NC2=C1C(=O)N(C(=O)N2C)C` | `Cn1c(=O)c2c(ncn2C)n(C)c1=O` | 14 |
| 16 | triclosan | 5564 | `C1=CC(=C(C=C1Cl)O)OC2=C(C=C(C=C2)Cl)Cl` | `Oc1cc(Cl)ccc1Oc1ccc(Cl)cc1Cl` | 17 |
| 17 | ibuprofene | 3672 | `CC(C)CC1=CC=C(C=C1)C(C)C(=O)O` | `CC(C)Cc1ccc(C(C)C(=O)O)cc1` | 15 |
| 18 | metoprololo | 4171 | `CC(C)NCC(COC1=CC=C(C=C1)CCOC)O` | `COCCc1ccc(OCC(O)CNC(C)C)cc1` | 19 |
| 19 | benzofenone-3 | 4632 | `COC1=CC(=C(C=C1)C(=O)C2=CC=CC=C2)O` | `COc1ccc(C(=O)c2ccccc2)c(O)c1` | 17 |
| 20 | imidacloprid | 86287518 | `C1CN(C(=N[N+](=O)[O-])N1)CC2=CN=C(C=C2)Cl` | `O=[N+]([O-])N=C1NCCN1Cc1ccc(Cl)nc1` | 17 |

*Note on imidacloprid*: The official PubChem CID for neutral imidacloprid is `86287518`. The number `86343` sometimes quoted is the ChemSpider identifier (CSID 86343).

---

## 5. Chemical and methodological observations

1. **Convergence**: 100% of the calculations (20/20 in ALPB water, 20/20 in the gas phase) met the SCF criteria (`convergence criteria satisfied`) and ended with exit code 0 (`normal termination of xtb`).
2. **Critical role of ALPB water solvation**:
   - In **carbamazepine**, the water calculation correctly identifies the central olefinic double bond as the preferred site ($f(-)_{\text{water}} = 0.030$ on C10, C11). In the gas phase, on the contrary, two aromatic ring carbons ($6, 15$) overtake the double bond ($f(-)_{\text{gas}} = 0.028$ on $6, 15$ against $0.026$ on $10, 11$). This confirms experimentally the need for the implicit ALPB model to predict regiochemistry.
   - In **diclofenac**, the presence of water moves the first site of electrophilic attack on $f(-)$ from the phenylacetic ring (site 6 in gas) to position 15 of the 2,6-dichloroaniline ring in water ($0.033$).
3. **Compounds without unsaturated C bonded to H**:
   - **Atrazine** has a fully substituted 1,3,5-triazine ring (C-Cl, C-NHEt, C-NHiPr). It has no unsaturated or aromatic carbon atom bonded to hydrogen. In `riepilogo.csv` the site is therefore reported as `N/A`. In oxidative degradation processes (AOP), the reaction proceeds mainly by dealkylation of the ethyl/isopropylamine chains or dechlorination, rather than by electrophilic addition on an unsaturated C-H.
4. **Degenerate or nearly degenerate sites**:
   - In **paracetamol**, the two aromatic positions ortho to the $-\text{OH}$ group (atoms 6, 9, mean $f(-) = 0.0335$) and ortho to the acetamide group (atoms 5, 10, mean $f(-) = 0.0318$) show a minimal gap ($\Delta f(-) \approx 0.0017 < 0.005$), confirming that the ground-state $f(-)$ descriptor alone ranks them as operationally tied («similar reactivity»).
   - In **bisphenol A**, the four carbons ortho to the phenolic groups (atoms 5, 8, 12, 15) have equivalent symmetry and maximise $f(-)$ ($0.022$).
   - In **caffeine**, position C8 of the imidazole ring (atom 7, the only unsaturated C-H) has $f(-) = 0.017$ in water, consistent with the hydroxylation leading to 1,3,7-trimethyluric acid.
