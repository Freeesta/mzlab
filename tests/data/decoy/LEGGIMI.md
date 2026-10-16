# Libreria di Parent Decoy e Regole di Trasformazione (T15)

## 1. Scopo
Questa cartella contiene una serie di dati di riferimento sintetici/negativi per il benchmark e la validazione del modulo Tabui di mzLab. In particolare, i composti parent inclusi qui fungono da **decoy (controlli negativi)** per misurare il tasso di **falsi positivi (false positive rate)** su spettri sperimentali o di demo:
- I 30 composti parent selezionati sono molecole pubbliche ben caratterizzate (pesticidi, farmaci, composti industriali) che **non sono presenti** nei file di acquisizione Full Scan di esempio di mzLab (`mzlab/web/esempi/FullScan_t*.mzML`).
- Eseguendo la ricerca non-target / screening con Tabui impostando uno di questi composti come genitore e applicando le regole di trasformazione, il numero atteso di prodotti di trasformazione (TP) candidati reali identificati deve essere pari a **zero** (o confinato al solo rumore statistico di fondo con score trascurabile).

## 2. File Inclusi
- `decoy_parent.csv`: 30 composti decoy pubblici con nome, SMILES canonico, formula molecolare neutra Hill, massa teorica monoisotopica di `[M+H]+` e `[M-H]-` (calcolata con Pyteomics, 4 decimali, o `N/A` se non ionizzabile in ESI-) e il flag di presenza nei demo (`present_in_demo = no`).
- `trasformazioni_decoy.csv`: 15 regole di trasformazione generiche (nomi in parole semplici senza trattini bassi, variazioni nette di formula) non duplicate rispetto al set `tests/data/nomi_tp/trasformazioni.csv`.

## 3. Criteri di Selezione e Verifica
- Intervallo di massa monoisotopica compreso rigorosamente tra 150 e 600 Da (da 279 Da a 585 Da).
- Verifica strumentale: per ciascuna massa `mz_pos` e `mz_neg` è stato estratto il cromatogramma a ioni estratti (XIC, tolleranza $\pm 0.5$ Da) su tutti i file FullScan della serie temporale di demo. Nessun composto presenta picchi cromatografici reali (tutti con $S/N \le 8.2$, ben al di sotto della soglia di $S/N = 10$).
- Tutti gli script di calcolo e screening sono stati eseguiti all'esterno del repository (in `~/Desktop/gemini_cc/script/`).
