# patRoon e i prodotti di trasformazione (TP): cosa fa, cosa fa già mzLab, cosa adottare

Nota di studio per Federico. Solo documento, nessun codice di prodotto. Sorgenti: documentazione ufficiale di patRoon (handbook cap. 7 «Transformation product screening» e pagine di reference `generateTPs`, `generateComponentsTPs`, `componentsTPs-class`, `retDir`, `generateTPsBioTransformer`), `DESCRIPTION` e `README` del repository GitHub `rickhelmus/patRoon`, lette il 2026-10-19. Il lato mzLab viene da `docs/agenti/interp.md`, `livelli.md`, `docs/studi/TABUI_TP_TIMOLOLO.md` e da `crates/mzlab-feat/src/interp/`.

**Come leggere i segni**: «letto» = scritto nella documentazione ufficiale; «ipotesi» = non verificato (nessuno ha installato né eseguito patRoon; le pagine sono state lette con un lettore web, che in parte riassume).

## 0. Risposta breve
- **Sì, patRoon ha un modulo TP**, dalla versione 2.0 (letto). Versione attuale 3.1.0, licenza **GPL-3** (letto: `DESCRIPTION`, README). È un pacchetto R (R ≥ 4.1) per l'analisi non mirata (NTA) con strumenti esterni.
- **Noi non l'avevamo valutato**; questa nota è la prima valutazione, fatta sulla sola documentazione.
- patRoon fa due cose distinte: (A) **genera** i TP attesi di un genitore (da reazioni note o da programmi di previsione); (B) **collega** le feature del genitore e dei TP e le valuta (similarità MS/MS, frammenti e perdite neutre in comune, direzione di ritenzione).
- mzLab (`interp`) fa soprattutto (B) a modo suo, partendo dal dato e non da una lista attesa: modifica rispetto al genitore, MS2 della serie, confronto con il genitore e con l'albero MSn. La parte (A) la copre in piccolo il registro `ipotesi/1` (Δ di formula note).

## 1. Cosa fa davvero patRoon (letto)
**Generare i TP** (`generateTPs(algorithm = ...)`), da usare con un elenco di genitori (SMILES/InChI o formula):
- `biotransformer`: previsione con BioTransformer (tipi `env`, `ecbased`, `cyp450`, `phaseII`, `hgut`, `superbio`, `allHuman`; `generations` = numero di passi; di default 2). Serve il programma `biotransformer.jar` installato a parte; la reference dice anche che si appoggia a OpenBabel.
- `cts`: tabelle del Chemical Transformation Simulator dell'EPA (idrolisi, riduzione abiotica, fotolisi, metabolismo dei mammiferi, PFAS...). Il README lo cita come sorgente; se serva la rete: **ipotesi** (CTS è un servizio web, non verificato).
- `library`: libreria di trasformazioni note (PubChem Transformations, DOI Zenodo 10.5281/zenodo.5644560, installata con il pacchetto) o tabella propria (`parent_SMILES`, `TP_SMILES`, e facoltativi i logP).
- `library_formula`: come `library` ma con le sole formule (`parent_formula`, `TP_formula`); nessuna libreria di default; `genFormulaTPLibrary()` la costruisce da regole.
- `logic`: parte dalle **feature** e non dai genitori; cerca differenze di m/z pari a trasformazioni elementari predefinite (Schollee et al. 2015). Regole = tabella `transformation`, `add`, `sub`, `retDir`.
- `ann_form`, `ann_comp` (dalla 3.0, Helmus et al. 2025): estraggono e ordinano i TP dai candidati di annotazione (formule o strutture) con un «TP Score» (adattamento del candidato dentro il genitore e somiglianza con i sospetti). **Nota**: l'algoritmo «annotated» citato nella richiesta non esiste con quel nome; sono questi due.
- Filtri sui TP generati (`filter`): proprietà del CTS (probabilità), `removeParentIsomers`, `removeTPIsomers`, `removeDuplicates`, `minSimilarity` (similarità strutturale), `minTPScore`, `topMost`.

**Collegare e valutare** (`generateComponents(algorithm = "tp")`, classe `componentsTPs`):
- Ingressi: feature dei genitori e dei TP (anche sottoinsiemi diversi: «prima»/«dopo» il trattamento), lista TP (o `NULL`: ogni feature è un TP possibile, con molti falsi positivi), liste MS2 (`MSPeakLists`), formule, composti. Di norma prima si fa un suspect screening di genitori e TP.
- **Similarità MS/MS genitore-TP**: coseno di spettro con tre allineamenti (nessuno, sul precursore, entrambi: `specSimilarity`, `…Prec`, `…Both`; soglie `minSpecSim*`). Il tipo di coseno e i suoi dettagli: **ipotesi** (non letti nel codice).
- **Frammenti e perdite neutre in comune**, contati sulle formule assegnate ai frammenti MS/MS (`fragmentMatches`, `neutralLossMatches`, totali con soli conteggi unici; soglie `minFragMatches`, `minNLMatches`, `minTot…`).
- **Direzione di ritenzione** `retDir` (−1 prima, +1 dopo, 0 simile o ignota): attesa dal logP (logP più basso = eluisce prima, fase inversa; soglia `minLogPDiff` in `TPStructParams`) o dalla colonna `retDir` delle regole `logic`; osservata dalla differenza di RT, solo oltre `minRTDiff` (20 s di default). Filtro `retDirMatch`: non scarta mai se una delle due direzioni è 0.
- Verifica delle regole `logic` con le formule (filtro `formulas`): il genitore contiene gli elementi sottratti e il TP quelli aggiunti.
- Esito: tabella per feature con `retDiff`, `mzDiff`, `formulaDiff`, e grafico dei percorsi (`plotGraph`). **Classificazioni chimiche per riga**: non le ho trovate nelle pagine lette (ipotesi: non ci sono; non verificato nel codice).

**Requisiti** (letto o no): R ≥ 4.1, compilatore C++14 e `make`, `rJava` e `rcdk` in `Imports` (quindi Java presente nella catena: letto; Java per BioTransformer: ipotesi); BioTransformer e OpenBabel a parte (letto); rete: non dichiarata, ipotesi per CTS.

## 2. Confronto
| Cosa fa patRoon | Cosa fa mzLab | Lacuna | Idea da adottare |
|---|---|---|---|
| Genera TP attesi da librerie (`library`, PubChem) e programmi (BioTransformer, CTS) | Nessuna previsione dei TP dalla struttura. `ipotesi/1` dà Δ di formula note (es. +O, −2H) e `interp` sceglie quella entro 5 ppm | Nessun elenco di TP attesi per un genitore con la struttura | Solo **dati**: piccola libreria locale di trasformazioni note come tabella (genitore → TP, anche solo formule) da proporre come ipotesi con la fonte. Niente rete, niente Java |
| `logic`: Δm elementari con colonna `retDir` | `ipotesi/1` ha le Δ; la direzione di RT non è nella regola (ipotesi: non verificato nel registro) | Le regole non portano la direzione attesa di RT | Aggiungere alla regola una direzione attesa (−1/0/+1) come campo facoltativo, mostrata come segnalazione, mai come scarto |
| Controllo `logic` con le formule: elementi tolti/aggiunti presenti nelle formule di genitore e TP | `iso_check` (M+1, M+2, conteggio di S) e Δ di formula in `modifica.rs` | Controllo simile, ma non sugli elementi della regola | Già coperto in sostanza; nessuna azione |
| Coseno MS/MS genitore-TP con 3 allineamenti | `against_dda`: coseno modificato (sposta di Δ), entropia, frammenti uguali e spostati | Nessuna: il coseno modificato corrisponde all'allineamento «precursore» di patRoon | Controllare in una prova che un singolo punteggio riassuntivo (come `specSimilarityBoth`) sia mostrato accanto agli altri |
| Frammenti e perdite neutre condivisi, per formula | `by_formula` (uguali/spostati per massa della formula) e perdite contro il consenso DDA | Nessuna sostanziale | Nessuna. Il conteggio «solo conteggi unici» di patRoon è un'idea di presentazione: un numero unico per riga |
| `retDir` da logP, soglia `minLogPDiff`, `minRTDiff` | Attributio ha RT contro logP (`rtlogp`, fit, z robusto); `TABUI_TP_TIMOLOLO` §1 punto 9 propone l'anomalia di RT per i TP (ipotesi: non so se è in `interp`) | Nella riga `interp` non c'è una direzione di RT attesa contro osservata | **Adottare**: per ogni TP, direzione osservata (RT − RT genitore, oltre una soglia) contro direzione attesa (da logP o dalla regola); se contrastano, nota «RT anomaly», mai scarto. Regola semplice, spiegabile a lezione |
| `minRTDiff`: sotto la soglia la direzione è 0 | Non c'è una soglia dichiarata nella riga | Possibile rumore sui TP quasi coeluiti | Stessa soglia esplicita, in secondi, visibile nella riga |
| Filtri di isomeri e duplicati sui TP attesi | `separate` (isomeri dalle tracce dei frammenti), F2 stessa molecola | Nessuno, sono approcci diversi (dato contro lista) | Nessuna |
| Ordinamento con TP Score (ann_form/ann_comp) | Livelli Schymanski (`livelli.md`) e «Prev. N»; il programma non assegna il 2b | Punteggio unico di plausibilità del TP | **NO a un punteggio unico** che nasconda le prove (principio: il programma mostra prove, decide il chimico) |
| Collega feature «prima» e «dopo» il trattamento | `tp/1` (forma, parentela, F3 formata dal trattamento) | Nessuna | Nessuna |
| `TPs = NULL`: ogni feature è un TP possibile (patRoon avverte dei falsi positivi) | È già la logica di `interp` | — | Nessuna |
| Report e grafico dei percorsi (`plotGraph`) | Scheda «Structure» e disegno del TP sul genitore | Nessun grafo dei percorsi genitore → TP → TP | Rimandato; sarebbe un grafico nuovo e dovrebbe nascere con la sua tabella `TDATI` |

## 3. Raccomandazione breve
**Riusare (idee, non codice)**
1. **Direzione di ritenzione attesa contro osservata**: per riga, con soglia in secondi dichiarata e solo come nota. È la lacuna più chiara e più facile da spiegare agli studenti.
2. **Libreria di trasformazioni note come tabella dati** (come `library_formula` e le regole `logic`): in mzLab estenderebbe `ipotesi/1` con la fonte e, se si vuole, la direzione di RT. Prima di tutto va deciso se serve nel mondo HR (ricerca) e mai nel mondo LR (principio 1: il programma non dà le risposte).
3. **Un solo numero riassuntivo** della somiglianza MS/MS genitore-TP, se non c'è già, accanto alle prove: si fa con quello che `confronto.rs` calcola.

**NON adottare**
- **Dipendenze R, Java, BioTransformer, OpenBabel, CTS**: contrastano con il principio «leggero, tutto nel browser, tutto in Rust» e con la regola «niente rete» (i file degli studenti non lasciano il computer).
- **Codice di patRoon**: è GPL-3; riscrivere un'idea è lecito, copiare codice no (il sito ha altre licenze: **ipotesi**, da verificare prima di ogni riuso diretto).
- **Punteggio unico tipo TP Score** e previsione di TP da struttura con modelli esterni: il programma non deve fare da oracolo.
- **Un'integrazione con patRoon** (chiamarlo, importare i suoi risultati) non è necessaria; resta comunque possibile importare una tabella di TP già calcolata come CSV, se un utente la porta (**ipotesi**, nessuno l'ha chiesto).

## 4. Da verificare prima di decidere (non fatto qui)
- Leggere il codice sorgente di patRoon per il coseno esatto (binning, pesi) e per come calcola `retDir` dal logP (le pagine dicono solo che il logP più basso eluisce prima).
- Provare patRoon su una serie nostra (es. timololo) per un confronto dei risultati: richiede R, Java e BioTransformer; farebbe un confronto di sensibilità, non una validazione.
- Guardare `ipotesi/1` per capire se una direzione di RT vi si può aggiungere senza cambiare lo schema.
- Leggere gli esempi del capitolo 7.3 (`TPsExamples.html`), non letti qui.
- Letteratura: Schollee et al. 2015, Anal. Chem. 87:12121 (regole `logic`); Helmus et al. 2025, Environ. Sci. Technol. 59:3723 (`ann_form`/`ann_comp`).
