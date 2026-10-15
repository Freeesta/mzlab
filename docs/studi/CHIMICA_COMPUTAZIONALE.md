# Chimica computazionale e posizione della sostituzione negli isomeri dei TP

Studio dell'Architetto sulla domanda di Federico: ha senso mettere in mzLab un calcolo di chimica computazionale (siti di attacco di OH•, altre reazioni) e altre prove scientifiche per capire DOVE sta la sostituzione in un isomero posizionale (per esempio i TP idrossilati)? Su richiesta di Federico la valutazione copre tutte le reazioni di trasformazione degli inquinanti (§2b), non solo OH•. Prompt dei pacchetti in `docs/studi/PROMPT_CHIMICA_COMPUTAZIONALE/`.

## 0. Riassunto (10 righe)
1. Risposta: **sì, ma ridotto**. Il valore vero non sta nel calcolo quantistico ma nel confronto ordinato tra le posizioni possibili e lo spettro MS2 misurato.
2. Gli indici di Fukui e la DFT dicono «dove è più probabile» l'attacco di OH• su molecole semplici; sui TP reali sono qualitativi, cambiano con il metodo e non sono mai una prova di struttura. Vale lo stesso per ozono, cloro, fotolisi, idrolisi, riduzione, SO4•⁻/Cl• e biotrasformazione (§2b): la previsione della posizione è validata bene solo per l'alogenazione aromatica (90-96 %) e, meno, per il metabolismo P450 (top-2 circa 76-82 %).
3. Su TiO2 l'attacco non è solo OH• libero in soluzione (lacune, superficie, adsorbimento): una previsione fatta in acqua omogenea vale ancora meno.
4. Nessun calcolo pesante (xtb, CREST, DFT) può girare nel browser in modo sensato: Fortran, minuti o ore per molecola, megabyte di `.wasm`. Si può solo importare un calcolo fatto fuori.
5. La prova più solida per localizzare un -OH resta la MS2: se un frammento porta +16 Da, l'OH sta in quel pezzo. Questo si può calcolare in Rust con il grafo SMILES che mzLab ha già.
6. Di solito la MS2 dice «in quale anello o catena», quasi mai «in orto o in para»: il programma deve mostrare le classi di posizioni che lo spettro non sa distinguere (livello 3a), non sceglierne una.
7. RT previsto e CCS previsto hanno errori più grandi della differenza tra isomeri posizionali: servono a scartare, non a decidere. Il 3200 QTRAP non ha mobilità ionica.
8. Prove sperimentali economiche descritte in letteratura: scambio H/D (D2O) per distinguere C-OH da N-ossido, MS3 e rapporti tra ioni a più energie di collisione, standard di riferimento; Federico ha deciso di non provarle in laboratorio (D5).
9. Proposta: fase 1 (circa 3 giorni-chat) «posizioni compatibili con lo spettro» in Rust, solo in Tabui (amministratore) e pronto per il Banco 2b; fase 2 facoltativa: una stima di reattività semplice, validata prima su dati veri, e l'importazione di calcoli esterni (xtb).
10. Niente sale da solo a 2a o 2b: il programma propone classi 3a/3b con le prove; il 2b lo assegna solo il chimico. Nel mondo LR degli studenti non compare nulla, salvo un paragrafo di Teoria (CC-05) voluto da Federico.

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

## 2b. Tutte le reazioni di trasformazione: cosa si prevede e cosa serve per la posizione
Per ogni reazione: cosa sanno prevedere gli strumenti, quanto è stato verificato contro l'esperimento, e se aiuta a dire DOVE sta la modifica in un isomero. «Posizione» = scegliere tra siti equivalenti per formula (orto/meta/para, quale metile, quale N).

| Reazione | Strumenti e metodi | Validazione sperimentale e accuratezza | Utile per la posizione? | Nel programma |
|---|---|---|---|---|
| OH• (AOP, TiO2, fotolisi indiretta) | Fukui, DFT con stati di transizione, contributi di gruppo (Minakata 2009), QCD-MLP, Radical-Net | k entro un fattore ~2 (gruppi); sito: fenolo 77/23 orto/para come l'esperimento (DFT); ML AUC 0,93 su 345 reazioni; nessun banco ampio sul sito (§2) | debole: ordina i siti dentro un anello, senza misura affidabile | fase 1 (classi dalla MS2); stima «Prev.» solo in fase 2 se passa la validazione |
| O3 (ozonazione) | piattaforma a regole di Lee et al. 2017 (~340 regole + k da E_HOMO); QSAR di Lee e von Gunten 2012 | k: errore medio circa un fattore 6 su 284 composti; prodotti coerenti su 3 composti di prova (carbamazepina, tramadolo, triclosan); il 74 % dei k QSAR entro un fattore 3 | sì per il **gruppo** attaccato (olefina, ammina, fenolo attivato), poco per la posizione nell'anello | no: le regole per gruppo stanno già in `regole_tp/2`; rifare 340 regole non vale il costo |
| Cloro libero (HOCl, disinfezione), Br | sostituzione elettrofila aromatica: RegioSQM (affinità protonica con PM3/xtb), RegioML; DFT su casi (carbamazepina + HOCl, Tandarić 2016: barriere 96-106 kJ/mol contro 98 sperimentale) | alogenazione aromatica: sito giusto nel 96 % di >525 esempi (RegioSQM 2018), 90-93 % (RegioML 2022); N-clorurazione delle ammine: regole di gruppo | **sì**: è il caso meglio validato; orto/para rispetto a OH e NH sono previsti bene | fase 1: `+Cl-H` e `+Br-H` tra le trasformazioni delle classi; fase 2: stima di Wheland (Hückel) validata sull'alogenazione |
| SO4•⁻ (persolfato) | ML dei k (Tang et al. 2024); trasferimento elettronico come primo passo | k sì (modelli ad albero); sito: nessuna previsione validata trovata | no | no |
| Cl•, Cl2•⁻ (UV/cloro) | QCD-MLP (AUC 0,89 su 150 reazioni), DFT su sonde | pochi casi | debole | no |
| Fotolisi diretta | libreria di regole CTS (Yuan et al. 2020, 2021); TD-DFT per l'assorbimento | regole valutate su dati interni ed esterni; resa quantica non prevedibile in modo affidabile | no: la posizione la decide il cromoforo, raramente ambigua | no |
| Fotolisi indiretta (¹O2, ³CDOM*, OH•) | QSAR dei k; per OH• vedi sopra | k sì per classi; sito come OH• o ¹O2 (gruppi ricchi di elettroni) | debole | solo tramite la riga OH• |
| Idrolisi | libreria CTS (24 schemi, Tebes-Stevens et al. 2017); HYDROWIN | prodotti e velocità per classe ben documentati | no: il sito è il gruppo idrolizzabile, non c'è ambiguità posizionale | già in `grafo.rs` (rotture di idrolisi) |
| Riduzione abiotica (nitro → ammina, dealogenazione) | libreria CTS in prova, senza validazione pubblicata | scarsa | sì solo per quale alogeno si perde: si vede dalla formula e dal frammento | coperta dalla fase 1 come «quale atomo cambia» se la trasformazione è nelle regole |
| Biotrasformazione microbica (fanghi, suolo) | EAWAG-BBD/PPS, enviPath (regole + ML «relative reasoning»), BioTransformer | percorsi: buoni per le liste di sospetti, precisione bassa (Trostel et al. 2023; Hafner et al. 2024; Tam et al. 2021) | debole: le regole dicono il gruppo (dealchilazione, idrossilazione), non la posizione | no nel sito: i loro prodotti entrano come lista di sospetti |
| Metabolismo di fase I (P450) | SMARTCyp (2D), FAME 3, XenoSite, GLORYx, SyGMa, BioTransformer 3.0, MetaTrans | sito: top-2 76 % su 394 substrati CYP3A4 (SMARTCyp), circa 82 % in media su 9 isoforme; FAME 3 top-2 > 80 %; prodotti: richiamo 60-77 %, precisione 6-14 % (GLORYx, SyGMa) | **sì in parte**: il miglior caso dopo l'alogenazione, ma vale per enzimi, non per TiO2 | fase 2 facoltativa, solo per campagne di metabolismo: ordine dei siti come «Prev.» |
| Fase II e coniugazioni (glucuronide, solfato, GSH, acetile) | GLORYx, BioTransformer, SyGMa | richiamo come sopra; sito: quale OH/NH è coniugato poco validato | debole: in MS2 il coniugato si perde intero (−176, −80), quindi i frammenti non localizzano | no; prova sperimentale: idrolisi enzimatica e confronto |

Sintesi di §2b: la **posizione** dentro un anello si prevede bene solo per l'alogenazione elettrofila e, meno, per il P450; per le reazioni dei nostri esperimenti (OH• su TiO2, ozono, fotolisi) gli strumenti dicono il **gruppo** reattivo, e il gruppo mzLab lo sa già da `regole_tp/2`. La prova che separa davvero gli isomeri resta sperimentale (MS2, MS3, H/D, standard).

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
| Opzione | Reazione | Dove gira | Peso | Tempo per molecola | Licenza | Giudizio |
|---|---|---|---|---|---|---|
| A. Posizioni compatibili con la MS2 (enumerazione + spostamento dei frammenti) | tutte (+O, −CH2, +O−2H, +Cl−H, +Br−H) | Rust, `.wasm` `grafo` | stima < 40 KB in più | millisecondi | MIT (nostro) | **fase 1** |
| B. Tipo di ossidazione dalle perdite neutre e suggerimento H/D | ossidazioni (OH•, O3, P450) | Rust, stesso modulo | pochi KB | millisecondi | MIT | **fase 1** |
| C. Reattività qualitativa (Hückel π + classi di BDE C-H per l'astrazione) | OH•, alogenazione; P450 se richiesto | Rust, `.wasm` `grafo` | stima < 60 KB | millisecondi | MIT | fase 2, solo se passa la validazione |
| D. Importazione di un calcolo esterno (CSV `siti/1`, uscita `--vfukui` di xtb) | tutte | Rust (lettore), calcolo fuori | pochi KB | lo decide chi calcola | xtb LGPL-3.0 usato come programma separato: nessun vincolo per noi | fase 2 |
| E. Contributi di gruppo k(OH•) (Minakata 2009) | OH• | Rust | piccolo | millisecondi | dati pubblicati | no per ora: dà la velocità, non la posizione |
| F. xtb/tblite/CREST nel browser | tutte | WASM da Fortran + LAPACK | diversi MB, nessuna compilazione WASM nota | minuti-ore (CREST) | LGPL-3.0 (link statico nel `.wasm` = obblighi di rilinkaggio) | **no** |
| G. DFT (ORCA, Psi4) | tutte | fuori | — | ore-giorni | ORCA proprietario, Psi4 LGPL-3.0 | solo come D |
| H. CFM-ID, MetFrag, SIRIUS nel sito | tutte | fuori (Java/C++) | decine di MB | secondi-minuti | SIRIUS AGPL-3.0; CFM-ID e MetFrag da verificare | no nel sito; i risultati entrano già da Attributio (AT-06) |
| I. RDKit nel browser (MinimalLib) | tutte | WASM | qualche MB | — | BSD-3 | no: abbiamo già il grafo in Rust |
| J. RT o CCS previsti per ogni isomero | tutte | — | — | — | — | no: errore maggiore della differenza |
| K. ML per i siti (QCD-MLP e simili) | OH•, Cl• | Python + descrittori DFT | — | — | da verificare | no: chiede DFT per ogni molecola e il codice non è nel nostro linguaggio |
| L. Regole di ozono di Lee et al. 2017 riscritte | O3 | Rust | grande (~340 regole) | — | da verificare (applicazione a sé) | no: dà il gruppo, già in `regole_tp/2` |
| M. Libreria CTS (idrolisi, fotolisi, riduzione) | idrolisi, fotolisi diretta, riduzione | servizio web EPA | — | — | servizio pubblico | no nel sito: i prodotti entrano come sospetti; nessuna ambiguità di posizione |
| N. enviPath, EAWAG-PPS, BioTransformer, GLORYx, SyGMa | biotrasformazione, fase I/II | fuori (servizi web, Java/Python) | — | secondi | da verificare per ognuno | no nel sito: liste di sospetti; la posizione non la danno in modo affidabile |
| O. Stima di sito P450 alla SMARTCyp (metodo pubblicato riscritto in Rust) | P450 | Rust, `grafo` | stima < 80 KB | millisecondi | MIT (metodo dall'articolo, non il codice) | fase 2 facoltativa, solo con il sì di Federico e la validazione |
| P. Contributi di gruppo per SO4•⁻, Cl•, fotolisi indiretta | SO4•⁻, Cl•, ¹O2 | — | — | — | — | no: danno k, non la posizione |

Licenze verificate sui file del repository a monte: xtb, CREST, tblite, Psi4 LGPL-3.0; SIRIUS AGPL-3.0; RDKit BSD-3. Le opzioni scelte (A-D) non includono codice di terzi: leggiamo numeri prodotti da altri programmi o calcoliamo con codice nostro.

## 5. Rischi scientifici e regole
- **Falsa precisione**: una barra colorata «sito più reattivo» su una struttura sembra una risposta. Regola: la stima di reattività (C, D) si mostra solo come «Prev.» e solo accanto alle classi compatibili con lo spettro; non riordina mai le classi e non toglie candidati.
- **Livelli**: la fase 1 produce classi di posizioni (3a se le classi sono poche e lo spettro ne esclude alcune, 3b se lo spettro non esclude nulla) come proposta; il livello deciso resta vuoto finché il chimico non decide. Nulla di computazionale sale a 2a o 2b.
- **Mondo LR degli studenti**: niente di questo nella vista Dati, nel Disegno degli studenti né nelle tabelle della relazione (principio 1). Solo Tabui e Attributio, dietro `uipAdminOn()`.
- **Limite del grafo**: le rotture sono solo di legami singoli fuori dagli anelli; un'apertura d'anello o un riarrangiamento non sono modellati. Un frammento osservato che nessuna rottura spiega non è una prova né contro né a favore: si scrive «non spiegato».
- **Risoluzione unitaria**: +16 Da si vede, ma un frammento a 1 Da da un altro può confondersi; la tolleranza e il numero di frammenti spiegati si mostrano sempre.
- **Simmetria**: posizioni equivalenti per simmetria (per esempio 2 e 6 in un anello para-sostituito) sono un solo candidato; un errore qui gonfierebbe il numero di isomeri.
- **Nomi**: nessun cognome di `nomi_vietati.txt` nei test e nei documenti (`tests/test_esempi_anonimi.py`); i composti di prova vengono dalla letteratura citata.
- **Teoria**: Teoria e Pratica si toccano solo con un'istruzione esplicita di Federico; per il paragrafo sulla localizzazione dell'OH l'istruzione c'è (D4, pacchetto CC-05) e vale solo per quel paragrafo e le sue domande.

## 6. Raccomandazione: SÌ ridotto, a gradini
**Fase 1 (alto valore, basso costo, circa 3 giorni-chat)**
- **CC-01** (Rust, nessuna interfaccia): dato il genitore (SMILES) e la trasformazione (+O, −CH2, +O−2H, +Cl−H, +Br−H), enumera le posizioni possibili (C-H aromatico, C-H alifatico, N-ossido, S-ossido; quale metile perso), le raccoglie per simmetria, calcola per ognuna i frammenti attesi con lo spostamento e le confronta con la MS2 osservata. Uscita `posiz/1`: classi di posizioni che lo spettro non distingue, con i frammenti che le sostengono o le escludono, il tipo di ossidazione dalle perdite neutre e gli esperimenti suggeriti (H/D, MS3).
- **CC-02** (`.wasm` `grafo` + Tabui, amministratore): pannello «Substitution position» nella riga di un TP con MS2, in LR (0,5 Da) e HR (ppm), con tabella dei dati; il contratto `posiz/1` resta pronto per il Banco 2b di AT-07.

**Fase 2 (facoltativa, parte solo con il sì di Federico dopo aver visto la fase 1)**
- **CC-03**: stima di reattività qualitativa in Rust, per reazione: OH• (densità di frontiera di Hückel, classi di BDE per l'astrazione di H) e alogenazione elettrofila (energia di localizzazione di Wheland, il caso meglio validato in letteratura), mostrata come «Prev.»; il P450 alla SMARTCyp resta un'opzione separata (§4 riga O) solo se Federico la chiede. Parte con una prova su almeno 20 composti con regioselettività di OH• misurata in letteratura (DOI per ognuno); si pubblica solo se il sito vero sta tra i primi due in almeno il 70 % dei casi e batte la scelta a caso.
- **CC-04**: importazione di calcoli esterni (CSV `siti/1` e uscita `--vfukui` di xtb) con la ricetta per farli fuori dal sito; i valori si mostrano accanto alle classi, con metodo e fonte.

**Teoria (D4 = sì)**
- **CC-05**: nel capitolo 14 (frammentazione in CID) un paragrafo «Dove sta l'OH: cosa dice l'MS2 e cosa no», con una o due domande nella Verifica; indipendente dagli altri pacchetti.

**No**: xtb/DFT/CREST nel browser, RT o CCS previsti per scegliere l'isomero, ML dei siti, frammentatori in silico nel sito.

| Pacchetto | Fase | Dipende da | `.wasm` | Stima (giorni-chat) | Stato |
|---|---|---|---|---|---|
| CC-01 | 1 | — | nessuno | 1,5 | fatto: 781 righe di codice (`chem/posizioni/`), 10 prove |
| CC-02 | 1 | CC-01 | `grafo` | 1,5 | fatto: `positions` nel `.wasm` `grafo` (158 151 → 234 654 B dopo `wasm-opt -Oz`, tetto 512 000 B invariato) e nella rotta `feat/grafo`; pannello `44_posizioni.js`, `e2e_posizioni.py` |
| CC-03 | 2 | CC-02 + sì di Federico | `grafo` | 2 | da decidere |
| CC-04 | 2 | CC-02 + sì di Federico | `grafo` | 1 | da decidere |
| CC-05 | Teoria | — | nessuno | 0,5-1 | da fare |

## 7. Decisioni di Federico (domande D1-D6)
- **D1** (fase 1 solo in Tabui, amministratore): **sì**. Attributio la usa quando nasce il Banco 2b (AT-07 legge `posiz/1`).
- **D2** (trasformazioni della fase 1: +O, −CH2, +O−2H, +Cl−H, +Br−H): **sì**; altre (+2O, coniugazioni) dopo.
- **D3** (fase 2, CC-03 e CC-04): **si decide dopo aver visto la fase 1 sui dati veri**; fino ad allora ferma.
- **D4** (paragrafo in Teoria «dove sta l'OH: cosa dice la MS2 e cosa no»): **sì**, istruzione esplicita di Federico; pacchetto CC-05.
- **D5** (prove di laboratorio: scambio H/D in D2O e MS3 sul QTRAP): **no**, nessuna prova in laboratorio. I codici `hdx` e `ms3` di `posiz/1` restano come controlli suggeriti dalla letteratura, non come esperimenti in programma.
- **D6** (stima dei siti del P450 per altre campagne): **no**; la riga O di §4 resta chiusa.

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
- de Bruyn Kops C. et al. (2020) GLORYx: prediction of the metabolites resulting from phase 1 and phase 2 biotransformations of xenobiotics. Chem. Res. Toxicol. doi:10.1021/acs.chemrestox.0c00224
- Gao J., Ellis L. B. M., Wackett L. P. (2010) The University of Minnesota Biocatalysis/Biodegradation Database: improving public access. Nucleic Acids Res. doi:10.1093/nar/gkp771
- Hafner J. et al. (2024) Advancements in biotransformation pathway prediction: enhancements, datasets, and novel functionalities in enviPath. J. Cheminform. doi:10.1186/s13321-024-00881-6
- Kromann J. C. et al. (2018) Fast and accurate prediction of the regioselectivity of electrophilic aromatic substitution reactions (RegioSQM). Chem. Sci. doi:10.1039/C7SC04156J
- Lee Y., von Gunten U. (2012) QSARs for the transformation of organic micropollutants during oxidative water treatment. Water Res. doi:10.1016/j.watres.2012.06.006
- Lee Y., von Gunten U. (2016) Advances in predicting organic contaminant abatement during ozonation of municipal wastewater effluent. Environ. Sci.: Water Res. Technol. doi:10.1039/C6EW00025H
- Litsa E. E. et al. (2020) Prediction of drug metabolites using neural machine translation (MetaTrans). Chem. Sci. doi:10.1039/D0SC02639E
- Ridder L., Wagener M. (2008) SyGMa: combining expert knowledge and empirical scoring in the prediction of metabolites. ChemMedChem. doi:10.1002/cmdc.200700312
- Tam J. et al. (2021) Holistic evaluation of biodegradation pathway prediction: assessing multi-step reactions and intermediate products. J. Cheminform. doi:10.1186/s13321-021-00543-x
- Tandarić T., Vrček V., Šakić D. (2016) A quantum chemical study of HOCl-induced transformations of carbamazepine. Org. Biomol. Chem. doi:10.1039/C6OB02166B
- Tang et al. (2024) Machine learning models with molecular fingerprints and chemical structures to predict sulfate radical rate constants. Processes. doi:10.3390/pr12020384
- Tebes-Stevens C. et al. (2017) Prediction of hydrolysis products of organic chemicals under environmental pH conditions. Environ. Sci. Technol. doi:10.1021/acs.est.6b05412
- Trostel L. et al. (2023) Combining predictive and analytical methods to elucidate pharmaceutical biotransformation in activated sludge. Environ. Sci.: Processes Impacts. doi:10.1039/D3EM00161J
- Wishart D. S. et al. (2022) BioTransformer 3.0: a web server for accurately predicting metabolic transformation products. Nucleic Acids Res. doi:10.1093/nar/gkac313
- Yuan C., Tebes-Stevens C., Weber E. J. (2020) Reaction library to predict direct photochemical transformation products of environmental organic contaminants. Environ. Sci. Technol. doi:10.1021/acs.est.0c00484
- Yuan C., Tebes-Stevens C., Weber E. J. (2021) Prioritizing direct photolysis products predicted by the Chemical Transformation Simulator. Environ. Sci. Technol. doi:10.1021/acs.est.0c08745
- Zhu et al. (2024) MetaPredictor: in silico prediction of drug metabolites based on deep language models. Brief. Bioinform. doi:10.1093/bib/bbae374
