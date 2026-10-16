# Dati di Riferimento Quantomeccanici xTB (GFN2-xTB)

Questo archivio contiene i calcoli di riferimento quantomeccanici semiempirici **GFN2-xTB** eseguiti sulle 20 molecole di prova descritte in `docs/studi/CC_MOTORE_NEL_BROWSER.md` (§6), comprendenti i 10 composti modello della bozza iniziale e 10 inquinanti organici persistenti / micropollutanti tipici dei prodotti di trasformazione (TP).

I dati servono come banco di prova e controllo di parità numerica per l'implementazione Rust/WASM in `crates/mzlab-chem`.

---

## 1. Ambiente di Calcolo e Versioni

- **xTB**: versione `6.7.1` (commit `edcfbbe`, macOS arm64)
- **RDKit**: `2026.09.1`
- **Python**: `3.11.17`
- **Ambiente di calcolo**: Conda Forge (`/Users/federicocristaudo/Desktop/gemini_cc/conda_env`)
- **Script generatore**: `/Users/federicocristaudo/Desktop/gemini_cc/script/calcola_xtb.py` (collocato all'esterno del repository per rispettare la regola di assenza di script Python nel repo).

---

## 2. Protocollo di Generazione e Comandi

Per ciascuna molecola:

1. **Geometria 3D iniziale**:
   - Generazione conformero con RDKit `ETKDGv3` con parametro deterministico `randomSeed = 42`.
   - Minimizzazione energetica con campo di forze `MMFF94` (`maxIters = 500`).
2. **File coordinate `.xyz`**:
   - Riga 1: numero totale di atomi ($N_{\text{tot}} = N_{\text{heavy}} + N_{\text{H}}$).
   - Riga 2: SMILES canonico RDKit.
   - Righe successive: **rigorosamente prima tutti gli atomi pesanti** nell'ordine esatto dello SMILES (indici $0 \dots N_{\text{heavy}}-1$), seguiti dagli atomi di idrogeno.
3. **Calcolo in acqua implicita (ALPB water)**:
   ```bash
   xtb <nome>.xyz --gfn 2 --alpb water --vfukui --chrg 0
   ```
   Produce:
   - `<nome>_xtb.out`: log completo di xTB.
   - `<nome>_fukui.csv`: indici di Fukui condensati per gli atomi pesanti ($0 \dots N_{\text{heavy}}-1$).
4. **Calcolo in fase gassosa**:
   ```bash
   xtb <nome>.xyz --gfn 2 --vfukui --chrg 0
   ```
   Produce:
   - `<nome>_gas_xtb.out`: log completo di xTB.
   - `<nome>_gas_fukui.csv`: indici di Fukui condensati per gli atomi pesanti in fase gassosa.

---

## 3. Struttura dei File

Per ciascuna delle 20 molecole sono presenti 5 file:
- `<nome>.xyz`: coordinate cartesiane (Å).
- `<nome>_xtb.out`: output di calcolo xTB ALPB water.
- `<nome>_fukui.csv`: valori di $f(+)$, $f(-)$, $f(0)$ in acqua.
- `<nome>_gas_xtb.out`: output di calcolo xTB fase gassosa.
- `<nome>_gas_fukui.csv`: valori di $f(+)$, $f(-)$, $f(0)$ in fase gassosa.

Inoltre:
- `riepilogo.csv`: tabella sintetica con tempi di calcolo, convergenza SCF e primo sito $f(-)$ tra i carboni insaturi legati ad H, mediato sulle classi di simmetria topologica `CanonicalRankAtoms`.
- `LEGGIMI.md`: questa documentazione tecnica.

### Formato dei file CSV Fukui (`<nome>_fukui.csv` e `<nome>_gas_fukui.csv`)
Colonne:
- `smiles`: SMILES molecolare canonico.
- `atom`: indice numerico 0-based dell'atomo pesante nell'ordine dello SMILES.
- `quantity`: descrittore di Fukui condensato (`f_plus`, `f_minus`, `f_zero`).
- `value`: valore numerico (3 decimali).
- `method`: metodo impiegato (`GFN2-xTB, ALPB water` oppure `GFN2-xTB, gas`).
- `source`: file di log di origine.

### Formato di `riepilogo.csv`
Colonne:
- `molecola`: nome identificativo del composto.
- `cid`: PubChem Compound ID.
- `n_atomi_pesanti`: numero di atomi pesanti.
- `tempo_xtb_water_s`: tempo CPU/wall in secondi (misurato con `perf_counter`).
- `tempo_xtb_gas_s`: tempo CPU/wall in secondi in fase gassosa.
- `convergente_water`: convergenza SCF in acqua (`true`/`false`).
- `convergente_gas`: convergenza SCF in gas (`true`/`false`).
- `sito_f_minus_water`: indice/i dell'atomo pesante del sito a maggior $f(-)$ (C insaturo con H, mediato su classi di simmetria).
- `sito_f_minus_gas`: corrispondente sito in fase gassosa.
- `note`: annotazioni chimiche e metodologiche.

---

## 4. Elenco delle 20 Molecole

| # | Molecola | PubChem CID | SMILES PubChem ufficiale | SMILES RDKit utilizzato | Atomi pesanti |
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

*Nota su imidacloprid*: Il PubChem CID ufficiale per l'imidacloprid neutro è `86287518`. Il numero `86343` citato talvolta corrisponde all'identificativo ChemSpider (CSID 86343).

---

## 5. Osservazioni Chimiche e Metodologiche

1. **Convergenza**: Il 100% dei calcoli (20/20 in acqua ALPB, 20/20 in fase gassosa) ha soddisfatto i criteri SCF (`convergence criteria satisfied`) e si è concluso con codice di uscita 0 (`normal termination of xtb`).
2. **Ruolo critico della solvatazione ALPB water**:
   - Nella **carbamazepina**, il calcolo in acqua individua correttamente il doppio legame olefinico centrale come sito preferenziale ($f(-)_{\text{water}} = 0.030$ su C10, C11). In fase gassosa, al contrario, due carboni aromatici dell'anello ($6, 15$) scavalcano il doppio legame ($f(-)_{\text{gas}} = 0.028$ su $6, 15$ contro $0.026$ su $10, 11$). Ciò conferma sperimentalmente la necessità del modello implicito ALPB per la predizione regiochimica.
   - Nel **diclofenac**, la presenza dell'acqua sposta il primo sito di attacco elettrofilo su $f(-)$ dall'anello fenilacetico (sito 6 in gas) alla posizione 15 dell'anello 2,6-dicloroanilina in acqua ($0.033$).
3. **Composti privi di C insaturi legati ad H**:
   - L'**atrazina** ha un anello 1,3,5-triazinico completamente sostituito (C-Cl, C-NHEt, C-NHiPr). Non possiede alcun atomo di carbonio insaturo o aromatico legato ad idrogeno. In `riepilogo.csv` il sito è opportunamente riportato come `N/A`. Nei processi di degradazione ossidativa (AOP), la reazione procede prevalentemente per dealchilazione delle catene etil/isopropilamminiche o declorazione, piuttosto che addizione elettrofila su C-H insaturo.
4. **Siti degeneri o quasi-degeneri**:
   - Nel **paracetamolo**, le due posizioni aromatiche orto al gruppo $-\text{OH}$ (atomi 6, 9, media $f(-) = 0.0335$) e orto al gruppo acetammidico (atomi 5, 10, media $f(-) = 0.0318$) presentano uno scarto minimo ($\Delta f(-) \approx 0.0017 < 0.005$), confermando che il solo descrittore $f(-)$ dello stato fondamentale le colloca a pari merito operativo («reattività simile»).
   - Nel **bisfenolo A**, i quattro carboni orto ai gruppi fenolici (atomi 5, 8, 12, 15) hanno simmetria equivalente e massimizzano $f(-)$ ($0.022$).
   - Nella **caffeina**, la posizione C8 dell'anello imidazolico (atomo 7, unico C-H insaturo) ha $f(-) = 0.017$ in acqua, coerente con l'idrossilazione che porta a acido 1,3,7-trimetilurico.
