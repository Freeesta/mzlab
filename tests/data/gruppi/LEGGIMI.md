# Dataset Sintetico Multi-Condizione (Gruppi e Cinetica)

Dataset sintetico di prova per il modulo di confronto condizioni di reazione di Tabui,
progettato per validare la differenziazione cinetica e l'attribuzione di pathway fotolitici
e fotocatalitici a partire da dati LC-MS a bassa risoluzione (full scan Q1, massa nominale, ESI+).

I dati sono interamente sintetici: nessun dato reale, proprietario o confidenziale e' presente.
Il composto genitore (Parent) e' lo standard pubblico Bentazone.

## 1. Nomenclatura e Struttura dei File

Il dataset e' composto da esattamente **38 file mzML**, strutturati secondo lo schema:
- **Corse cinetiche (36 file)**: `<Condizione>_<tempo>_<replica>.mzML`
  - 6 condizioni sperimentali:
    1. `Air`: fotocatalisi in atmosfera d'aria
    2. `O2`: fotocatalisi in ossigeno puro
    3. `N2`: fotolisi in atmosfera inerte di azoto
    4. `Ar`: fotolisi in atmosfera inerte di argon
    5. `He`: fotolisi in atmosfera inerte di elio
    6. `Reference`: controllo di stabilita' al buio
  - 3 punti temporali: `t0` (tempo zero), `t5` (5 minuti), `t10` (10 minuti)
  - 2 repliche per tempo: `r1`, `r2` (variabilita' sperimentale deterministica +/- 3-5%)
- **Controlli analitici (2 file)**:
  - `bianco.mzML`: bianco del solvente (eluente / matrice)
  - `standard.mzML`: standard analitico del solo composto parent al tempo t0

Ogni file e' registrato nell'intervallo cromatografico 1.0 - 5.0 minuti con 80 scansioni
centroid zlib-compresse ed ha una dimensione di circa 59-60 KB, per un peso totale del dataset
di circa 2.2 MB, ampiamente sotto il limite di 3 MB.

## 2. Profilo dei Composti

| Composto | Ruolo Analitico | m/z Nominale | RT Teorico (min) | FWHM (min) | Comportamento Cinetico |
|---|---|---|---|---|---|
| **Parent** | Bentazone (substrato) | 241 | 3.00 | 0.14 | Decresce nelle condizioni attive (100% -> 50% -> 15%); stabile nel Reference (100% -> 98% -> 96%); assente nel bianco. |
| **TP1** | Intermedio mono-idrossilato | 257 | 2.20 | 0.13 | Cinetica a campana in tutte le condizioni attive (t0=0, t5=max ~2.0e5, t10=calo ~6.0e4); traccia debole nel Reference; assente in bianco e standard. |
| **TP2** | Foto-ossigenazione | 273 | 1.80 | 0.12 | Presente SOLO in O2 e Air (t0=0, t5=1.0-1.8e5, t10=1.5-2.5e5); totalmente assente nelle condizioni inerti (N2, Ar, He), Reference e bianco. |
| **TP3** | Fotolisi anaerobica | 225 | 2.60 | 0.13 | Presente SOLO in N2, Ar, He (t0=0, t5=1.2e5, t10=2.0e5); totalmente assente in presenza di ossigeno (O2, Air), Reference e bianco. |
| **TP4** | Contaminante / Plastificante | 279 | 4.20 | 0.15 | Presente ad area costante (~1.2e5) in TUTTI i file inclusi bianco e standard. Deve essere scartato dal filtro del bianco. |
| **TP5** | In-Source Fragment (ISF) | 197 | 3.00 | 0.14 | Frammento formato in sorgente ESI. Sempre co-eluente con il parent (RT identico) ad altezza pari al ~18% del parent. Assente nel bianco. |

## 3. Criteri di Validazione per Tabui

Il dataset consente a Tabui di verificare i seguenti algoritmi:

1. **Filtro del Bianco (Blank Subtraction)**:
   - TP4 (m/z 279, RT 4.20) e' rilevato sia nei campioni cinetici sia in `bianco.mzML`.
   - Il filtro del bianco deve escludere TP4 dall'elenco dei transformation product reali.
2. **Deconvoluzione degli In-Source Fragments (ISF)**:
   - TP5 (m/z 197) co-eluisce al 100% con il Parent (m/z 241, RT 3.00) in tutte le corse e
     mantiene un rapporto di area costante (~18%).
   - L'algoritmo di raggruppamento per famiglie di ioni (`mode: family`) lo assegna alla
     famiglia del parent senza considerarlo un prodotto di degradazione indipendente.
3. **Classificazione per Condizione Ambientale**:
   - Condizioni aerobiche (`Air`, `O2`): formazione selettiva di TP2 (m/z 273). La resa in O2
     puro e' significativamente maggiore rispetto ad Air.
   - Condizioni anaerobiche / inerti (`N2`, `Ar`, `He`): formazione selettiva di TP3 (m/z 225).
     Nessuna traccia di TP2.
   - Condizione di riferimento al buio (`Reference`): tasso di degradazione del parent inferiore
     al 5% a t10, confermando che la scomparsa del parent nelle altre condizioni e' fotochimica.
4. **Cinetica di Formazione e Consumo**:
   - TP1 (m/z 257) manifesta l'andamento a campana tipico di un intermedio di reazione
     consecutivo (Parent -> TP1 -> prodotti secondari/mineralizzazione).

## 4. Ground Truth: attesi.json

Il file `attesi.json` contiene la verita' analitica strutturata per la verifica automatica:
- `metadata`: parametri chimici del parent, elenco condizioni, tempi, repliche e controlli.
- `composti`: definizione di ogni composto (formula, m/z, RT, ruolo analitico, descrizione).
- `file`: dizionario indicizzato per nome file con metadati sperimentali e, per ciascun
  composto, stato di presenza (`presente: true/false`), tempo di ritenzione atteso, altezza
  e area attese (in counts * s).

## 5. Verifica Tramite mzlab-cli

Per estrarre la tabella delle feature da uno o piu' file del dataset:

```bash
# Estrazione feature in modalita famiglie (default: raggruppa TP5 con il parent)
cargo run --release -p mzlab-cli -- features tests/data/gruppi/standard.mzML --out /tmp/feat.csv

# Estrazione feature a singoli ioni (evidenzia separatamente sia parent 241 sia ISF 197)
echo '{"mode":"ion"}' > /tmp/params.json
cargo run --release -p mzlab-cli -- features tests/data/gruppi/standard.mzML --params /tmp/params.json --out /tmp/feat_ion.csv
```
