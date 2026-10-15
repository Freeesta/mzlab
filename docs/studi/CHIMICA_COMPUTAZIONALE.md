# Chimica computazionale e posizione della sostituzione negli isomeri dei TP

Studio dell'Architetto sulla domanda di Federico: ha senso mettere in mzLab un calcolo di chimica computazionale (siti di attacco di OH•, altre reazioni) e altre prove scientifiche per capire DOVE sta la sostituzione in un isomero posizionale (per esempio i TP idrossilati)? Prompt dei pacchetti in `docs/studi/PROMPT_CHIMICA_COMPUTAZIONALE/`.

## 0. Riassunto (10 righe)
1. Risposta: **sì, ma ridotto**. Il valore vero non sta nel calcolo quantistico ma nel confronto ordinato tra le posizioni possibili e lo spettro MS2 misurato.
2. Gli indici di Fukui e la DFT dicono «dove è più probabile» l'attacco di OH• su molecole semplici; sui TP reali sono qualitativi, cambiano con il metodo e non sono mai una prova di struttura.
3. Su TiO2 l'attacco non è solo OH• libero in soluzione (lacune, superficie, adsorbimento): una previsione fatta in acqua omogenea vale ancora meno.
4. Nessun calcolo pesante (xtb, CREST, DFT) può girare nel browser in modo sensato: Fortran, minuti o ore per molecola, megabyte di `.wasm`. Si può solo importare un calcolo fatto fuori.
5. La prova più solida per localizzare un -OH resta la MS2: se un frammento porta +16 Da, l'OH sta in quel pezzo. Questo si può calcolare in Rust con il grafo SMILES che mzLab ha già.
6. Di solito la MS2 dice «in quale anello o catena», quasi mai «in orto o in para»: il programma deve mostrare le classi di posizioni che lo spettro non sa distinguere (livello 3a), non sceglierne una.
7. RT previsto e CCS previsto hanno errori più grandi della differenza tra isomeri posizionali: servono a scartare, non a decidere. Il 3200 QTRAP non ha mobilità ionica.
8. Prove sperimentali economiche e provate: scambio H/D (D2O) per distinguere C-OH da N-ossido, MS3 e rapporti tra ioni a più energie di collisione sul QTRAP, standard di riferimento quando esistono.
9. Proposta: fase 1 (circa 3 giorni-chat) «posizioni compatibili con lo spettro» in Rust, solo in Tabui (amministratore) e pronto per il Banco 2b; fase 2 facoltativa: una stima di reattività semplice, validata prima su dati veri, e l'importazione di calcoli esterni (xtb).
10. Niente sale da solo a 2a o 2b: il programma propone classi 3a/3b con le prove; il 2b lo assegna solo il chimico. Nel mondo LR degli studenti non compare nulla.

## 1. Cosa c'è già in mzLab (da non rifare)
- **Grafo SMILES in Rust** (`crates/mzlab-feat/src/chem/smiles.rs`, `grafo.rs`, `gruppi.rs`): lettura di SMILES e molfile, formula, gruppi funzionali, `Mol::cleavages()` (ogni legame singolo fuori dagli anelli, con le formule dei due lati) e `fragment_formulas()`. Gli anelli non si aprono. Nel sito sta nel `.wasm` `grafo` (caricato solo quando l'utente dà un SMILES).
- **Disegno** (`docs/agenti/disegno.md`): frammenti di un legame, strutture parziali con Δ («+O») e **multicentro** («attacco su uno di questi atomi», V3000 con `ENDPTS`): mzLab sa già rappresentare «OH su uno di questi atomi», che è proprio una classe di isomeri posizionali (livello 3 per `disegno-parziali.js`).
- **Tabui** (mzFinder, solo amministratore): righe dei TP con MS2 abbinate in LR (`lr/ms2link.rs`, `lr/ms2.rs`) e in HR (`legacy/ms2.rs`), albero dei frammenti del genitore (`lr/tree.rs`).
- **Attributio** (in corso, `docs/studi/ATTRIBUTIO.md`): AT-06 porta RT contro logP e il frammentatore; AT-07 il Banco 2b con la lista di controllo («alternative con la stessa formula escluse»). Il Banco 2b non esiste ancora.
- **Livelli** (`docs/agenti/livelli.md`): 2b solo dal chimico; 3a = pochi candidati posizionali tra omologhi; 3b = classi generiche. Scala di previsione «Prev. N» (Löffler et al. 2025) separata da quella di Schymanski.
- Nessuno studio in `docs/studi/` tratta questo tema: non c'è nulla di già deciso.

## 2. Reattività verso OH•: cosa dice la letteratura
**Fukui condensati (f⁰, f⁻)**. Molto usati negli articoli sui TP di processi avanzati di ossidazione, quasi sempre per spiegare a posteriori prodotti già trovati, non per prevederli alla cieca. Un confronto su eterocicli azotati mostra che la funzione di Fukui radicalica dipende dallo schema di popolazione (Hirshfeld, Mulliken…) e sbaglia spesso il sito su substrati disostituiti (Barrera et al. 2025). Non ho trovato un banco di prova sistematico di f⁰ contro prodotti misurati di micro-inquinanti: l'accuratezza reale non è nota.

**DFT con stati di transizione**. Funziona su casi semplici: per il fenolo M06-2X prevede 77 % orto e 23 % para, come l'esperimento (Dwinandha et al. 2022). La rassegna di Tentscher, Lee e von Gunten (2019) mostra che il calcolo quantistico aiuta a capire meccanismi e cinetiche, ma chiede validazione sperimentale caso per caso (solvatazione, conformeri, scelta del funzionale). Costo: ore o giorni di CPU per molecola e per sito, con un esperto che guida il calcolo.

**Contributi di gruppo** (Minakata et al. 2009): costanti di velocità k(OH•) da 66 costanti di gruppo e 80 fattori, entro un fattore circa 2 su gran parte dei composti; danno il peso relativo dei meccanismi (astrazione di H, addizione all'anello, ad alcheni, a N/S/P) ma non la posizione nell'anello. La rassegna Minakata et al. 2023 riassume i generatori di vie di reazione: regole sì, posizione precisa ancora debole.

**Apprendimento automatico**. QCD-MLP (ACS ES&T Water 2025) prevede i TP di sistemi con OH• e Cl• da descrittori DFT: AUC 0,93 su 345 reazioni con OH•, escluse le reazioni successive. Radical-Net (2026) è un trasformatore per reazioni radicaliche elementari. Holo-Mol (2024) e simili prevedono k(OH•), non il sito. Per la sostituzione elettrofila aromatica (chimica vicina, ma non uguale) RegioSQM20 e RegioML arrivano al 90-96 % sul sito, con xtb (Ree et al. 2021, 2022). Gli strumenti di «sito di metabolismo» (SMARTCyp, FAME 3) valgono per il citocromo P450, non per OH• in acqua.

**Il caso TiO2**. Nella fotocatalisi su TiO2 lavorano OH• libero e legato alla superficie, lacune (trasferimento diretto di elettroni) e l'adsorbimento orienta la molecola (Turchi e Ollis 1990). Una previsione «OH• in acqua» è quindi un'ipotesi in più, non il meccanismo. La prospettiva di Löffler et al. (2025) sugli strumenti in silico per i TP indica come limiti principali il dominio di applicabilità, i dati sbilanciati e l'incertezza del meccanismo.

**Conclusione per §2**. Una stima di reattività può ordinare le posizioni («più probabile/meno probabile»), con un'accuratezza che sui micro-inquinanti nessuno ha misurato bene. È un indizio a priori, non una prova, e non decide mai tra due isomeri che lo spettro non separa.

## 3. Prove per localizzare la sostituzione da MS (provate e no)
| Prova | Cosa dice | Provata? | Con il nostro strumento |
|---|---|---|---|
| Frammenti diagnostici MS2 (spostamento di +15,995 Da in un frammento) | in quale parte della molecola sta l'O | sì, è la base del livello 3 (Schymanski et al. 2014) | sì, anche a risoluzione unitaria (+16 Da) |
| Perdite neutre tipiche (H2O da OH alifatico, [M+H−O]+ da N-ossido in APCI) | tipo di ossidazione: C-OH alifatico, fenolo, N-ossido | sì (Ramanathan et al. 2000) | parziale: ESI, una sola energia |
| Scambio H/D in D2O | un H scambiabile in più = C-OH; nessuno = N-ossido o S-ossido | sì (Cajka et al. 2026) | sì, è un esperimento di laboratorio economico |
| MS3 e rapporti tra ioni a più CE | separa isomeri posizionali con spettri MS2 simili | sì, su isomeri di droghe sintetiche (Murakami et al. 2018) | sì, il 3200 QTRAP ha la trappola lineare |
| Frammentazione in silico (CFM-ID 4, MetFrag, SIRIUS/CSI:FingerID) | ordina candidati con formule o strutture diverse | sì per l'identificazione generale (CASMI); debole tra isomeri posizionali, che hanno frammenti quasi uguali | solo HR, fuori dal sito |
| Spettri da calcolo quantistico (QCxMS2) | spettro previsto per ogni isomero | no per ESI-CID: provato solo in EI, 16 molecole, somiglianza media 0,70 (Gorges et al. 2025) | no |
| RT previsto (RepoRT, 2-step, Retip, QSRR) | ordine di eluizione atteso | parziale: errori da qualche decimo di minuto a più di un minuto, spesso più grandi della distanza tra isomeri | solo per scartare, meglio con standard propri |
| logP atomico (Wildman e Crippen 1999) | idrofobicità | sì in generale, ma orto/meta/para hanno quasi gli stessi contributi atomici | no per la posizione |
| CCS prevista | forma dello ione | no tra isomeri posizionali: i modelli 2D non li distinguono (Ross et al. 2022) | il 3200 non ha mobilità ionica |
| Mobilità ionica misurata, derivatizzazione + IMS, reazioni ione-molecola, spettroscopia IR degli ioni, EAD | posizione anche senza standard | sì in laboratori specializzati (Shimizu et al. 2013; Palm et al. 2024) | no: strumenti che non abbiamo |
| Marcatura isotopica parziale | regioisomeri senza standard | sì in sintesi (Sojdak et al. 2025) | no per la degradazione |
| Standard di riferimento | identità (livello 1) | sì, è la sola prova definitiva | sì, quando si comprano o si sintetizzano |

Il punto chiave: la MS2 localizza l'O nel **pezzo** che lo porta. Se il pezzo è un anello aromatico intero, le posizioni dentro quell'anello restano indistinguibili con la sola MS2 e il risultato onesto è «OH su uno di questi atomi»: esattamente il multicentro del Disegno e il livello 3a.

## 4. Fattibilità nel nostro stack
| Opzione | Dove gira | Peso | Tempo per molecola | Licenza | Giudizio |
|---|---|---|---|---|---|
| A. Posizioni compatibili con la MS2 (enumerazione + spostamento dei frammenti) | Rust, `.wasm` `grafo` | stima < 40 KB in più | millisecondi | MIT (nostro) | **fase 1** |
| B. Tipo di ossidazione dalle perdite neutre e suggerimento H/D | Rust, stesso modulo | pochi KB | millisecondi | MIT | **fase 1** |
| C. Reattività qualitativa (Hückel π + classi di BDE C-H per l'astrazione) | Rust, `.wasm` `grafo` | stima < 60 KB | millisecondi | MIT | fase 2, solo se passa la validazione |
| D. Importazione di un calcolo esterno (CSV `siti/1`, uscita `--vfukui` di xtb) | Rust (lettore), calcolo fuori | pochi KB | lo decide chi calcola | xtb LGPL-3.0 usato come programma separato: nessun vincolo per noi | fase 2 |
| E. Contributi di gruppo k(OH•) (Minakata 2009) | Rust | piccolo | millisecondi | dati pubblicati | no per ora: dà la velocità, non la posizione |
| F. xtb/tblite/CREST nel browser | WASM da Fortran + LAPACK | diversi MB, nessuna compilazione WASM nota | minuti-ore (CREST) | LGPL-3.0 (link statico nel `.wasm` = obblighi di rilinkaggio) | **no** |
| G. DFT (ORCA, Psi4) | fuori | — | ore-giorni | ORCA proprietario, Psi4 LGPL-3.0 | solo come D |
| H. CFM-ID, MetFrag, SIRIUS nel sito | fuori (Java/C++) | decine di MB | secondi-minuti | SIRIUS AGPL-3.0; CFM-ID e MetFrag da verificare | no nel sito; i risultati entrano già da Attributio (AT-06) |
| I. RDKit nel browser (MinimalLib) | WASM | qualche MB | — | BSD-3 | no: abbiamo già il grafo in Rust |
| J. RT o CCS previsti per ogni isomero | — | — | — | — | no: errore maggiore della differenza |
| K. ML per i siti (QCD-MLP e simili) | Python + descrittori DFT | — | — | da verificare | no: chiede DFT per ogni molecola e il codice non è nel nostro linguaggio |

Licenze verificate sui file del repository a monte: xtb, CREST, tblite, Psi4 LGPL-3.0; SIRIUS AGPL-3.0; RDKit BSD-3. Le opzioni scelte (A-D) non includono codice di terzi: leggiamo numeri prodotti da altri programmi o calcoliamo con codice nostro.

## 5. Rischi scientifici e regole
- **Falsa precisione**: una barra colorata «sito più reattivo» su una struttura sembra una risposta. Regola: la stima di reattività (C, D) si mostra solo come «Prev.» e solo accanto alle classi compatibili con lo spettro; non riordina mai le classi e non toglie candidati.
- **Livelli**: la fase 1 produce classi di posizioni (3a se le classi sono poche e lo spettro ne esclude alcune, 3b se lo spettro non esclude nulla) come proposta; il livello deciso resta vuoto finché il chimico non decide. Nulla di computazionale sale a 2a o 2b.
- **Mondo LR degli studenti**: niente di questo nella vista Dati, nel Disegno degli studenti né nelle tabelle della relazione (principio 1). Solo Tabui e Attributio, dietro `uipAdminOn()`.
- **Limite del grafo**: le rotture sono solo di legami singoli fuori dagli anelli; un'apertura d'anello o un riarrangiamento non sono modellati. Un frammento osservato che nessuna rottura spiega non è una prova né contro né a favore: si scrive «non spiegato».
- **Risoluzione unitaria**: +16 Da si vede, ma un frammento a 1 Da da un altro può confondersi; la tolleranza e il numero di frammenti spiegati si mostrano sempre.
- **Simmetria**: posizioni equivalenti per simmetria (per esempio 2 e 6 in un anello para-sostituito) sono un solo candidato; un errore qui gonfierebbe il numero di isomeri.
- **Nomi**: nessun cognome di `nomi_vietati.txt` nei test e nei documenti (`tests/test_esempi_anonimi.py`); i composti di prova vengono dalla letteratura citata.
- **Teoria**: un capitolo sulla localizzazione dell'OH sarebbe utile, ma Teoria e Pratica si toccano solo con un'istruzione esplicita di Federico (domanda D4).

## 6. Raccomandazione: SÌ ridotto, a gradini
**Fase 1 (alto valore, basso costo, circa 3 giorni-chat)**
- **CC-01** (Rust, nessuna interfaccia): dato il genitore (SMILES) e la trasformazione (+O, −CH2, +O−2H), enumera le posizioni possibili (C-H aromatico, C-H alifatico, N-ossido, S-ossido; quale metile perso), le raccoglie per simmetria, calcola per ognuna i frammenti attesi con lo spostamento e le confronta con la MS2 osservata. Uscita `posiz/1`: classi di posizioni che lo spettro non distingue, con i frammenti che le sostengono o le escludono, il tipo di ossidazione dalle perdite neutre e gli esperimenti suggeriti (H/D, MS3).
- **CC-02** (`.wasm` `grafo` + Tabui, amministratore): pannello «Substitution position» nella riga di un TP con MS2, in LR (0,5 Da) e HR (ppm), con tabella dei dati; il contratto `posiz/1` resta pronto per il Banco 2b di AT-07.

**Fase 2 (facoltativa, parte solo con il sì di Federico dopo aver visto la fase 1)**
- **CC-03**: stima di reattività qualitativa in Rust (densità di frontiera di Hückel per l'addizione all'anello, classi di BDE per l'astrazione di H), mostrata come «Prev.». Parte con una prova su almeno 20 composti con regioselettività di OH• misurata in letteratura (DOI per ognuno); si pubblica solo se il sito vero sta tra i primi due in almeno il 70 % dei casi e batte la scelta a caso.
- **CC-04**: importazione di calcoli esterni (CSV `siti/1` e uscita `--vfukui` di xtb) con la ricetta per farli fuori dal sito; i valori si mostrano accanto alle classi, con metodo e fonte.

**No**: xtb/DFT/CREST nel browser, RT o CCS previsti per scegliere l'isomero, ML dei siti, frammentatori in silico nel sito.

| Pacchetto | Fase | Dipende da | `.wasm` | Stima (giorni-chat) | Stato |
|---|---|---|---|---|---|
| CC-01 | 1 | — | nessuno | 1,5 | da fare |
| CC-02 | 1 | CC-01 | `grafo` | 1,5 | da fare |
| CC-03 | 2 | CC-02 + sì di Federico | `grafo` | 2 | da decidere |
| CC-04 | 2 | CC-02 + sì di Federico | `grafo` | 1 | da decidere |

## 7. Domande per Federico (con la scelta predefinita che i prompt seguono)
- **D1**: fase 1 solo in Tabui (amministratore)? Predefinito: sì; Attributio la usa quando nasce il Banco 2b (AT-07 legge `posiz/1`).
- **D2**: trasformazioni della fase 1: +O, −CH2, +O−2H. Predefinito: sì; altre (+Cl−H, +2O) dopo.
- **D3**: fase 2 (CC-03, CC-04): si decide dopo aver visto la fase 1 sui dati veri. Predefinito: ferma.
- **D4**: un paragrafo in Teoria su «dove sta l'OH: cosa dice la MS2 e cosa no»? Predefinito: no finché non lo chiedi tu.
- **D5**: in laboratorio, provare lo scambio H/D (D2O nella fase mobile) e una MS3 sul QTRAP per un TP idrossilato? Non è software: è la prova più economica che abbiamo.

## 8. Bibliografia (DOI verificati su Crossref)
- Barrera Y. et al. (2025) Comparative study of predicting radical C–H functionalization sites in nitrogen heteroarenes using a radical general-purpose reactivity indicator and the radical Fukui function. J. Comput. Chem. doi:10.1002/jcc.70130
- Bannwarth C., Ehlert S., Grimme S. (2019) GFN2-xTB. J. Chem. Theory Comput. doi:10.1021/acs.jctc.8b01176
- Bonini P. et al. (2020) Retip: retention time prediction for compound annotation in untargeted metabolomics. Anal. Chem. doi:10.1021/acs.analchem.9b05765
- Cajka T. et al. (2026) Using hydrogen/deuterium exchange in untargeted LC-MS analysis of small molecules. Analytica. doi:10.3390/analytica7030057
- Djoumbou-Feunang Y. et al. (2019) BioTransformer. J. Cheminform. doi:10.1186/s13321-018-0324-5
- Dührkop K. et al. (2019) SIRIUS 4. Nat. Methods. doi:10.1038/s41592-019-0344-8
- Dwinandha D. et al. (2022) Prediction of reaction mechanism for OH radical-mediated phenol oxidation using quantum chemical calculation. Chemosphere. doi:10.1016/j.chemosphere.2021.132763
- Gorges J. et al. (2025) QCxMS2: calculation of electron ionization mass spectra via automated reaction network discovery. Phys. Chem. Chem. Phys. doi:10.1039/D5CP00316D
- Huang et al. (2026) Radical-Net: a chemistry-enhanced transformer for elementary radical reactions in pollutant chemistry. J. Hazard. Mater. doi:10.1016/j.jhazmat.2025.140631
- Kretschmer F. et al. (2024) RepoRT: a comprehensive repository for small molecule retention times. Nat. Methods. doi:10.1038/s41592-023-02143-z
- Kretschmer F. et al. (2025) Times are changing but order matters: transferable prediction of LC retention times (preprint). doi:10.26434/chemrxiv-2024-wd5j8-v3
- Lee M. et al. (2017) A computer-based prediction platform for the reaction of ozone with organic compounds in aqueous solution. Environ. Sci.: Processes Impacts. doi:10.1039/C6EM00584E
- Löffler P. et al. (2025) In silico frontiers shaping the next generation of transformation product prediction and toxicological assessment. Environ. Sci. Technol. 59:19095. doi:10.1021/acs.est.5c06790
- Mao et al. (2024) Holo-Mol: hybrid deep learning for the reactivity of hydroxyl radical to water contaminants. Chem. Eng. J. doi:10.1016/j.cej.2024.148652
- Minakata D. et al. (2009) Development of a group contribution method to predict aqueous phase hydroxyl radical (HO•) reaction rate constants. Environ. Sci. Technol. doi:10.1021/es900956c
- Minakata D. et al. (2023) Predicting transformation products during aqueous oxidation processes: current state and outlook. Environ. Sci. Technol. doi:10.1021/acs.est.3c04086
- Murakami T. et al. (2018) Differentiation of AB-FUBINACA and its five positional isomers using LC-ESI linear ion trap and triple quadrupole MS. Forensic Toxicol. doi:10.1007/s11419-018-0410-4
- Palm E. et al. (2024) Gas phase reactivity of isomeric hydroxylated polychlorinated biphenyls. J. Am. Soc. Mass Spectrom. doi:10.1021/jasms.4c00035
- Pracht P., Bohle F., Grimme S. (2020) Automated exploration of the low-energy chemical space with fast quantum chemical methods (CREST). Phys. Chem. Chem. Phys. doi:10.1039/C9CP06869D
- QCD-MLP (2025) A machine-learning framework for predicting transformation products of organic micropollutants in radical-mediated systems. ACS ES&T Water. doi:10.1021/acsestwater.5c00500
- Ramanathan R. et al. (2000) Liquid chromatography/mass spectrometry methods for distinguishing N-oxides from hydroxylated compounds. Anal. Chem. doi:10.1021/ac9911692
- Ree N., Göller A. H., Jensen J. H. (2021) RegioSQM20. J. Cheminform. doi:10.1186/s13321-021-00490-7
- Ree N., Göller A. H., Jensen J. H. (2022) RegioML. Digital Discovery. doi:10.1039/D1DD00032B
- Ross D. H. et al. (2022) High-throughput measurement and machine learning-based prediction of collision cross sections for drugs and drug metabolites. J. Am. Soc. Mass Spectrom. doi:10.1021/jasms.2c00111
- Ruttkies C. et al. (2016) MetFrag relaunched. J. Cheminform. doi:10.1186/s13321-016-0115-9
- Rydberg P. et al. (2010) SMARTCyp. ACS Med. Chem. Lett. doi:10.1021/ml100016x
- Schymanski E. L. et al. (2014) Identifying small molecules via high resolution mass spectrometry: communicating confidence. Environ. Sci. Technol. doi:10.1021/es5002105
- Shimizu A. et al. (2013) Ion mobility spectrometry–mass spectrometry analysis for the site of aromatic hydroxylation. Drug Metab. Dispos. doi:10.1124/dmd.113.051953
- Šícho M. et al. (2019) FAME 3. J. Chem. Inf. Model. doi:10.1021/acs.jcim.9b00376
- Sojdak C. et al. (2025) Direct (LC-)MS identification of regioisomers from C–H functionalization by partial isotopic labeling. ACS Cent. Sci. doi:10.1021/acscentsci.4c01765
- Tentscher P. R., Lee M., von Gunten U. (2019) Micropollutant oxidation studied by quantum chemical computations. Acc. Chem. Res. doi:10.1021/acs.accounts.8b00610
- Turchi C. S., Ollis D. F. (1990) Photocatalytic degradation of organic water contaminants: mechanisms involving hydroxyl radical attack. J. Catal. doi:10.1016/0021-9517(90)90269-P
- Wang F. et al. (2021) CFM-ID 4.0: more accurate ESI-MS/MS spectral prediction and compound identification. Anal. Chem. doi:10.1021/acs.analchem.1c01465
- Wicker J. et al. (2016) enviPath: the environmental contaminant biotransformation pathway resource. Nucleic Acids Res. doi:10.1093/nar/gkv1229
- Wildman S. A., Crippen G. M. (1999) Prediction of physicochemical parameters by atomic contributions. J. Chem. Inf. Comput. Sci. doi:10.1021/ci990307l
- Zhou Z. et al. (2020) Ion mobility collision cross-section atlas for known and unknown metabolite annotation. Nat. Commun. doi:10.1038/s41467-020-18171-8
