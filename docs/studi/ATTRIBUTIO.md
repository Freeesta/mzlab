# Attributio in mzLab: pipeline untargeted HR con annotazione automatica e livello 2b guidato

Studio dell'Architetto per la seconda pipeline del programma (la prima è Tabui, la ricerca dei prodotti di trasformazione). Fonti lette:
`docs/agenti/{mzfinder-untargeted,mzfinder-import,alta-risoluzione,librerie,mzfeat,mzfinder-validazione,mzfeat-wasm}.md`, `mzlab/web/livelli-1.js`, `libreria.js`, `crates/mzlab-feat/src/{recipe,trace,align}.rs`, `tools/bench/corri_mzmine.py`, `tools/valida/{abbina,cancello_misure}.py`, il repository privato dei dati (stato, fattibilità mzmine, consegne del benchmark) e il sorgente di mzmine (`mzmine/mzmine`, tag `v3.9.0` e `master` del 15/10/2026). Prompt dei pacchetti: `docs/studi/PROMPT_ATTRIBUTIO/`.

## 0. Verdetto
- **La concordanza con mzmine è bassa per definizione delle righe e per il protocollo di misura, non per le soglie.** La nostra pipeline (canali log-ppm a 4 ppm, segmenti con buchi ≤ 2 scansioni, tracce di fondo scartate, allineamento a catena con una riga per famiglia) e quella di mzmine (cromatogrammi ADAP su tutta la corsa, risolutore a minimo locale, join aligner uno a uno, una riga per ione) contano cose diverse; in più il confronto filtra le aree in unità diverse e mette a confronto righe per famiglia con righe per ione (§2.3).
- **Strada**: una seconda ricetta, `untargeted`, scritta DA ZERO in Rust leggendo gli algoritmi di mzmine 3.9.0 (licenza MIT), con parità per stadio contro le uscite di mzmine sui sintetici e concordanza sui 13 file Timolol. La ricetta di oggi resta quella di Tabui. Il codice nuovo va in un `.wasm` a parte (`untargeted`), così il motore, `detect` e `finder` non crescono.
- **L'annotazione non aspetta il rilevamento**: ricerca in libreria, formule in lotto, identità e livello 2b guidato lavorano su qualunque tabella (nostra o importata da mzmine/FBMN/MS-DIAL, che mzLab legge già). Se il cancello di concordanza non si apre, Attributio parte comunque dalle tabelle importate.
- **Il 2b resta del chimico**: il programma prepara le prove diagnostiche (frammenti spiegati dalla struttura proposta, sottoformule, isotopi, alternative per formula, controlli), non propone mai 2b né 1; la decisione è registrata con motivazione, autore e data, in coda (mai riscritta), ed esportata.

## 1. Inventario: cosa c'è e cosa manca

| Fase | In mzLab oggi | Dove | Manca per Attributio |
|---|---|---|---|
| Ingresso | mzML, `.raw`, `.wiff`, `.D`, altri formati per famiglia → `.mzidx` (MS1 in `f32`, tabella delle scansioni con precursore e finestra) | `mzlab-core`, `lettura-nativa.md`, `TP_Mine/files/finder-idx.js` | niente (i DDA grandi li porta `RAW_VELOCE`) |
| Feature (tracce) | canali log-ppm 4 ppm su due griglie, buchi ≤ 2, fondo > 400 scansioni scartato, lobi FT, `peakq` per la qualità | `mzlab-feat/src/{trace,channels,post,quality}.rs` | un costruttore di tipo ADAP (§2.1) |
| Risoluzione | taglio dei massimi su tracce lunghe o di forma < 0,9 (valle < 50 %, apici ≥ 0,1 min) | `resolve.rs`, `peakq::detect_prominence` | il risolutore a minimo locale con soglia cromatografica (§2.2) |
| Isotopi | gruppi nel file (13C, +2), averagine, mai scarto | `isotopes.rs`, `isoscore.rs` | gruppi per riga allineata alla mzmine (isotope grouper), carica |
| Allineamento | catena m/z 5 ppm + taglio di RT 0,1 min; per famiglia (`famalign.rs`, predefinito) o per ione; mappa di RT ad ancore | `align.rs`, `famalign.rs`, `rtmap.rs` | join aligner a punteggio, uno a uno per file (§2.3) |
| Riempimento | integrazione alla soglia nella finestra di RT con mappa | `gapfill.rs` | peak finder alla mzmine (opzione della ricetta `untargeted`) |
| Addotti, ISF, identità | famiglie per file, grafo delle identità con prove degli archi, coppie POS/NEG | `family.rs`, `evid/`, `finder` (`identity_graph`, `polarity_pairs`) | collegarle alla tabella `untargeted` |
| MS2 → feature | legami con purezza; nella tabella importata `ms2{mz,it}` | `ms2.rs`, `25_untarget.js` | spettro rappresentativo e unito fra scansioni e campioni (§2.6) |
| Librerie | `.mzlib`, entropia (Flash), coseno pesato, coseno GNPS, coseno modificato, analoghi, FDR con esche, pacchetti pubblici | `mzlab-lib`, `libreria.js`, `livelli-1.js` | niente nel motore; manca il lotto dalla tabella nostra (oggi `UNTM.run` lo fa sulle importate) |
| Livelli | cascata «mzlab» e «attributio» (`livelli/1`), punti delle prove, «proposto» e «deciso» | `livelli.js`, `livelli-1.js`, `UNTM.decide` | decisione persistente con motivazione, autore, data e prove (`decisioni/1`) |
| Formule | composizione elementare, punteggio isotopico fine, vincolo dai frammenti, sottoformule, albero MSn | `mzlab-core/src/{composition,isoscore,subformulas,msntree}.rs` | formule in lotto per tutte le righe, con punteggio MS2 (frazione spiegata) e regole euristiche |
| Struttura | grafo SMILES, tagli dei legami, struttura parziale, Ketcher | `mzlab-feat/src/chem/`, `grafo` `.wasm`, Disegno | spiegazione di uno spettro MS2 da una struttura (tagli + spostamenti di H) |
| Candidati per formula | PubChemLite 3.3.0 e NORMAN SusDat (CC BY 4.0) nell'indice dei sospetti | `sospetti.md` | ricerca «tutte le strutture con questa formula» per le alternative del 2b |
| Importazione | mzmine (completa, annotazioni, quant FBMN, archi IIN), MGF, MS-DIAL, GNPS2, SIRIUS no | `crates/mzlab-feat/src/import/`, `import` `.wasm` | risultati SIRIUS/CSI:FingerID come prova esterna (facoltativo, dopo) |
| Esportazione | xlsx/CSV/MGF, GNPS2 FBMN zip, SIRIUS `.ms` | `28_untarget_export.js`, `30_untarget_fair.js` | colonne della decisione, rapporto per livello, `mzid/1` |
| Interfaccia | finestra «Match a feature table», raggruppamento per identità, «Cos'è?», specchio | `27_untarget_ui.js`, `29_untarget_identity.js` | Banco 2b (§3) |

## 2. mzmine: algoritmi, confronto, causa della concordanza bassa
**Licenza**: `LICENSE.txt` di `mzmine/mzmine` (sia `v3.9.0` sia `master`) è la **MIT**, «Copyright (c) 2004-2022 The mzmine Development Team». Il programma 4.x chiede un account, il codice resta MIT. La MIT permette di leggere, tradurre e ridistribuire; noi riscriviamo da zero in Rust e citiamo la fonte nel commento di modulo e in `LICENZE-TERZI.md` (§2.8). mzmine usa CDK (LGPL) per le formule: da CDK non si prende nulla, le formule restano nostre (`composition.rs`).
**Versione di riferimento**: `v3.9.0` (quella che gira nel cloud senza login, `tools/bench/corri_mzmine.py`). Fra 3.9.0 e `master` cambiano il risolutore (`adjustStartAndEnd`: i bordi si allargano di un punto verso lo zero) e l'organizzazione del join aligner (`BaseFeatureListAligner`); l'ADAP no. La ricetta nostra ha un interruttore `compat = V3_9 | V4` solo dove differiscono.

### 2.1 Costruttore ADAP (`featdet_adapchromatogrambuilder/ModularADAPChromatogramBuilderTask.java`, `ADAPChromatogram.java`)
- Tutti i punti della lista di masse MS1 (sopra la soglia di rumore della mass detection) di tutte le scansioni si ordinano per intensità decrescente (ordinamento stabile: a pari intensità resta l'ordine scansione → m/z).
- Ogni punto entra nel cromatogramma il cui intervallo di m/z lo contiene (`TreeRangeMap`); se non c'è e il punto è ≥ `minHighestPoint`, apre un cromatogramma con intervallo = tolleranza attorno al SUO m/z, tagliato agli intervalli vicini (`startNewChromatogramLimitMzRanges`; intervallo vuoto ma bordi uguali → il punto va al vicino superiore).
- Un cromatogramma tiene un punto per scansione (il primo arrivato, cioè il più intenso); m/z = media dei punti. Copre tutta la corsa: i buchi NON lo spezzano.
- Tenuto se ha ≥ `minimumConsecutiveScans` scansioni consecutive con intensità ≥ `minGroupIntensity` e almeno un punto ≥ `minHighestPoint`; poi uno zero ai lati di ogni buco (`addNZeros(scans, 1, 1)`).
- **Noi**: canali fissi a 4 ppm su due griglie e somma di tutti i picchi del canale nella scansione; segmenti spezzati ai buchi > 2; tracce oltre 400 scansioni = fondo, scartate. Diversi: centro degli intervalli (seme contro griglia), un punto per scansione contro la somma, buchi, fondo.

### 2.2 Risolutore a minimo locale (`featdet_chromatogramdeconvolution/minimumsearch/MinimumSearchFeatureResolver.java`, `AbstractResolver.java`, `MathUtils.calcQuantile`)
- L'ingresso è il cromatogramma su TUTTE le scansioni MS1 della lista, con zeri dove manca (`FeatureFullDataAccess`).
- Soglia cromatografica = quantile `chromThreshold` (0,85) delle intensità con gli zeri; sotto la soglia l'intensità diventa 0. Effetto: in un cromatogramma rado la soglia è 0, in una traccia di fondo presente in tutta la corsa resta solo ciò che sporge.
- Regioni fra zeri; dentro una regione un punto è minimo locale se nessun punto entro ±`searchRange` (0,1 min) è più basso e il massimo della regione fino a lì è ≥ `minRatio` (1,7) volte quel minimo: lì la regione si chiude.
- Una regione è un picco se ha ≥ `minDataPoints` punti, massimo ≥ max(altezza assoluta, relativa × massimo), massimo ≥ `minRatio` × i due bordi e durata in `peakDuration` (0-1 min). Un solo zero (una scansione mancante) chiude la regione.
- **Noi**: taglio solo per tracce lunghe o di forma < 0,9, valle < 50 % del più basso, apici ≥ 0,1 min; nessuna durata massima; il fondo non viene «tagliato sopra», viene scartato.

### 2.3 Join aligner (`align_join/`, `align_common/BaseFeatureListAligner.java`, `FeatureListUtils.getAlignmentScore`)
- Liste in ordine di righe decrescenti; la prima è la base. Per ogni riga delle altre liste, candidati della base entro tolleranza di m/z e RT; punteggio = Σ pesi × (1 − |Δ|/semiampiezza) / Σ pesi (m/z peso 3, RT peso 1).
- Tutti i punteggi in ordine decrescente; una riga si aggiunge alla migliore riga base che non ha ancora una feature di quel file (uno a uno per file). Le righe non abbinate diventano righe nuove e il ciclo si ripete finché le liste sono vuote.
- **Noi**: catene di m/z con passi ≤ 5 ppm (ampiezza ≤ 10 ppm), poi taglio dove il salto di RT supera 0,1 min; per (gruppo, file) si tiene la feature più grande e le altre spariscono nel gruppo. In più la ricetta predefinita è `Mode::Family`: **una riga per famiglia** (isotopi, addotti, frammenti in sorgente diventano membri).

### 2.4 Riempimento (`gapfill_peakfinder/multithreaded/`)
Per ogni cella vuota si cercano nei dati grezzi i punti entro m/z ± tolleranza e RT ± tolleranza attorno alla riga; il picco si accetta se la forma resta entro `intensityTolerance` da quella attesa e ha ≥ `minDataPoints`. Noi integriamo alla soglia con la mappa di RT: simile, non uguale.

### 2.5 Isotopi (`filter_isotopegrouper/IsotopeGrouperTask.java`)
Dalla riga più intensa, per ogni carica 1…max si cercano a destra (e a sinistra) righe a n × 1,003355/z entro m/z e RT; forma monotona facoltativa; vince la carica che spiega più righe (a pari merito la più bassa); gli isotopologhi si tolgono dalla lista e restano nel pattern della riga principale. Noi: gruppi nel file prima dell'allineamento, mai scarto: si tiene il nostro e si aggiunge la versione per riga, solo come segno (non toglie righe).

### 2.6 Annotazione con librerie (`id_spectral_library_match/`, `util/scans/similarity/`)
Precursore entro 0,001 Da o 5 ppm, frammenti entro 0,0015 Da o 10 ppm, precursore tolto (±4 Da), ≥ 4 segnali in comune, coseno pesato (pesi MassBank: intensità^0,5 × m/z^2) ≥ 0,7, segnali non accoppiati contati come zero; lo spettro della riga è scelto o unito fra le scansioni (`SpectraMergeSelectParameter`). **Noi**:
entropia pesata (Flash), coseno pesato (radici), coseno GNPS, coseno modificato, FDR con esche, livelli con prove: più completi. Manca solo lo spettro unito per riga (tutte le MS2 della feature nei campioni, picchi fusi entro 10 ppm, intensità relative, numero di scansioni che concordano), che serve anche al cancello «≥ 2 scansioni concordi».

### 2.7 Formule (`id_formulapredictionfeaturelist/FormulaPredictionSubTask.java`) e reti (`id_ion_identity_networking/`, `group_metacorrelate/`)
- Formule: enumerazione entro ppm (CDK), filtri euristici (le «sette regole d'oro» di Kind e Fiehn 2007: H/C, rapporti N/O/P/S su C, RDBE intero e nell'intervallo), punteggio isotopico (somiglianza del pattern), punteggio MS2 = frazione d'intensità dei frammenti spiegata da sottoformule. **Noi**: `composition.rs` + `isoscore.rs` + `subformulas.rs` già fanno tutti e tre i pezzi per UNA formula alla volta; manca il lotto per riga e le regole euristiche come filtro dichiarato.
- IIN e metaCorrelate (Schmid et al. 2021): correlazione di forma e d'intensità fra campioni + relazioni di massa → reti di ioni. Noi le abbiamo già (famiglie e identità con prove degli archi, riscritte da Attributio): si collegano alla tabella `untargeted`.

### 2.8 Perché la concordanza è 0,294 / 0,293 / 0,777 / 0,617
In ordine di peso stimato (da verificare con AT-00, che lo misura invece di supporlo):
1. **Righe per famiglia contro righe per ione**: la ricetta predefinita (`recipe.rs`, `mode: Mode::Family`) dà una riga per famiglia; il batch di mzmine (`corri_mzmine.py`) non ha isotope grouper né IIN, quindi ogni M+1, M+2, addotto e frammento in sorgente è una riga. Queste righe di mzmine non hanno compagno fra le nostre (richiamo giù) e le nostre famiglie hanno un m/z di riferimento che può essere un addotto (precisione giù).
2. **Popolazione del confronto senza un pavimento comune**: `concordanza_mzmine` tiene le righe con area ≥ 10 × 50 in unità diverse (mzmine: intensità × min; noi: conteggi × s, fattore 60) e senza legame con le altezze minime dei due programmi (mzmine 3·10⁴, la nostra ricetta automatica la sceglie dal file).
  Entrano così le righe vicine ai due limiti di rilevamento, dove il caso decide. La prova «soglie di mzmine abbassate, risultato identico» non distingue nulla: abbassare mzmine può solo aggiungere righe sue; un risultato identico fa sospettare che i parametri del batch non siano stati applicati (da controllare nel log del batch).
3. **Fondo**: le tracce > 400 scansioni le scartiamo; mzmine ne estrae i picchi che sporgono sopra il quantile 0,85. Nelle acque trattate con TiO2 gli ioni sempre presenti sono molti.
4. **Buchi e tagli**: mzmine chiude un picco a una sola scansione mancante e scarta picchi > 1 min; noi colmiamo fino a 2 scansioni e non abbiamo durata massima. Un picco debole con buchi è una riga da noi, zero o due da mzmine.
5. **Allineamento**: uno a uno per file (mzmine) contro «la feature più grande del gruppo» (noi): due feature vicine nello stesso file sono due righe in mzmine e una sola da noi.
6. **Aree** (Spearman 0,777, |log2| 0,617): somma dei picchi del canale per scansione contro un punto per scansione; integrazione su bordi diversi; celle riempite con due metodi; e gli abbinamenti sbagliati dei punti 1-5 entrano nella correlazione.
**Conclusione**: algoritmo (definizione della riga) e protocollo, non parametri. Per questo il piano rifà gli stadi alla mzmine con parità per stadio e corregge il protocollo (confronto per ione, pavimento comune in altezza, unità, confronto per file prima dell'allineamento, motivo di ogni riga non abbinata).

### 2.9 Attribuzioni da inserire (pacchetto ATTRIBUTIO_IN_MZLAB, distribuito nei pacchetti)
- `LICENZE-TERZI.md`, una riga nuova: «mzmine (The mzmine Development Team) | 3.9.0 (algoritmi letti al tag `v3.9.0`; differenze di `master` annotate nei moduli) | MIT | Algoritmi riscritti da zero in Rust, nessuna riga copiata: `crates/mzlab-feat/src/untargeted/{adap,minsearch,join,peakfinder,isogroup,merge,formula_rules}.rs`. Testo della licenza: https://github.com/mzmine/mzmine/blob/master/LICENSE.txt».
- Commento in testa a ogni modulo: «Rewritten from scratch after the algorithm of mzmine 3.9.0 `<percorso del file Java>` (MIT, © 2004-2022 The mzmine Development Team); no code copied. Paper: <citazione>.»
- Citazioni: MZmine 3 (Schmid et al., Nat. Biotechnol. 41:447, 2023, doi:10.1038/s41587-023-01690-2); ADAP (Myers et al., Anal. Chem. 89:8696, 2017, doi:10.1021/acs.analchem.7b00947); IIN (Schmid et al., Nat. Commun. 12:3832, 2021, doi:10.1038/s41467-021-23953-9); sette regole (Kind e Fiehn, BMC Bioinformatics 8:105, 2007, doi:10.1186/1471-2105-8-105); livelli (Schymanski et al., Environ. Sci. Technol. 48:2097, 2014, doi:10.1021/es5002105); entropia (Li et al., Nat. Methods 18:1524, 2021).
- I dati d'oro prodotti da mzmine sui file sintetici sono uscite del programma su dati nostri: si committano con una nota di provenienza (versione, batch XML, SHA-256 degli ingressi) in `tests/data/untargeted/mzmine/LEGGIMI.md`.
- `CITATION.cff`/`.zenodo.json` non si toccano (decide Federico).

## 3. Il flusso del chimico per il livello 2b
Schymanski 2014: **2a** = spettro di libreria o letteratura che concorda; **2b** = prova diagnostica (frammenti, comportamento in sorgente, contesto: progenitore, trasformazione) per cui nessun'altra struttura regge, senza standard né libreria; **3** = candidati possibili; **4** = formula univoca; **5** = massa esatta.

**Ingresso nel Banco 2b**: dalla tabella untargeted (nostra o importata) una riga con livello proposto 3 o 2a (le altre si possono aprire, ma il pulsante «decidi 2b» resta spento con la frase del perché, come `UNTM.decide`).

**Cosa vede** (una finestra, pannelli che esistono già dove possibile):
1. Testa: chiave `(polarità, id)`, m/z, RT, livello proposto con le prove di `livelli/1`, livello deciso (vuoto), impronta della tabella.
2. Picco: XIC della feature in ogni campione (dal `.mzidx` collegato per SHA-256; se il grezzo non c'è, l'avviso e solo i numeri della tabella), forma, campioni dove manca, bianchi.
3. MS1: pattern isotopico osservato contro quello della formula scelta (`isoscore`), addotti e frammenti in sorgente della stessa identità (archi con prove, `UNTID`).
4. MS2: spettro unito della riga (numero di scansioni e campioni), specchio contro il miglior candidato di libreria se c'è; ogni frammento con la sua sottoformula (`subformulas`) e la sua spiegazione dalla struttura proposta (tagli di un legame o due, spostamenti di H ±1/±2); frazione d'intensità spiegata; frammenti non spiegati in evidenza.
5. Formula: le formule della riga in ordine di punteggio (massa, isotopi, MS2 spiegato, regole), con la differenza dalla seconda.
6. Struttura proposta: SMILES incollato, scelto fra i candidati per formula (PubChemLite/SusDat offline) o disegnato (Ketcher, Disegno); per ogni alternativa con la stessa formula lo stesso punteggio di spiegazione, così il chimico vede se un'altra struttura spiega altrettanto bene (la domanda del 2b: «nessun'altra regge?»).
7. Lista di controllo diagnostica: per ogni prova (frammento diagnostico, perdita neutra, isotopi, addotti coerenti, progenitore/trasformazione, RT plausibile, assenza nei bianchi) il chimico mette «a favore», «contro», «non valutata»; il programma precompila solo i numeri, mai il segno.

**Come conferma o rifiuta**: tre pulsanti: «2b» (richiede struttura, almeno una prova «a favore» scritta e una motivazione libera ≥ 20 caratteri), «resta 3» (con motivazione), «declassa» (a 4 o 5, con motivazione); «1» solo con uno standard segnato (regola che c'è già).
Ogni decisione è una voce nuova di `decisioni/1` (`{key, table_sha256, from, to, smiles, inchikey, evidence[{id, sign, value, note}], motivation, author, at, program_version}`); la più recente vale, le vecchie restano (storia visibile). Tastiera e lettori di schermo: ogni pulsante con `aria-label`, ordine di tabulazione dall'alto in basso, grafici con la loro tabella dei dati.

**Cosa si esporta**: CSV/xlsx con `level_proposed`, `level_decided`, struttura, motivazione, autore, data e prove in colonne; MGF con `LEVEL=`; `decisioni/1` come JSON nel `.mzflow`; un rapporto per livello (quante righe per livello, e per ogni 2b la scheda con spettro annotato e prove) in HTML stampabile; `mzid/1` quando lo schema sarà fissato (pacchetto A1 del piano Attributio-mzLab, fuori da questo studio).

**Domande di prodotto per Federico** (con la scelta predefinita che i prompt seguono finché non risponde):
- Q1 Visibilità: Attributio resta dentro mzFinder (5 clic sul logo) finché il cancello non si apre? Predefinito: sì; il Banco 2b sulle tabelle importate può diventare visibile prima.
- Q2 Nome: «Attributio» come scheda di mzLab (o di mzFlow) o programma a sé? Predefinito: scheda «Untargeted» nel mondo HR, nome nei documenti.
- Q3 Il programma può proporre un «2b?» (solo suggerimento) quando una struttura spiega molto più delle alternative? Predefinito: no, solo i numeri.
- Q4 Autore della decisione: testo libero (nome o sigla) ricordato nel browser, o niente? È richiesta una storia non modificabile (tracciabilità tipo BPL)? Predefinito: testo libero, storia in coda.
- Q5 Candidati per formula: bastano PubChemLite e SusDat offline (già nel sito per mzFinder) o serve PubChem intero solo nel desktop? Predefinito: i due offline.
- Q6 RT plausibile (logP stimato contro RT) come prova del 2b? Predefinito: no in questa serie (la scala di confidenza non lo chiede).
- Q7 Protocollo del cancello: si accetta il protocollo corretto (§2.8) accanto al vecchio, riportando entrambi? E il riferimento resta mzmine 3.9.0 nel cloud o Federico produce il CSV con mzmine 4 sul Mac? Predefinito: entrambi i protocolli, riferimento 3.9.0.
- Q8 POS e NEG nella stessa campagna: un'unica tabella con coppie di polarità o due tabelle? Predefinito: due tabelle e le coppie come prova (`polarity_pairs`).

## 4. Piano a pacchetti
Pesi attuali (byte, tetto): `detect` 419 435 (512 000), `proofs` 458 034, `finder` ~470 000, `finder_msn` ~320 000, `table` 219 750, `grafo` ~93 000, `import` (512 000), motore 1 439 240 (1 703 936).
**Nessun pacchetto di Attributio cambia il motore, `detect`, `proofs`, `finder` o `finder-lr`.** Il codice nuovo va in `crates/mzlab-feat/src/untargeted/` (modulo dietro la feature `untargeted` di `mzlab-feat`) e in un `.wasm` nuovo `untargeted` (feature omonima di `mzlab-feat-wasm`, cartella `wasm-feat/untargeted/`, voce `untargeted` del manifest, stesso limite 512 000 B, stima finale 250-380 KB). Le formule in lotto riusano `mzlab-core` senza modificarlo. Il Banco 2b aggiunge una funzione al `.wasm` `grafo` (93 KB → stima 110 KB).
`decisioni/1` aggiunge due funzioni al `.wasm` `table` (stima +10 KB).

| Pacchetto | Cosa | `.wasm` che cambia | Dipende da | Giorni-chat |
|---|---|---|---|---|
| AT-00 | Diagnosi della concordanza (protocollo corretto, motivi delle righe perse), dati d'oro di mzmine sui sintetici, file d'area `attributio.md` | nessuno | — | 1,5 |
| AT-01 | Costruttore ADAP in Rust | nessuno (solo crate) | AT-00 | 1 |
| AT-02 | Risolutore a minimo locale | nessuno (solo crate) | AT-00 | 1 |
| AT-03 | Join aligner + peak finder | nessuno (solo crate) | AT-00 | 1,5 |
| AT-04 | Ricetta `untargeted`, `.wasm` `untargeted`, worker, pannello Pipeline, desktop, cancello misurato | `untargeted` (nuovo) | AT-01, 02, 03 | 1,5 |
| AT-05 | Isotopi per riga + famiglie/identità sulla tabella `untargeted` | `untargeted` | AT-04 | 1 |
| AT-06 | Spettro unito per riga, MGF, annotazione automatica in lotto (librerie + livelli) dalla tabella nostra | `untargeted` | AT-05 | 1,5 |
| AT-07 | Formule in lotto (massa, isotopi, MS2 spiegato, regole), livello 4 | `untargeted` (il codice nasce nel crate in onda 1, si collega qui) | AT-06 | 1,5 |
| AT-08 | `decisioni/1`: schema, convalida in Rust, archivio nel taccuino e nel `.mzflow`, storia | `table` (`decisioni_append`, `decisioni_current`) | — | 0,5 |
| AT-09 | Banco 2b (interfaccia) + `explain_spectrum` nel `grafo` | `grafo` | AT-08 | 2,5 |
| AT-10 | Esportazioni e rapporto per livello | `table` (solo se il rapporto passa da lì; altrimenti nessuno) | AT-06, AT-08, AT-09 | 1 |

**Onde** (fino a 15 chat in parallelo se i file non collidono):
1. Onda 1: AT-00, AT-08, e la parte «crate» di AT-07 (punti 1-3 del suo prompt: `untargeted/formula.rs` senza `.wasm`).
2. Onda 2 (dopo AT-00): AT-01, AT-02, AT-03 insieme (moduli diversi; toccano tutti `docs/agenti/attributio.md` in righe diverse: chi unisce per secondo fa `git merge origin/main`). AT-09 dopo AT-08, in parallelo.
3. Onda 3: AT-04 (primo che crea il `.wasm` `untargeted`).
4. Onda 4, in serie sul `.wasm` `untargeted`: AT-05 → AT-06 → AT-07 (collegamento). AT-10 dopo AT-06 e AT-09.
- **Cancello**: AT-04 misura la concordanza; se non passa, AT-05…07 proseguono (servono anche alle tabelle importate) ma la ricetta `untargeted` resta nascosta (Q1) e AT-04 apre la domanda al Coordinatore con i numeri.

**Collisioni con i lavori in corso**: `RAW_VELOCE` (RV-01…RV-11) tocca motore, lettori, `finder-idx.js`, `tpmine-worker.js`, `20_tpmine.js`: Attributio non tocca nessuno di questi file (legge i `.mzidx` come oggi). `TABUI_MS2_LR` (M0-M5) tocca `legacy/lr/`, `finder-lr`, `similarity.rs` e il `.wasm` `finder` (M5): Attributio non li tocca. Formati nativi (DEV-*): solo lettori e motore.
File condivisi: `tools/build_feat_wasm.sh` (AT-04 aggiunge un blocco; M5 non lo modifica), `LICENZE-TERZI.md` (una riga, AT-01 la scrive, gli altri aggiungono i loro moduli alla stessa riga), `docs/agenti/mzfinder-untargeted.md` (AT-06, AT-10: righe diverse), `TP_Mine/js/27_untarget_ui.js` (AT-06, AT-09, AT-10: in serie per questo file, nell'ordine AT-09 → AT-06 → AT-10 se si sovrappongono nel tempo).

**Tempi**: circa 14 giorni-chat; con le onde, 5-6 turni di lavoro. Il collo di bottiglia è AT-00 (installare mzmine 3.9.0 e JDK 20 nel cloud, ore di calcolo sui 13 file: un solo strumento alla volta, disco pulito, `scratchpad/consegne/BENCH.md` del privato).

## 5. Rischi
| Rischio | Mitigazione |
|---|---|
| La parità per stadio con mzmine non è esatta (ordinamenti a pari intensità, virgola mobile, aree in minuti) | soglie per stadio ≥ 0,99 sui sintetici e ≥ 0,97 sui veri; differenze elencate per caso nei test |
| ADAP ordina tutti i punti del file per intensità: memoria | lastre di m/z separate da vuoti ≥ 2 × tolleranza (nessun intervallo può attraversarle), stessa uscita a ogni dimensione di lastra (prova) |
| Concordanza ancora bassa dopo AT-04 | il protocollo di AT-00 dice per ogni riga persa il motivo: il pacchetto successivo si sceglie dai numeri, non dalle ipotesi |
| Il 2b preso come verdetto | il programma non scrive mai 2b; motivazione obbligatoria; storia; esportazione separa proposto e deciso |
| Crescita del `finder` o del motore | tutto in `untargeted`; il peso entra nella descrizione di ogni PR |
