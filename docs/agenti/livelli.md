# Livelli di identificazione (Tabui, Attributio, flusso untargeted)

Definizioni scelte da Federico. Valgono per Tabui (MS2 a bassa risoluzione), Attributio, la vista «Cos'è?», «Identifica tutte le MS2» e il flusso untargeted di mzFinder. Regole in `mzlab/web/livelli-1.js` (dati), cascata in `mzlab/web/livelli.js`, copia per Rust in `crates/mzlab-feat/data/livelli-1.json` (`python3 tools/esporta_livelli.py`).

## Scala di Schymanski (livello di identificazione)
- **L1**: standard di riferimento segnato dal chimico (massa, RT, MS2).
- **L2a**: corrispondenza dello spettro MS2 ad alta risoluzione con una libreria sperimentale (mzCloud, MassBank, MoNA). Una libreria in silico o un'MS2 senza libreria non danno mai 2a (`evid/graph.rs`: `LibHit.experimental`, `max_level`).
- **L2b**: struttura probabile supportata da frammenti diagnostici e da spettri in silico (MetFrag, CFM-ID, SIRIUS/CSI:FingerID), oppure da evidenze di contesto chimico univoche senza standard in libreria. **Solo il chimico assegna il 2b, mai il programma**: il programma può proporre le prove, il livello sta in «deciso» (`LIVELLI.decide`: da 2a, 3a, 3b o 4; rifiutato su 5 e 0).
- **L3a**: pochi candidati posizionali/isomerici tra omologhi (proposto dal programma per candidato che non passa la conferma, provvisorio, nomi in conflitto, isomeri non distinti, isomero di RT, revisione dell'esperto, CSI:FingerID).
- **L3b**: classi generiche con molti isomeri plausibili (proposto per il solo analogo; strutture parziali del Disegno `parz/1`, `level: "3b"`).
- **L4** formula univoca; **L5** massa esatta, **L0** rilevata senza annotazione.
- Una classe CANOPUS (ClassyFire, NPClassifier) è un'annotazione separata (colonne `classes`/`class_probs`, soglia `canopus_min`): non assegna né alza alcun livello, in nessun profilo.
- A risoluzione unitaria (profilo `lr`, `tetto` "4" in `livelli-1.json`) una massa nominale è un candidato e una MS2 non alza il livello: oggi il profilo emette solo 4 (la formula nominale basta a spiegare lo ione) o 5.
- **Tabui** (risoluzione unitaria): il programma non propone alcun livello di Schymanski (`level` null, colonna «Lvl» vuota) e mostra solo «Prev. N» della regola; il livello lo sceglie il chimico nella riga aperta (`TPDEC`, il 2b resta rifiutato senza livello proposto). `level_text` dice cosa reggono i dati: «Proposed formula» vuole un picco cromatografico vero, il progenitore che decade, un solo passo e una classe di ipotesi sopra il caso (`legacy/lr/summary.rs`, `docs/agenti/mzfinder-lr.md`). Una MS2 abbinata non cambia nulla (D6 = no).

## Livelli raggiungibili per risoluzione
Principio: in bassa risoluzione (LR) il programma mostra solo i livelli che le prove possibili reggono, senza elencarli a priori; la regola resta `livelli.js` / `livelli-1.json` (parità `tests/livelli_casi.json`). La colonna «LR per definizione» deriva dalle definizioni di Schymanski; «LR oggi» da ciò che il codice emette.

| Livello | Servono | LR per definizione | LR oggi |
|---|---|---|---|
| L1 | standard: massa, RT, MS2 | sì (standard misurato sullo stesso strumento) | no (profilo `lr` senza `standard`) |
| L2a | MS2 sperimentale ad alta risoluzione + libreria | no (spettro HR) | no |
| L2b | frammenti diagnostici / in silico / contesto, deciso dal chimico | solo con frammenti diagnostici misurati | no |
| L3a/L3b | candidati posizionali o classe + MS2 o libreria | sì, come «tentativo» (libreria MS2 a massa nominale) | no (tetto 4) |
| L4 | formula univoca | solo se l'accuratezza di massa la rende univoca: a massa nominale è un candidato | sì (tetto `lr`: formula nominale unica) |
| L5 | massa esatta | massa nominale | sì (senza prove) |
| L0 | rilevata, nessuna annotazione | sì | sì |

Le righe «per definizione» ≠ «oggi» sono il lavoro dei livelli automatici; Tabui emette `level` null anche dove la tabella dice sì.

## Due scale, due etichette
- Scala di Schymanski: etichetta «L» + livello, sottolivello in minuscolo («L2a», «L3b»); in JS `LIVELLI.tag(lv)`, classi colore `.lv-2a`, `.lv-3a`, `.lv-3b`… (variabili `--lv*` in `app.css`). Nelle esportazioni (CSV, JSON) i valori restano senza «L» (`2a`, `3b`).
- Scala di previsione di Löffler et al., ES&T 59:19095 (2025): etichetta «Prev. N» (gradi 1-4), colonna «Prev.» della tabella dei prodotti di mzFinder (`.lvb.pd-N`).
- Sono scale diverse e nell'interfaccia non si confondono mai: la stessa cifra con etichette diverse («L3a» contro «Prev. 3») è normale.
- Il programma propone un livello (in HR; in Tabui LR nessuno) e mostra le prove; il livello deciso resta vuoto finché il chimico non decide.
- La tabella dei prodotti HR di mzFinder (`21_tpmine_hr.js`) mostra il livello proposto 5/4 senza sottolivello come «L5/L4» (`TPTAB.level`, colonna «Lvl», la qualità va da prova diagnostica a L4, L5; una MS2 abbinata non dà L3, la dice «Prev. N») e la previsione come «Prev. N» (`TPTAB.pred`); le intestazioni dell'esportazione sono «Level (Schymanski)» e «Prediction (Löffler)».
- Test `tests/test_livelli_schymanski.py`: one fixed fixture per evidence class of the 2014 paper (standard, library, weak match, analogue, unique formula, exact mass, unit resolution) must give the level and name its evidence; 2b only through the chemist's decision.
