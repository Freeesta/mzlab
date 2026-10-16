# Controllo indipendente dei numeri del motore di mzLab (T1)

## 1. Obiettivo e metodo

Questo rapporto verifica in modo indipendente i calcoli numerici fondamentali del motore di mzLab confrontandoli con librerie scientifiche di riferimento in Python (**RDKit 2026.09.1**, **Pyteomics 5.0.1**, **PyOpenMS 3.6.0**, **NumPy 2.4.6**) su **30 composti pubblici** rappresentativi (fenolo, anisolo, toluene, acido benzoico, nitrobenzene, clorobenzene, anilina, naftalene, carbamazepina, paracetamolo, caffeina, atrazina, ibuprofene, diclofenac, sulfametossazolo, trimetoprim, metoprololo, propranololo, atenololo, venlafaxina, ciprofloxacina, tetraciclina, bisfenolo A, acido salicilico, naprossene, ketoprofene, furosemide, idroclorotiazide, diuron, isoproturon).

Le verifiche coprono cinque categorie (totale **300 confronti** salvati in `confronto_numeri.csv` e riproducibili con `controllo_numeri.py`):
1. **Massa esatta monoisotopica neutra da formula** (`mzlab.chem.elements.mass` in parità col motore Rust `crates/mzlab-core/src/elements.rs`) confrontata con Pyteomics, RDKit e PyOpenMS (30 test).
2. **Rapporti $m/z$ di 5 addotti comuni** (`[M+H]+`, `[M+Na]+`, `[M+NH4]+`, `[M-H]-`, `[M+HCOO]-`) calcolati con `mzlab.chem.elements.ion_mz` e confrontati con Pyteomics + massa dell'elettrone CODATA/NIST ($0{,}00054858\text{ Da}$) (150 test).
3. **Pattern isotopico teorico** (abbondanze relative di $M$, $M+1$, $M+2$ da `mzlab.chem.composition.isotope_ratios`) confrontato con `CoarseIsotopePatternGenerator` di PyOpenMS (90 test).
4. **Errore di massa in ppm** (`mzlab.qaqc.ppm_error`) su 10 punti analitici tra $100\text{ Da}$ e $1000\text{ Da}$ ($\text{da } -15\text{ a } +20\text{ ppm}$) confrontato con calcolo in virgola mobile a 64 bit (10 test).
5. **Rivelazione di picchi cromatografici (feature LC-MS)** eseguita direttamente con il binario Rust `cargo run --release -p mzlab-cli -- features` su un file `mzML` sintetico contenente 5 picchi gaussiani noti ($m/z$, tempo di ritenzione $t_R$, altezza, area) (20 test).

---

## 2. Sintesi dei risultati

| Categoria | Confronti | Tolleranza analitica | Differenza massima osservata | Fuori tolleranza |
| :--- | :---: | :---: | :---: | :---: |
| **Massa neutra monoisotopica** (vs Pyteomics / PyOpenMS) | 30 | $10^{-3}\text{ Da}$ | $3{,}72 \times 10^{-8}\text{ Da}$ | **0** |
| **Addotti** (`[M+H]+`, `[M+Na]+`, `[M+NH4]+`, `[M-H]-`, `[M+HCOO]-`) | 150 | $10^{-3}\text{ Da}$ | $3{,}77 \times 10^{-8}\text{ Da}$ | **0** |
| **Pattern isotopico** ($M$, $M+1$, $M+2$ vs PyOpenMS) | 90 | $0{,}5\%$ | $0{,}216\%$ | **0** |
| **Errore in ppm** (`ppm_error`) | 10 | $10^{-5}\text{ ppm}$ | $0{,}00\text{ ppm}$ | **0** |
| **Rivelazione picchi (`mzlab-cli features`)** — $m/z$ e $t_R$ | 10 | $5\text{ mDa}$ / $0{,}05\text{ min}$ | $0{,}00\text{ Da}$ / $6{,}7 \times 10^{-6}\text{ min}$ | **0** |
| **Rivelazione picchi (`mzlab-cli features`)** — altezza e area | 10 | $1\%$ | $1{,}1 \times 10^{-2}\text{ counts}$ / $0{,}17\%$ | **0** |
| **Totale** | **300** | — | — | **0** |

---

## 3. Analisi delle differenze oltre tolleranza e delle micro-differenze numeriche

**Nessun confronto supera la tolleranza analitica (0 FAIL su 300).** Il motore di mzLab è numericamente solido e tratta con il segno corretto la massa dell'elettrone sia negli ioni positivi ($-m_e$) sia negli ioni negativi ($+m_e$).

Si segnalano unicamente le micro-differenze sub-tolleranza osservate tra librerie e la loro causa fisica/numerica:

1. **Massa monoisotopica e addotti ($\le 3{,}8 \times 10^{-8}\text{ Da}$ vs Pyteomics/PyOpenMS; $\le 1{,}85 \times 10^{-5}\text{ Da}$ vs RDKit):**
   - **Causa:** mzLab (`crates/mzlab-core/src/elements.rs`), Pyteomics e PyOpenMS condividono le masse monoisotopiche IUPAC/NIST CODATA a 9--11 cifre decimali; la tabella interna di RDKit arrotonda invece alcuni isotopi a 6--7 cifre decimali, generando scarti fino a $18\text{ }\mu\text{Da}$ ($\sim 0{,}04\text{ ppm}$) sulle molecole più grandi (es. tetraciclina, $\text{C}_{22}\text{H}_{24}\text{N}_2\text{O}_8$).

2. **Abbondanza relativa di $M+1$ e $M+2$ ($\le 0{,}216\%$ in punti percentuali sulla tetraciclina):**
   - **Causa:** deriva dalla convenzione per l'abbondanza naturale media del $^{13}\text{C}$: mzLab adotta il valore standard IUPAC $^{13}\text{C} = 1{,}07\%$ (`crates/mzlab-core/src/elements.rs`), mentre `CoarseIsotopePatternGenerator` di PyOpenMS usa $^{13}\text{C} \approx 1{,}078\text{--}1{,}08\%$ e normalizza la distribuzione polinomiale troncata prima di riscalare al picco base. Su 22 atomi di carbonio lo scarto teorico atteso su $M+1$ è $22 \times 0{,}01\% \approx 0{,}22\%$, identico al valore osservato ($0{,}216\%$).

3. **Altezza ($\le 0{,}011\text{ counts}$) e area ($\le 0{,}17\%$) dei picchi cromatografici da `mzlab-cli features`:**
   - **Causa:** nel formato standard `mzML` il vettore delle intensità è codificato in virgola mobile IEEE 754 a 32 bit (`float32`, $\sim 7$ cifre significative), il che introduce un arrotondamento di $\sim 0{,}01\text{ counts}$ su altezze di $3\text{--}6 \times 10^5\text{ counts}$. Per l'area, la piccola differenza ($\le 0{,}17\%$) dipende dalla definizione dei bordi di integrazione della feature discreta rispetto al taglio fisso di soglia sul profilo gaussiano campionato.
