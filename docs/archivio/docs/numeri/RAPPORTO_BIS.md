# Controllo indipendente dei numeri del motore di mzLab — Rapporto BIS (T6)

## 1. Informazioni generali ed esecuzione

- **Data di esecuzione:** 2026-10-16
- **Commit di `origin/main` utilizzato:** `188b4409fdcec88760221d59b6863822e5e5b6cc`
- **Ambiente di verifica:** Python 3 standard library (`csv`, `re`, `sys`, `pathlib`), Pyteomics 5.0.1, RDKit 2026.09.1, PyOpenMS 3.6.0, NumPy 2.4.6, binario nativo `mzlab-cli` compilato da sorgenti Rust (`crates/mzlab-cli`).
- **File di confronto generati:**
  - `docs/controlli/numeri/confronto_numeri.csv` (riferimento iniziale)
  - `docs/controlli/numeri/confronto_numeri_bis.csv` (re-run indipendente T1 bis)
  - `docs/controlli/numeri/uscita_cli/features.csv` (estrazione feature LC-MS da mzML sintetico)
  - `docs/controlli/numeri/controlla_intestazioni.py` (script di controllo automatico delle intestazioni e del formato)

---

## 2. Sintesi del re-run indipendente T1 (confronto riga per riga)

Il re-run della suite indipendente di calcolo numerico (`controllo_numeri.py`) è stato rieseguito in modo deterministico sul commit di riferimento, generando il file `confronto_numeri_bis.csv`.

Il confronto riga per riga tra `confronto_numeri_bis.csv` e `confronto_numeri.csv` attesta la perfetta concordanza:
- **Totale test eseguiti:** 300
- **Differenze riscontrate:** 0 (confronto byte a byte identico, hash SHA-256: `a3750cbb8006dd36af39b4859a24f9fee57a9d3af4a356a79ad5e8cd33255b33`)
- **Esiti:** 300 PASS, 0 FAIL
- **Tolleranze analitiche:** tutte pienamente rispettate

| Categoria di controllo | Numero test | Tolleranza analitica | Differenza massima osservata | Test fuori tolleranza | Esito |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Massa neutra monoisotopica** (30 composti vs Pyteomics / PyOpenMS) | 30 | $10^{-3}\text{ Da}$ | $3{,}72 \times 10^{-8}\text{ Da}$ | 0 | **PASS** |
| **Addotti ESI comuni** (`[M+H]+`, `[M+Na]+`, `[M+NH4]+`, `[M-H]-`, `[M+HCOO]-`) | 150 | $10^{-3}\text{ Da}$ | $3{,}77 \times 10^{-8}\text{ Da}$ | 0 | **PASS** |
| **Pattern isotopico teorico** ($M$, $M+1$, $M+2$ vs PyOpenMS) | 90 | $0{,}5\%$ | $0{,}216\%$ | 0 | **PASS** |
| **Errore di massa in ppm** (`mzlab.qaqc.ppm_error` su 10 punti analitici) | 10 | $10^{-5}\text{ ppm}$ | $0{,}00\text{ ppm}$ | 0 | **PASS** |
| **Rivelazione picchi CLI (`mzlab-cli features`)** — $m/z$ e $t_R$ | 10 | $5\text{ mDa}$ / $0{,}05\text{ min}$ | $0{,}00\text{ Da}$ / $6{,}7 \times 10^{-6}\text{ min}$ | 0 | **PASS** |
| **Rivelazione picchi CLI (`mzlab-cli features`)** — altezza e area | 10 | $1\%$ | $1{,}1 \times 10^{-2}\text{ counts}$ / $0{,}17\%$ | 0 | **PASS** |
| **Totale complessivo** | **300** | — | — | **0** | **300 PASS** |

---

## 3. Verifica delle regole di formato e intestazioni (`controlla_intestazioni.py`)

L'output prodotto dalla riga di comando (`docs/controlli/numeri/uscita_cli/features.csv`) è stato analizzato con lo script di verifica dedicato `controlla_intestazioni.py`. Lo script implementa il controllo delle tre regole concordate di qualità e leggibilità dei dati esportati.

| Regola | Descrizione sintetica | Stato | Dettaglio / Motivazione |
| :--- | :--- | :---: | :--- |
| **Regola (a)** | **Intestazione formula:** divieto del nome isolato `formula` (case-insensitive); obbligo di specificare `Neutral formula` o l'addotto tra parentesi quadre (es. `[M+H]+ formula`). | **NON ANCORA** | Nessuna colonna relativa a formule molecolari è presente nell'output attuale di `mzlab-cli features`. Nessuna intestazione non conforme (`formula` nuda) è stata rilevata. La funzionalità di emissione delle formule è in fase di integrazione (PR branch `claude/formula-neutra`). |
| **Regola (b)** | **Formato aree dei picchi:** le colonne `.*:area` devono essere espresse in notazione scientifica esponenziale `d.dde+N` (regex `r"^[+-]?\d+(\.\d+)?[eE][+-]?\d+$"`) e non devono recare suffissi come M/k/G. | **NON ANCORA** | I valori della colonna `datafile:sintetico:area` sono numeri decimali standard (es. `2293075.4`, `4785867.2`) e rispettano il divieto di suffissi M/k/G. La notazione esponenziale scientifica non è ancora attiva nella CLI poiché la relativa PR di formattazione non è ancora confluita nel ramo `main`. |
| **Regola (c)** | **Nomi leggibili delle modifiche:** divieto del carattere trattino basso `_` nei nomi e nelle etichette leggibili di colonne di modifica (es. annotazioni di trasformazione TP). | **NON ANCORA** | Nessuna colonna di modifica o annotazione di prodotti di trasformazione (TP) è presente nell'output attuale di `mzlab-cli features`. |

---

## 4. Punti di attenzione e note per i team di sviluppo delle nuove funzionalità

A supporto del lavoro coordinato sulle feature in corso di sviluppo nei vari rami di lavoro, si segnalano i seguenti punti operativi:

1. **Esportazione formule molecolari nella CLI:**
   - Attualmente `mzlab-cli features` esporta la tabella `full` secondo la convenzione base di MZmine (`id, mz, rt, rt_range:min, rt_range:max, charge, datafile:<file>:area, datafile:<file>:height, datafile:<file>:feature_state`).
   - Nel momento in cui le formule identificate verranno incluse nell'output CLI, occorre verificare che la colonna sia nominata `Neutral formula` per la molecola neutra, oppure con l'indicazione dell'addotto tra quadre (es. `[M+H]+ formula`), evitando rigorosamente l'intestazione generica o non qualificata `formula`.

2. **Formattazione esponenziale delle aree dei picchi:**
   - La funzione interna di formattazione `sig8` in `crates/mzlab-feat/src/export.rs` effettua attualmente un parsing a `f64` che riconverte valori in notazione decimale standard per numeri di media grandezza (es. `2293075.4` invece di `2.293075e+06`).
   - Quando la PR per l'adozione sistematica della notazione esponenziale `d.dde+N` verrà fusa in `main`, le aree dovranno essere stampate direttamente come stringhe scientifiche conformi al pattern `r"^[+-]?\d+(\.\d+)?[eE][+-]?\d+$"`.
   - È confermata l'assenza di suffissi abbreviati (`M`, `k`, `G`), garantendo l'interoperabilità con i parser numerici standard.

3. **Etichette leggibili per modifiche e prodotti di trasformazione (TP):**
   - Le modifiche e le annotazioni TP non sono ancora esposte nelle colonne della CLI.
   - Quando queste colonne verranno introdotte, i nomi mostrati all'utente e inseriti nei campi testuali dovranno utilizzare spazi o formattazioni naturali senza underscore (es. "mono oxidation" o "desmethyl" anziché "mono_oxidation" o "des_methyl").

4. **Validazione automatizzata continua:**
   - Lo script `docs/controlli/numeri/controlla_intestazioni.py` non richiede alcuna dipendenza esterna (solo libreria standard Python) ed è pronto per essere eseguito automaticamente nelle pipeline di integrazione o manualmente con:
     ```bash
     python3 docs/controlli/numeri/controlla_intestazioni.py
     ```
   - Lo script segnalerà automaticamente lo stato `RISPETTATA` non appena i rami contenenti le nuove feature saranno confluiti in `main`.
