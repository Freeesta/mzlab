# Banca Dati dei Prodotti di Trasformazione Noti (TP Noti) in Letteratura

Questa cartella raccoglie la banca dati di riferimento dei prodotti di trasformazione
(Transformation Products, TP) già documentati e identificati nella letteratura scientifica
peer-reviewed per 15 composti modello di rilevanza ambientale e farmaceutica.

---

## 1. Descrizione del Database e delle 15 Molecole Coperte

Il database `tp_noti.csv` censisce 112 prodotti di trasformazione generati attraverso processi di:
- **Fotolisi UV diretta e UV da vuoto** (VUV a 185 nm e UVC a 254 nm);
- **Fotocatalisi eterogenea** su semiconduttori (principalmente TiO2 Degussa P25 o modificato);
- **Processi di Ossidazione Avanzata (AOPs)** guidati dal radicale ossidrile (•OH): UV/H2O2,
  reattivo di Fenton e foto-Fenton solare/UV, ozonizzazione (O3).

### Tabella dei Composti Madre Censiti

| Composto | Formula Neutra | m/z [M+H]+ | m/z [M-H]- | Classe d'uso / Famiglia Chimica | TP Censiti |
|:---|:---|:---:|:---:|:---|:---:|
| **bentazone** | C10H12N2O3S | 241.0641 | 239.0496 | Erbicida benzotiadiazinonico | 6 |
| **atrazina** | C8H14ClN5 | 216.1010 | 214.0865 | Erbicida clorotriazinico (s-triazina) | 9 |
| **diuron** | C9H10Cl2N2O | 233.0243 | 231.0097 | Erbicida fenilureico clorurato | 7 |
| **carbamazepina** | C15H12N2O | 237.1022 | 235.0877 | Farmaco antiepilettico dibenzazepinico | 10 |
| **diclofenac** | C14H11Cl2NO2 | 296.0240 | 294.0094 | Farmaco antinfiammatorio (FANS arilacetico) | 9 |
| **ibuprofene** | C13H18O2 | 207.1380 | 205.1234 | Farmaco antinfiammatorio (FANS arilpropionico) | 7 |
| **sulfametossazolo** | C10H11N3O3S | 254.0594 | 252.0448 | Farmaco antibatterico solfonammidico | 8 |
| **propranololo** | C16H21NO2 | 260.1645 | 258.1500 | Farmaco beta-bloccante naftalenico | 7 |
| **timololo** | C13H24N4O3S | 317.1642 | 315.1496 | Farmaco beta-bloccante morfolino-tiadiazolico | 6 |
| **metoprololo** | C15H25NO3 | 268.1907 | 266.1762 | Farmaco beta-bloccante arilossietilico | 8 |
| **caffeina** | C8H10N4O2 | 195.0877 | 193.0731 | Alcaloide stimolante metilxantinico | 8 |
| **paracetamolo** | C8H9NO2 | 152.0706 | 150.0561 | Farmaco analgesico p-amminofenolico | 7 |
| **imidacloprid** | C9H10ClN5O2 | 256.0596 | 254.0450 | Insetticida sistemico neonicotinoide | 7 |
| **terbutilazina** | C9H16ClN5 | 230.1167 | 228.1021 | Erbicida clorotriazinico (s-triazina) | 6 |
| **atenololo** | C14H22N2O3 | 267.1703 | 265.1558 | Farmaco beta-bloccante acetammidico | 7 |
| **TOTALE** | — | — | — | — | **112** |

---

## 2. Struttura dei Campi in `tp_noti.csv`

Il file `tp_noti.csv` presenta esattamente le seguenti 13 colonne:

1. `parent`: nome del composto precursore (uno dei 15 composti).
2. `tp_label_letteratura`: etichetta o nome chimico/analitico del TP riportato nell'articolo
   originale (es. `6-hydroxybentazone`, `desethylatrazine`, `1-(3,4-dichlorophenyl)-3-methylurea`,
   `carbamazepine-10,11-epoxide`, `5-hydroxydiclofenac`, `PD1`).
3. `neutral_formula`: formula chimica neutra espressa rigorosamente secondo l'ordine
   convenzionale di Hill (carbonio C, idrogeno H, seguiti dagli altri elementi in ordine
   alfabetico: es. C15H12N2O2).
4. `mz_pos`: rapporto massa/carica teorico monoisotopico per l'addotto protonato [M+H]+ a 4
   decimali, calcolato con pesi atomici IUPAC (AME2020) e sottrazione della massa dell'elettrone.
5. `mz_neg`: rapporto massa/carica teorico monoisotopico per l'addotto deprotonato [M-H]- a 4
   decimali (o `N/A` per specie prive di idrogeni acidi non ionizzabili in ESI negativa, come
   idrocarburi o chinoni neutri).
6. `nominal_pos`: massa nominale intera di [M+H]+, arrotondata all'intero più vicino (`round(mz_pos)`).
7. `nominal_neg`: massa nominale intera di [M-H]-, arrotondata all'intero più vicino (`round(mz_neg)`),
   oppure `N/A`.
8. `reaction`: classificazione sintetica della trasformazione in lingua inglese (es. `hydroxylation`,
   `dihydroxylation`, `N-dealkylation`, `dechlorination`, `desulfonation`, `cleavage`, `oxidation`,
   `epoxidation`, `deamination`, `hydrolysis`).
9. `net_change`: variazione stechiometrica netta rispetto alla formula del genitore (es. `+O`, `+2O`,
   `-C2H4`, `-Cl +OH`, `-H2 +O`, `+H2O`, `-CH2`, `-SO2 +H2`).
10. `condition`: condizione di reazione e processo AOP impiegato nello studio originale (`UV`, `TiO2`,
    `UV/H2O2`, `Fenton`, `O3`).
11. `doi`: identificatore univoco Digital Object Identifier (DOI) dell'articolo peer-reviewed,
    verificato al 100% via Crossref REST API.
12. `evidence`: livello di evidenza analitica comprovato nel paper:
    - `authentic standard`: TP confermato mediante iniezione comparativa di standard chimico autentico puro;
    - `HRMS`: spettrometria di massa ad alta risoluzione (Orbitrap, Q-ToF) con accuratezza < 5 ppm
      e frammentazione MS/MS coerente;
    - `unit mass`: spettrometria di massa a bassa risoluzione (GC-MS, LC-MS quad) con match libreria
      NIST/spettro di frammentazione.
13. `note`: annotazione esplicita della provenienza del valore di massa (`calculated from neutral
    formula; ...`) e dettagli strutturali e stereochimici di rilievo.

---

## 3. Criteri di Inclusione ed Evidenza Analitica

1. **Origine peer-reviewed**: nessun dato è stato generato sinteticamente o ipotizzato. Ogni
   singolo TP censito è stato estratto da studi pubblicati su riviste internazionali indicizzate
   (tra cui *Water Research*, *Environmental Science & Technology*, *Applied Catalysis B:
   Environmental*, *Chemosphere*, *Journal of Hazardous Materials*, *Chemical Engineering Journal*,
   *Microchemical Journal*, *Rapid Communications in Mass Spectrometry*).
2. **Verifica automatica dei DOI**: ogni DOI è stato verificato tramite interrogazione HTTP alle API
   ufficiali di Crossref (`https://api.crossref.org/works/<doi>`), accertando la corrispondenza
   univoca di titolo del lavoro, anno di pubblicazione e periodico editoriale. I DOI non attivi o
   non rispondenti sono stati scartati.
3. **Calcolo monoisotopico ad alta precisione**: conformemente agli standard di spettrometria di
   massa, le masse esatte teoriche sono state calcolate a partire dalla formula neutra Hill
   mediante le librerie scientifiche `pyteomics.mass` e convalidata con l'infrastruttura chimica
   interna di `mzLab` (`mzlab.chem.elements`).

---

## 4. Composti con Pochi TP Documentati in Letteratura UV/AOP e Motivazione Chimica

Tra i 15 composti modello analizzati, due molecole si distinguono per un numero limitato di TP
primari stabili identificabili: **bentazone** e **timololo**. La letteratura di fotochimica
ambientale e degradazione AOP ne evidenzia i seguenti fattori chimico-fisici determinanti:

### 4.1 Bentazone (C10H12N2O3S)
- **Instabilità dell'eterociclo benzotiadiazinonico**: la struttura del bentazone contiene un anello
  diidro-benzotiadiazin-4-one 2,2-diossido. L'attacco radicalico da parte di •OH o la fotolisi diretta
  induce una rapida destabilizzazione del legame solfonammidico S–N e del gruppo carbonilico
  coniugato in C4.
- **Rapida mineralizzazione senza accumulo di intermedi complessi**: a differenza di substrati
  policiclici o aromatici stabili (come carbamazepina o propranololo), l'apertura dell'anello
  tiadiazinico conduce velocemente al rilascio di frammenti polari a basso peso molecolare e
  specie inorganiche (SO4^2-, NO3-, NH4+, CO2). Gli unici intermedi aromatici a vita utile sufficiente
  per la rilevazione analitica sono i mono-idrossilati in C6 e C8 e l'acido solfamminico da idrolisi
  dell'anello, rendendo la frazione di TP isolabili molto ridotta rispetto ad altri pesticidi.
- **Contrasto con il destino nel suolo**: mentre nel suolo e nei tessuti vegetali la biodegradazione
  enzimatica produce metaboliti stabili coniugati e acidi antranilici persistenti, in ambiente acquoso
  sotto irraggiamento UV e fotocatalisi TiO2 il bentazone degrada con cinetiche quasi istantanee verso
  la completa mineralizzazione.

### 4.2 Timololo (C13H24N4O3S)
- **Inerzia fotolitica nello spettro UV solare (lambda > 290 nm)**: il timololo presenta un
  coefficiente di assorbimento molare pressoché nullo nella regione UV-A e del visibile, con un debole
  assorbimento centrato a circa 295 nm. Pertanto, la fotolisi diretta solare in acque naturali è quasi
  inapprezzabile (tempo di dimezzamento di diverse settimane).
- **Frammentazione competitiva dell'anello morfolinico e della catena N-alchilica**: nei processi di
  ossidazione avanzata guidati da UVC o ozono, il timololo è suscettibile all'ossidazione dell'azoto
  morfolinico e del nucleo 1,2,5-tiadiazolico. Tali reazioni scatenano la rapida perdita della catena
  ammino-alcolica (tert-butilamminopropossilica) e l'apertura ossidativa della morfolina. Pochi
  intermedi mantengono la struttura di base intatta a sufficienza per essere dosati come TP
  macroscopici, riducendo l'elenco dei TP stabili a poche specie identificate solo recentemente
  mediante tecniche HRMS ad altissima sensibilità (PD1–PD5).

---

## 5. Istruzioni per la Verifica e il Ricalcolo

Tutti gli script di supporto, estrazione da Crossref e calcolo analitico sono situati tassativamente
**all'esterno** del repository `mzLab` nella cartella `/Users/federicocristaudo/Desktop/gemini_cc/script/`,
rispettando la totale integrità del codice sorgente del progetto.

### Ricalcolo del Database
Per rigenerare il file `tp_noti.csv` e ricalcolare tutte le masse esatte:
```bash
/Users/federicocristaudo/Desktop/gemini_cc/conda_env/bin/python /Users/federicocristaudo/Desktop/gemini_cc/script/build_tp_csv.py
```

### Verifica Automatica dei DOI
Per interrogare le API di Crossref e verificare la validità in tempo reale di tutti i 32 DOI:
```bash
/Users/federicocristaudo/Desktop/gemini_cc/conda_env/bin/python /Users/federicocristaudo/Desktop/gemini_cc/script/verify_all_dois.py
```

### Controlli di Conformità Repository
```bash
python3 tools/indice.py --controlla
python3 tools/controlla.py
python3 -m pytest -q tests/test_esempi_anonimi.py
```
