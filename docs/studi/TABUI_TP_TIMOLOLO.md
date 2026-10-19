# Tabui: una feature table dei prodotti di trasformazione, dal grezzo alla struttura (serie del timololo)

Studio dell'Architetto per Federico. Solo stato e proposte; nessun codice di prodotto. Prompt delle chat: `docs/studi/PROMPT_TABUI_TP_TIMOLOLO/`. Numeri veri (m/z, tempi, conteggi delle MS2, esiti): solo nel repository privato, `scratchpad/consegne/TABUI_TP_TIMOLOLO_DATI.md`, `scratchpad/timolol_truth.json` e `scratchpad/timolol_msn_truth.json`.

## 0. Verdetto
- **Una sola tabella**: la feature table della Pipeline (T3 di `TABUI_FEATURE_TABLE.md`: rilevamento alla mzmine 4.10 in Rust → `.mzfeat` → `tp/1` → `TPTAB` + riga espansa `TPRIGA`). Lo Scavo HR (T2, `legacy/`) non diventa la tabella: i suoi pezzi migliori (albero MSn del genitore, libreria dei frammenti, `localize`, `similarity`) si riusano come librerie dentro un passo nuovo.
- **Il pezzo nuovo è l'interpretazione per riga** (`interp/1`): un modulo Rust che, per ogni riga che passa i cancelli di `tp/1`, collega TUTTE le MS2 DDA della serie, le confronta col genitore (DDA e albero di infusione), propone la modifica, la localizza con una confidenza dichiarata e prepara le strutture da disegnare. Si scrive nel `.mzfeat` come blocco `interp` (come `tp/1`); la riga lo mostra nella scheda nuova «Structure» e nella scheda «MS2».
- **Il disegno**: il programma disegna il TP sul genitore (SMILES + Δ + sito). Sito certo: una struttura. Sito incerto: il genitore con gli atomi candidati colorati per classe e il punteggio di ciascuna, più le varianti («+O on N16», «+O on C18»…), con la scritta «Position uncertain». Le strutture le costruisce Rust (molfile con le coordinate del genitore); la pagina le disegna in SVG e le apre nel Disegno con `TPDraw.load` in un clic.
- **Il programma propone, il chimico decide**: livello proposto al più 3a/3b (`posiz/1`, `livelli.md`); 2b solo dal chimico (`TPDEC`); nessuna struttura è scritta come identificazione.
- **Il grezzo serve**: con i 13 mzML si fa quasi tutto; i `.raw` della serie servono per tre cose che l'mzML non dice (§2): energia di collisione per passo (stepped NCE), esclusione dinamica e lock mass. Si chiedono a Federico (§10).

## 1. Che cosa farebbe in più un professionista (oltre la richiesta)
1. **Purezza dell'isolamento**: per ogni MS2 la frazione dello ione precursore nella finestra di isolamento (MS1 prima e dopo, interpolata al tempo della MS2; precursor ion fraction). Sotto 0,8 lo spettro è marcato «chimeric» e non vota la posizione. `tp/1` ha già `NOTE_CHIMERIC_MS2` (purezza del `.mzfeat`); Attributio MS-04 (PR aperta) porta gli «intrusi chimerici».
2. **Isotopo M+2 dello zolfo**: il genitore ha uno zolfo; ³⁴S dà un M+2 riconoscibile anche con risoluzione 70 000. Il conteggio di S (e di Cl, Br) dall'MS1 è una prova di formula forte e quasi gratis: entra nella riga come «S count consistent / not».
3. **Energia di collisione multipla (stepped)**: nei file della serie la CE nel filtro è un valore medio con decimali, tipico di una stepped NCE: lo spettro è la somma di tre energie. Le intensità relative non si confrontano con l'infusione (CID in trappola, una energia per passo): si usano le FORMULE dei frammenti, non le intensità, per legare la MS2 del TP all'albero del genitore. Il metodo vero si legge nel `.raw`.
4. **Copertura DDA ed esclusione dinamica**: nei file la DDA è top-1 e i precursori più campionati sono ioni di fondo leggeri, ripetuti a pochi secondi: l'esclusione dinamica sembra assente o corta. Molti TP hanno MS2 solo in alcuni file o in nessuno. La riga dice per ogni TP in quanti file e a che tempo ha una MS2 («MS2 in 4 of 13 files»), e la vista elenca i TP formati senza MS2 come «needs targeted MS2» (lista d'inclusione da esportare per una nuova iniezione: il professionista la chiederebbe).
5. **Taglio della MS2**: il primo m/z è fisso e l'ultimo dipende dal precursore: i frammenti sotto il primo m/z non ci sono mai, e un frammento «assente» sotto quella soglia non è una prova contraria. La finestra di scansione si legge dal filtro e limita le prove.
6. **Errore di massa e calibrazione per file**: errore mediano in ppm per file su ioni noti (genitore, fondo, frammenti del genitore), con la stessa `Calibration` dell'albero MSn (`msn/space.rs::fit`); la tolleranza di formula si adatta (mai oltre 5 ppm). Un file con deriva diversa è segnalato.
7. **Falsi TP di sorgente**: addotti ([M+Na]+, [M+K]+, [M+NH4]+), dimeri [2M+H]+, ioni a più cariche e frammenti in sorgente coeluiti col genitore (un frammento del genitore può avere lo stesso m/z di un TP vero). Già in `tp/1` (cancello 2, ISF) e nelle famiglie di `untargeted/ident.rs`; la riga mostra il motivo, e un TP vero allo stesso m/z di un ISF resta visibile con la nota «coelutes with an in-source fragment of the parent».
8. **Isomeri non risolti**: due o tre isomeri allo stesso m/z e RT vicini (nella serie ce ne sono: il conteggio è nel privato). Prima di localizzare si separano le MS2 nel tempo: frammenti che salgono e scendono insieme = un isomero (correlazione delle tracce dei frammenti lungo il picco, deconvoluzione semplice). Se non si separano: «isomers not separated», nessuna posizione.
9. **Plausibilità dell'RT**: in fase inversa +O e la perdita di un gruppo apolare anticipano, la deidrogenazione di poco; un +O che eluisce dopo il genitore è un'anomalia da mostrare (non uno scarto). La stima di Attributio MS-04 (`rtpredict`) si usa solo come prova «Prev.».
10. **Cinetica come prova**: un TP di prima generazione cresce mentre il genitore cala, uno di seconda arriva dopo; un frammento in sorgente segue il genitore. Se il genitore satura l'ESI nei primi tempi (da controllare nel privato), la sua area non vale per un modello cinetico, la forma sì. `tp/1` ha forma e parentela; la riga aggiunge «generation suggested» (1 o ≥ 2) dal ritardo del massimo, come ipotesi.
11. **FDR con esche**: Δ di massa esca (le Δ delle regole spostate di pochi mDa, lontane oltre 10 ppm da ogni Δ vera: una riga che le abbina lo fa per caso) passate negli stessi cancelli: la frazione di esche sopravvissute stima i falsi positivi di ogni filtro; per la posizione, librerie esca con gli insiemi di atomi permutati fra i frammenti danno la distribuzione nulla del margine.
12. **Senza bianco di solvente**: la serie ha il buio (adsorbimento) e il t0 ma nessun bianco di solvente: contaminanti del sistema e carry-over non si separano dai TP presenti nel buio. Si chiede il bianco a Federico; nel frattempo il controllo è «dark + t0» e la riga lo dice.
13. **Una replica per tempo**: nessuna statistica fra repliche; la cinetica è descrittiva, nessun p-value nella riga.
14. **Prove che il programma non dà ma suggerisce**: MS3 del TP, scambio H/D, standard autentico (`posiz/1` `suggest`): restano suggerimenti nella scheda.

## 2. Il materiale più grezzo
| Materiale | Dove | Cosa dà che l'elaborato non dà | Stato |
|---|---|---|---|
| 13 mzML della serie (buio, t0…t120) | privato `HRMS/Timolol/` e Mac | MS1 a centroidi, DDA HCD con precursore, finestra di isolamento, CE nel filtro, intensità del precursore | c'è; DDA verificata in un file (privato, `TABUI_TP_TIMOLOLO_DATI.md`) |
| `.raw` della serie | non nel privato né sul Mac | stepped NCE per passo, esclusione dinamica, lock mass, AGC e tempo di iniezione, profilo (se acquisito), metodo strumentale | **da chiedere** |
| mzML dell'infusione MSn del genitore | privato | albero dei cammini MSn (CID in trappola), formule esatte dei frammenti | c'è; verità dell'albero in `timolol_msn_truth.json` |
| `.raw` dell'infusione | da chiedere | energia e attivazione per passo senza dubbi di conversione | da chiedere |
| Bianco di solvente | non c'è | contaminanti, carry-over | **da chiedere** |
| Tune e calibrazione del giorno | da chiedere | errore di massa atteso | facoltativo |
| Manoscritto (ET&I) e SI | privato `HRMS/Timolol/` | TP, frammenti, regioni: verità dell'autore | c'è; in `timolol_truth.json` |
| Letteratura indipendente | §8 | TP e frammenti pubblicati da altri | da estrarre (chat locale) |
| Oro di mzmine 4.10 | da generare sul Mac | confronto del rilevamento, non verità | da fare (chat locale) |

ThermoRawFileParser non è installato sul Mac: serve per leggere il metodo e le scansioni dai `.raw` (`-m` metadati, `-f 2` mzML con tutti i cvParam); mzLab ha il suo lettore nativo dei `.raw` (`docs/agenti/lettura-nativa.md`), che si confronta con lui.

## 3. Metodi e strumenti
| Strumento | Ruolo nello studio | Dove gira |
|---|---|---|
| ThermoRawFileParser | metadati e metodo dei `.raw`; mzML di confronto per il lettore nativo | Mac (chat locale), mai nel sito |
| mzmine 4.10 (batch di riferimento) | oro del rilevamento e dell'allineamento sugli stessi 13 file; concordanza di righe e aree con `untargeted/` | Mac; confronto in `tools/bench/` |
| SIRIUS (CSI:FingerID, alberi) | formule dei TP e albero di frammentazione indipendente; confronto delle formule dei frammenti con la libreria del genitore | Mac; Attributio legge già le sue tabelle |
| MetFrag / MS-FINDER | punteggio di ogni variante di struttura sulla MS2 del TP | in Rust: `attributio/ms2_engine/insilico.rs` (MS-02, PR aperta) e `rearrange.rs` (MS-03); MS-FINDER non gira (Windows) |
| RDKit | enumerazione indipendente dei siti e delle varianti (SMARTS di idrossilazione, N-ossidazione, deidrogenazione) per controllare `chem/posizioni.rs` | solo in `tools/` (riferimento di parità), mai nel sito |
| Coseno modificato ed entropia | `similarity::pair_components`: frammenti uguali, spostati della Δ, perdite comuni, con le due misure | Rust, riusato così com'è |
| `legacy/msn` | albero MSn del genitore (`build_tree`), libreria dei frammenti (`library::build`, anche con la DDA del genitore), `localize` (verosimiglianza sugli insiemi di atomi) | Rust, riusato |
| `chem/posizioni.rs` | siti chimicamente possibili per tipo di modifica, classi di simmetria, prove con/senza modifica dai tagli del grafo | Rust, riusato |

Da `pair_components` si prende la tipizzazione degli abbinamenti (uguale / spostato / perdita) per ogni picco del TP: il picco **uguale** è un frammento che non contiene la modifica (U di `localize`), lo **spostato** uno che la contiene (S). Oggi `localize` rifà questa classificazione per formula (`classify_fragments`); con l'infusione e la DDA del genitore le due vie si uniscono: la formula decide (HR, ≤ 5 ppm), il coseno resta la misura mostrata.

## 4. Flusso: estrazione → filtro → interpretazione
**Estrazione** (c'è): Pipeline della pagina (`32_pipeline.js`, `untargeted/pipeline.rs`): massa → cromatogrammi ADAP → risolutore a minimo locale → isotopi → allineamento join → riempimento → `.mzfeat` con riassunti, qualità e identità (`docs/agenti/mzfeat-wasm.md`). Ruoli e tempi da `campioni/2`: buio = `dark`, t0, tempi trattati, infusione = `infusion` (mai riferimento né trattato).

**Filtro** (in parte c'è): ogni filtro NASCONDE con un motivo, non cancella; il registro `filtri/1` (§6) dice soglia, quante righe e quali. Ordine:
| # | Filtro | Motivo | Dove |
|---|---|---|---|
| F1 | qualità del picco (forma, S/N, punti) | rumore | `qreason` del `.mzfeat` (c'è) |
| F2 | isotopi e addotti della stessa molecola | la stessa molecola due volte | `untargeted/isotopes.rs`, `ident.rs` (c'è) |
| F3 | presente nei controlli (buio, t0; bianco quando c'è) con rapporto < 3 | non formato dal trattamento | `tp/cancelli.rs` cancello 1 (c'è) |
| F4 | frammento in sorgente o addotto del genitore | artefatto di sorgente | `tp/cancelli.rs` cancello 2 (c'è) |
| F5 | errore di massa oltre la tolleranza del file | formula non sostenuta | nuovo, `interp` (calibrazione per file) |
| F6 | esche (solo misura) | stima dei falsi positivi di F1-F5 | nuovo, `tools/valida/` |
Una riga che passa F1-F5 entra nella vista dei TP; quelle nascoste si contano in «Hidden: N» con il motivo, e un clic le mostra.

**Interpretazione** (nuova, `interp/1`, §5): per ogni riga visibile, in quest'ordine:
1. **Modifica**: Δ della formula neutra rispetto al genitore dall'ipotesi migliore (`ipotesi/1`: regole, tagli, rete) e dalla formula dell'MS1 con l'isotopo (¹³C, ³⁴S); le alternative entro la tolleranza restano elencate.
2. **MS2 collegate**: tutte le scansioni DDA di tutti i file con centro di isolamento entro 10 ppm e tempo dentro i bordi del picco del file (`legacy/ms2.rs::select` con i bordi del `.mzfeat`), ciascuna con file, tempo, CE, finestra, purezza; consenso per file e per serie (`consensus`); gli isomeri separati per correlazione dei frammenti (§1.8).
3. **Confronto col genitore**: `pair_components` contro la DDA del genitore (stessa HCD) e contro i nodi dell'albero dell'infusione (per formula); coseno modificato ed entropia mostrati.
4. **Albero**: per ogni nodo dell'albero del genitore, il frammento nel TP: uguale (formula del nodo), spostato (formula + Δ), assente (sopra il taglio di scansione o sotto la soglia) o non verificabile (fuori dalla finestra di scansione).
5. **Posizione** (§7): siti possibili, punteggi, classe di confidenza, varianti da disegnare.
6. **Livello proposto e prove**: cascata di `livelli/1` più `proposal.sublevel` (3a/3b); cinetica, controlli, RT.

## 5. Modello dati della riga
- **Nessun cambio di `riga/1`**: `table_row` resta com'è; la rotta nuova `table_interp(id)` dà `interp/1` della riga (o `null` senza blocco) e `TPRIGA` li unisce. `.mzfeat`: meta `interp` (versione, parametri, impronta del genitore e dell'albero) + un testo JSON per riga compresso nel blocco, come `tp/1` sta nei suoi array (`store.rs`); una tabella senza blocco si legge come oggi; un blocco di versione più nuova segue `formato-nuovo.md`.
- **Schema `interp/1`** (`docs/schemi/interp-1.json`, nuovo):
```
{schema:"interp/1", id, parent:{smiles, ion_formula, mz},
 change:{delta:"+O", neutral_formula, ion_formula, ppm, source:"rule|cleavage|network|formula", alternatives:[{delta, ion_formula, ppm}]},
 ms1:{m1:{obs, exp}, m2:{obs, exp}, s_count:"consistent|inconsistent|unknown"},
 ms2:{scans:[{file, scan, rt, precursor, iso:[lo,hi], ce, scan_range:[lo,hi], purity, chimeric}], files_with_ms2, consensus:{mz:[], rel:[]}, isomers:"single|separated|not_separated"},
 vs_parent:{ref:"dda|infusion|both", cos_mod, entropy, equal:[mz], shifted:[mz], losses:[mz]},
 tree:{nodes:[{path:[mz], formula, state:"same|shifted|absent|not_checkable", mz}], n_same, n_shifted},
 site:{confidence:"site|region|uncertain|cannot_say", reasons:[code], region, margin,
       classes:[{id, atoms:[i], kind, support, status:"supported|undistinguished|excluded", for:[mz], against:[mz]}],
       variants:[{class, smiles, molfile}]},
 proposal:{sublevel:"3a|3b", why}, rt:{vs_parent:"earlier|later|same", note},
 kinetics:{shape, tmax, generation_hint}, flags:[{code, why}]}
```
- `support` di una classe è un numero fra 0 e 1 che somma a 1 fra le classi non escluse (softmax della verosimiglianza di `localize` con temperatura 1); si chiama «relative support», mai probabilità, e nell'interfaccia si scrive in decimi.
- Dove vivono i dati: `.mzfeat` in OPFS come oggi; l'albero MSn del genitore si calcola una volta dall'`.mzidx` dell'infusione e si tiene nel meta `interp` (`msnlib`: formule e insiemi di atomi dei nodi, poche decine di KB) per non rifarlo a ogni riga. Le correzioni del chimico (classe scelta, «not a TP») vanno in `TPDEC` (`45_tabui_decisioni.js`), non nel `.mzfeat`.

## 6. Registro dei filtri `filtri/1`
`{schema:"filtri/1", stages:[{id:"F3", name:"Formed by the treatment", threshold:{ratio:3, refs:["dark","t0"]}, hidden:N, ids:[...], why:"..."}], decoys:{rules:N, survived:N, fdr}|null}` nel meta `interp`; ogni riga nascosta porta il codice del primo filtro che l'ha nascosta (come `tp_status`). Nessun filtro senza testo d'aiuto (inglese, catalogo di Tabui `40_tabui_testi.js`). «Export view» esporta anche il registro (foglio «Filters»), per il materiale supplementare del paper.

## 7. La posizione: come si usa l'albero MSn del genitore
**Oggi ci sono due motori** e nessuno dei due basta da solo:
- `legacy/msn/localize.rs`: con l'albero e la libreria del genitore, ogni frammento del TP la cui formula è (uguale o spostata) una formula della libreria vota per gli insiemi di atomi che possono darla; verosimiglianza per sito, regione, margine. Pro: usa davvero l'infusione (insiemi di atomi vincolati dalla gerarchia dei cammini MSn). Contro: i nomi dei gruppi sono in italiano (`default_groups`: «anello A», «catena A»; da portare in inglese prima di mostrarli in Tabui); i siti sono maschere di atomi senza chimica (un +O può finire su un atomo che non lo accetta).
- `chem/posizioni.rs`: siti chimici per tipo (`aromatic_oh`, `aliphatic_oh`, `n_oxide`, `s_oxide`, `demethyl`, `carbonyl`, …), classi di simmetria, frammenti attesi dai tagli del grafo. Pro: chimica e simmetria. Contro: i tagli sono a un legame (non l'albero vero); non conosce l'infusione.

**Fusione (raccomandazione)**: i SITI vengono da `posizioni::sites` (più `-2H`: legame C-C sp3 adiacenti con H, da aggiungere) raggruppati per simmetria; i VOTI vengono da `localize` (insiemi di atomi della libreria dell'albero + DDA del genitore) quando c'è l'albero, altrimenti da `posizioni::match_spectrum` (tagli). Il punteggio di una classe è la verosimiglianza di `localize` ristretta ai suoi atomi; lo stato (`supported`/`excluded`/`undistinguished`) segue le regole di `posizioni` (un picco intenso spostato che la classe non può spiegare la esclude). Per le perdite (dealchilazione, asportazione di un anello) la «posizione» è la parte TENUTA (`Found::retained`), disegnata col resto in grigio.

**Confidenza dichiarata** (`site.confidence`):
| Classe | Quando | Disegno |
|---|---|---|
| `site` | una sola classe `supported`, margine ≥ 2 unità di log-verosimiglianza sulla seconda, ≥ 3 frammenti informativi (≥ 5 % del picco base, formula ≤ 5 ppm), nessun frammento contro, purezza ≥ 0,8, isomero unico | la struttura del TP |
| `region` | tutte le classi entro il margine stanno nello stesso gruppo (anello o catena) | il genitore con la regione colorata e le varianti della regione; «Position uncertain within ring A» |
| `uncertain` | classi in gruppi diversi entro il margine | tutte le varianti, ognuna col suo supporto; «Position uncertain» |
| `cannot_say` | motivi sotto | nessuna variante; il motivo in parole |

Motivi di `cannot_say` (`reasons`): `no_ms2` (nessuna MS2 in nessun file: «needs targeted MS2»), `chimeric` (purezza < 0,8 in tutte), `isomers_not_separated`, `no_reference` (né albero né DDA del genitore), `few_fragments` (< 3 informativi), `multiple_changes` (+2O, +3O, +2O+2H: le combinazioni di siti sono troppe per dirne una; si mostra solo la regione se c'è), `ring_opening` (la modifica apre un anello: `posizioni` non apre anelli; si dice «ring A opened» se la regione è certa, senza struttura), `change_lost` (la Δ si perde come neutro in tutti i frammenti, es. H2O da +O: nessun frammento la porta), `unsupported_change` (Δ fuori dalle trasformazioni note a `posizioni`).

**Limiti onesti** (scritti nella scheda, una riga): isomeri di posizione dentro un anello intero non si distinguono con i frammenti; C-OH contro N-ossido sullo stesso pezzo si distinguono solo con prove dirette (`ox_type`, H/D, standard); un frammento allo stesso m/z per caso è più probabile a energie alte; la stereochimica non c'è mai.

## 8. Il disegno del TP
- **Coordinate**: il genitore ha bisogno di coordinate 2D una volta. Raccomandazione: le dà Ketcher (il Disegno, già nel sito, sandbox): al primo disegno della serie la pagina chiama `TPDraw.load(smiles)` e ne legge il molfile, ricordato con l'impronta del genitore (IndexedDB `mzlab_lab`); se il chimico ha già disegnato il genitore («Use this structure»), si usa quel molfile. Nessun generatore di coordinate nuovo in Rust (costoso e peggiore di Ketcher).
- **Varianti in Rust**: `interp::variants(parent_molfile, transform, class)` → molfile del TP (atomo nuovo con coordinate sulla bisettrice esterna dell'atomo del sito, idrogeni impliciti tolti, ordine degli atomi del genitore conservato) e SMILES; formula verificata con `check_formulas`. Trasformazioni disegnabili: quelle di `posizioni` più `-2H`; le perdite si disegnano come «parte tenuta» (atomi persi in grigio tratteggiato); apertura d'anello e Δ multiple: niente disegno (§7).
- **Pagina**: `TP_Mine/js/46_tp_mol.js` (nuovo, `TPMOL.svg(molfile, {highlight:[{atoms, color, label}]})`): legami singoli/doppi/tripli e aromatici, etichette degli eteroatomi, colori Okabe-Ito da variabili CSS per le classi, simboli oltre al colore (numero della classe accanto all'atomo); `role="img"` con `aria-label` («Timolol with +O; candidate atoms: …») e la tabella delle classi come `TDATI`. Nessuna animazione oltre una dissolvenza (rispetta `prefers-reduced-motion`).
- **Pochi clic** (regola 26): la scheda «Structure» si apre già disegnata (nessun pulsante «Compute»); le varianti sono miniature in fila (la prima = classe col supporto più alto), un clic la ingrandisce, un pulsante con icona «Open in Draw» la apre nel Disegno con `TPDraw.load` (con il multicentro se incerta, come `TPDraw.partial()`). Nessuna conferma.
- **Disegno (chat C08)**: non si tocca `draw.js`, `disegno-parziali.js` né Ketcher: si chiamano solo `TPDraw.load` e il molfile che restituisce. Se serve una funzione nuova del Disegno, si chiede alla chat C08.
- **Prova esterna del disegno**: le posizioni pubblicate (§10) per i TP con sito noto; la variante col supporto più alto deve contenere il sito pubblicato o essere marcata `uncertain`/`region` con la regione giusta.

## 9. La riga espansa: cosa si aggiunge
Si tengono le schede di `TABUI_FEATURE_TABLE.md` §6 (Overview, Samples, Identity, Quality) e se ne aggiungono due, dopo Identity:
- **MS2**: tabella delle scansioni collegate (file con il colore del tempo, RT, CE, finestra, purezza come parola e numero, «chimeric»); spettro di consenso a specchio col genitore (uguali, spostati, perdite con tre segni diversi, non solo colori), coseno modificato ed entropia; la riga «MS2 in k of n files».
- **Structure**: la frase di testa («+O · Position uncertain · 3 classes · relative support 0.6 / 0.3 / 0.1»), il disegno (§8), la tabella delle classi (atomi, tipo, supporto, stato, frammenti a favore e contro), l'albero del genitore con i nodi trovati uguali/spostati (riuso di `23_tpmine_albero.js` con i dati di `interp/1`), i limiti in una riga, i suggerimenti (`suggest`).
Testa della riga chiusa: la colonna «Identity» mostra «+O (ring A?)» quando la confidenza è `region`, «+O (?)» quando `uncertain`; mai un nome di composto.

## 10. Prove esterne e piano di prova
**Prove che non dipendono da noi**:
1. **Letteratura indipendente** (TP e frammenti pubblicati, da estrarre con DOI nel privato): Chen et al. 2013, fotodegradazione sensibilizzata del timololo, taglio C-O della catena e ossidazione dell'anello a sei ([doi:10.1016/j.jhazmat.2013.02.035](https://doi.org/10.1016/j.jhazmat.2013.02.035)); Ofrydopoulou et al. 2021, fotolisi di una miscela con il timololo, Orbitrap ([doi:10.1016/j.scitotenv.2020.144079](https://doi.org/10.1016/j.scitotenv.2020.144079)); Lejwoda et al. 2024, fotodegradanti con HRMS/MS ([doi:10.3390/ph17010098](https://doi.org/10.3390/ph17010098)); Devrukhakar et al. 2018, prodotti di idrolisi e fotolisi ([doi:10.1016/j.jpba.2018.02.057](https://doi.org/10.1016/j.jpba.2018.02.057)); per la chimica della morfolina, Tekle-Röttering et al. 2016 ([doi:10.1016/j.watres.2015.11.027](https://doi.org/10.1016/j.watres.2015.11.027)). Fonte: PubMed. Il manoscritto di Federico è una verità, ma dello stesso autore: conta come «autore», non come indipendente.
2. **Standard autentici**: se il laboratorio ha uno standard di un TP (es. un prodotto di dealchilazione in commercio), RT + MS2 = livello 1 e verità piena del sito. Da chiedere a Federico.
3. **Altri programmi sugli stessi file**: mzmine 4.10 (righe, aree, allineamento), SIRIUS (formule dei TP e dei frammenti); concordanza misurata, non uguaglianza richiesta.
4. **Prova cieca**: un'altra serie della cartella HRMS con sito noto (scelta da Federico senza dire i TP alla chat che sviluppa), o un composto con un sito d'idrossilazione unico e pubblicato; i parametri si congelano prima.
5. **Coerenza cinetica**: i TP di prima generazione hanno il massimo prima di quelli di seconda; nessun TP accettato segue il genitore con rapporto costante (sarebbe ISF).
6. **Esche**: FDR stimato per F1-F5 e per la classe `site` (librerie permutate): obiettivo FDR ≤ 10 % per `site`.
7. **Revisione di Federico**: i primi 20 TP della vista in Tabui, con `TPDEC`: per ciascuno «agree / disagree / not a TP» su modifica, confidenza e sito; le risposte si esportano e diventano verità del privato.

**Criteri di accettazione** (i numeri attesi stanno nel privato): almeno l'80 % dei TP di `timolol_truth.json` con RT osservato nella vista; nessun ISF noto accettato come TP senza la nota; per i casi `loc_test` la regione giusta in `site` o `region` (come `timolol_msn_truth.json` `localization.cases`, `min_correct`), nessun `site` sbagliato; i casi ambigui (`ambiguous_ids`) mai `site`; isomeri della componente tripla separati o `isomers_not_separated`; concordanza delle righe con mzmine misurata e scritta in `misure.md`.

**In CI (dati sintetici)**: serie sintetica di `tools/dati_sintetici.py` estesa con un genitore che ha un sito unico noto e un isomero di posizione, un'infusione MSn sintetica (`finder_msn_e2e.json` esiste) e MS2 DDA chimeriche; test `cargo test -p mzlab-feat --test interp*` (parità con un riferimento Python in `tools/interp_riferimento.py` che usa RDKit per i siti, se installato, altrimenti salta solo quella parte); e2e `e2e_tabui_struttura.py` (scheda Structure, disegno, tastiera, axe). **Con i dati veri** (salto senza `MZLAB_DATI`): `tests/interp_dati.rs` legge le due verità del privato; `tools/valida/tp_timololo.py` scrive le metriche nel privato (`scratchpad/consegne/`).

## 11. Mappa del codice
**Da toccare** (righe da ricontrollare con `grep -n` al momento):
- `crates/mzlab-feat/src/legacy/msn/localize.rs`: `default_groups` (nomi in inglese) e un'entrata pubblica che accetta siti dati (maschere da `posizioni`) invece di `candidate_sites`; nessun cambio di numeri sulle prove esistenti.
- `crates/mzlab-feat/src/legacy/ms2.rs`: `select` con i bordi del `.mzfeat` (è già la forma); una funzione per la purezza dall'MS1 (se Attributio MS-04 non la dà riusabile).
- `crates/mzlab-feat/src/chem/posizioni.rs` e `posizioni/`: trasformazione `-2H`; accesso alle maschere dei siti per `localize`.
- `crates/mzlab-feat/src/store.rs`: scrittura/lettura del meta `interp` (come `write_table_tp`).
- `crates/mzlab-feat-wasm` (feature nuova `interp`), `tools/build_feat_wasm.sh` (una riga, tetto comune 512 000 B), `TP_Mine/files/feat-worker.js` e `feat-desktop-pool.js` (una rotta), `crates/mzlab-desktop/src/feat.rs`.
- `TP_Mine/js/44_tp_riga.js` (due schede), `42_tp_table.js` (testo di Identity, «Hidden: N»), `40_tabui_testi.js` (aiuti), `23_tpmine_albero.js` (dati da `interp/1`: solo l'ingresso).
**Da inventare**:
- `crates/mzlab-feat/src/interp/` (`mod.rs`, `ms2link.rs`, `confronto.rs`, `albero.rs`, `sito.rs`, `varianti.rs`, `filtri.rs`, `json.rs`), prove `crates/mzlab-feat/tests/interp*.rs`; `docs/schemi/interp-1.json`, `filtri-1.json`; `docs/agenti/interp.md` (file d'area nuovo).
- `.wasm` `interp` (feature di `mzlab-feat-wasm`, cartella `wasm-feat/interp/`): lettura del `.mzfeat` e degli `.mzidx`, albero MSn, `localize`, `similarity`, `posizioni` + grafo. Stima: 380-460 KB dopo `wasm-opt -Oz` (`finder_msn` 320 KB + grafo/posizioni circa 80 KB + store); sotto il tetto di 512 000 B. **Cancello**: se la prima misura supera 470 000 B, l'albero si costruisce col `.wasm` `finder_msn` esistente (che esporta `msnlib`) e `interp` legge solo la libreria. Caricato solo con SMILES del genitore e modalità amministratore.
- `TP_Mine/js/46_tp_mol.js` (disegno SVG), `tests_e2e/e2e_tabui_struttura.py`.
- `tools/valida/tp_timololo.py`, `tools/interp_riferimento.py` (RDKit, solo test).
**Da non toccare**: `chem` `.wasm` (569 344 B, non alzabile), motore (1 703 936 B), `sciex` (327 680 B); `crates/mzlab-feat/src/attributio/` (chat MS, PR aperte: si usano `insilico`, `rearrange`, `chimeric` solo dopo l'unione, senza cambiarli); `mzlab/web/draw.js`, `disegno-parziali.js` (C08); `mzlab/web/teoria/`, intestazione (chat interfaccia/Teoria); pannelli spettro della vista Dati; liste e registri di Attributio (RG); regole delle perdite e degli addotti LR/HR (chat perdite-addotti: `interp` legge le famiglie, non le cambia); `.github/workflows/`.

## 12. Pacchetti e chat
Regola 25: un pacchetto Rust e uno di interfaccia, ognuno in UNA chat che fa le sue PR in fila; la chat locale per i dati.
| Pacchetto | Chat | Cosa | Dipende da |
|---|---|---|---|
| TT-D | locale (Mac, `mz-loc`) | oro mzmine 4.10 dei 13 file; copertura DDA per TP e file; letteratura estratta (§10.1) in `timolol_lit.json`; metodo dai `.raw` quando arrivano | nulla |
| TT-R1 | R (Rust, Sonnet) | `interp/` passi 1-4 di §4 (modifica, MS2 collegate con purezza e isomeri, confronto, albero), blocco nel `.mzfeat`, `filtri/1`, rotta e `.wasm` `interp`, misura del peso | nulla (parallelo a TT-D) |
| TT-R2 | R (stessa chat) | posizione (§7): fusione `posizioni` + `localize`, confidenza, motivi, varianti (molfile), `-2H` | TT-R1 unita |
| TT-R3 | R (stessa chat) | prove sui dati veri e `tools/valida/tp_timololo.py`, esche e FDR | TT-R2, TT-D |
| TT-U1 | U (pagina, Sonnet) | schede MS2 e Structure, `46_tp_mol.js`, «Hidden: N» col registro, testi, e2e | TT-R1 per MS2; TT-R2 per Structure (due PR) |
| TT-P | Coordinatore + Federico | revisione dei primi 20 TP, frase e tabella per il paper | TT-U1, TT-R3 |
Prompt: `PROMPT_TABUI_TP_TIMOLOLO/R.md`, `U.md`, `LOCALE.md`. Conflitti: la chat MS (Attributio) tocca `attributio/` e i suoi `.wasm`; la chat U tocca solo `44_tp_riga.js`, `42_tp_table.js`, `40_tabui_testi.js`, `46_tp_mol.js`; il documento comune `mzfinder-tabella.md` si unisce tenendo le righe di tutti.

## 13. Decisioni (raccomandazioni; i prompt usano queste)
- D1 La tabella unica è la Pipeline (T3), non lo Scavo HR.
- D2 `riga/1` non cambia; `interp/1` è una rotta e un blocco a parte.
- D3 Coordinate del genitore da Ketcher, una volta per serie.
- D4 Intensità dell'infusione mai confrontate con la DDA: solo formule (CID contro HCD stepped).
- D5 Livello proposto al più 3a/3b; `site` non alza mai il livello oltre la proposta.
- D6 Controlli senza bianco: «dark + t0», detto nella riga, finché il bianco non arriva.
- D7 Δ multiple (+2O, +3O): solo regione, mai varianti combinatorie.

## 14. Trappole
- I m/z, i tempi e i nomi dei TP veri non vanno nel pubblico: i test con dati veri leggono la verità del privato e saltano senza (`eprintln!("SKIP ...")`).
- `default_groups` scrive nomi italiani: Tabui è solo in inglese (`tests/test_tabui_inglese.py`), quindi vanno tradotti prima di arrivare alla pagina.
- `localize` è in parità con il riferimento Python di mzFinder: un'entrata nuova si aggiunge accanto, la vecchia non cambia numeri.
- La CE «media» del filtro non è la CE dello spettro: non usarla come chiave di raggruppamento con l'infusione.
- Un frammento sotto il primo m/z della scansione non è mai «absent».
- `#tp-det` è unico (riga espansa): le schede nuove si disegnano dentro, senza un secondo nodo.
