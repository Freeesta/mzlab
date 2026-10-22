# Regole di frammentazione in CID e HCD per il grafo MSn: studio

Studio per il workflow MSn predefinito: l'utente carica l'infusione, dà la molecola (formula + SMILES) e il programma INTERPRETA i frammenti del grafo `msngrafo/1` con regole prese dalla letteratura. Questo file: principi, differenze CID/HCD, fonti, licenze, prove esterne. Schema `fragrules/1` e tabella delle regole: `regole-frammentazione-schema.md`. Mappa del codice, piano delle PR e prompt: `regole-frammentazione-piano.md`. Contratto del grafo: `docs/agenti/msn-grafo.md`.
Le regole danno PROPOSTE con prove e citazione, mai identificazioni (principio 1 di AGENTS.md: nel mondo HR candidati con punteggio, mai verità). Le citazioni hanno chiavi `[XXX00]` risolte nella bibliografia (§8); «da verificare» = riferimento o contenuto non controllato sul testo, da non usare come prova finché qualcuno non lo legge.

## 0. Sintesi delle scelte
1. **Cosa aggiungere**: una tabella di regole `fragrules/1` (dati, non codice) e un valutatore Rust che, dato il SMILES e il grafo con le sottoformule, segna ogni arco con le regole compatibili (perdita neutra + gruppo funzionale presente + polarità + attivazione) e ogni nodo con gli ioni diagnostici; chiave facoltativa `regole` in `msngrafo/1`, come `strutture`.
2. **Le regole sono vincoli e priorità, non generatori**: il grafo misurato resta la verità sperimentale; una regola spiega un arco già osservato o abbassa la priorità di una sottoformula che nessuna regola giustifica. Nessun picco inventato.
3. **CID e HCD separati**: ogni regola dichiara `attivazione` (`cid`, `hcd`, `entrambe`); il valutatore usa l'etichetta per scansione già in `misure[].att` e applica il cut-off di bassa massa solo alla CID in trappola.
4. **Licenze**: nessun parametro, lista o peso copiato da codice GPL/LGPL/AGPL (MetFrag, SIRIUS, CFM-ID); regole riscritte con parole e formule nostre dai fatti pubblicati, con la citazione.
5. **Prova fuori da noi**: spettri pubblici di MassBank Europe con SMILES noto e licenza CC0/CC BY (release fissata, già scaricata da `tools/librerie_pacchetti.py`), copertura dei picchi spiegati contro decoy (SMILES rimescolati e perdite spostate); soglie dichiarate prima della misura.

## 1. Cosa farebbe un professionista di altissimo livello (non chiesto)
1. **Separerebbe le regole dalla «spiegazione combinatoria»**: oggi ogni sottoformula entro 10 ppm è un candidato; aggiungere regole senza un controllo decoy gonfia le «spiegazioni». Misura prima il tasso di spiegazione per caso (decoy) e poi quello delle regole.
2. **Userebbe la parità degli elettroni come filtro morbido, non duro**: la regola dell'elettrone pari ha eccezioni documentate (nitroaromatici, alogenati aromatici, chinoni, metossiaromatici) [LEV07]; un frammento radicale resta, con l'avviso «radical loss» e la regola che lo permette.
3. **Legherebbe le regole al sito di protonazione**: in [M+H]+ la carica sta sul sito più basico (ammina > piridina > ammide > carbonile) e la frammentazione «carica-diretta» parte da lì [DEM16][HOL10]; un protomere diverso (solvente, [TIA08]) cambia i canali. Il motore `protomer.rs` di `mzlab-chem` già stima il sito a pH 7: va riusato come indizio, non come verità in fase gassosa.
4. **Distinguerebbe gli addotti**: [M+Na]+ e [M+NH4]+ non frammentano come [M+H]+ (Na+ resta sul frammento o si perde come ione; NH4+ perde prima NH3 e poi si comporta da [M+H]+) [HOL10]; le regole hanno la condizione `ione`.
5. **Userebbe le curve di breakdown**: le misure per energia di uno stesso arco (già in `misure[]`) danno l'ordine di apertura dei canali; un professionista confronta la CE50 [KER09] fra isomeri e non giudica un arco da una sola energia.
6. **Tratterebbe la MSn come prova gerarchica**: un frammento F visto nella MS3 di P conferma che F viene da P (non da un'altra via); MAGMa mostra che l'uso dei livelli successivi migliora la scelta del candidato [RID12]. Il grafo `msngrafo/1` lo fa già sul piano delle formule; le regole lo estendono ai gruppi.
7. **Correggerebbe il cut-off della trappola**: nella CID in trappola i frammenti sotto circa 0,28 × m/z del precursore non si vedono (§3); un frammento atteso sotto il cut-off NON è un'assenza informativa. Senza questa regola il punteggio penalizza a torto.
8. **Separerebbe frammenti primari e secondari in HCD**: in un collisore a fascio i frammenti si rompono ancora (frammenti di frammenti); una perdita «composta» (H2O + CO) in un solo arco MS2 HCD è normale, in CID di trappola meno [SLE04][MCL97].
9. **Darebbe ogni regola con un controesempio**: per ogni regola un caso in cui il gruppo c'è ma la perdita non appare (es. OH e NH2 aromatici non danno perdite che li rivelano [LEV07]); l'assenza della perdita NON esclude il gruppo [MA14].
10. **Fisserebbe la tolleranza sulla perdita, non solo sul frammento**: la perdita neutra si misura come differenza di due picchi, quindi l'errore si somma; tolleranza in mDa assoluti (≈ 2-3 mDa in Orbitrap sotto m/z 300) più che in ppm della perdita.
11. **Terrebbe una «lista nera» di perdite incompatibili con la formula** (Kind & Fiehn: regole di plausibilità H/C, eteroatomi, RDBE [KIN07]): un frammento con RDBE non intero non può essere uno ione a elettroni pari.
12. **Pubblicherebbe la convalida**: i numeri (copertura, decoy, per classe e per strumento) in un documento ripetibile, con la release di MassBank e lo SHA-256 del file, così chiunque rifà la misura.

## 2. Materiale grezzo da cui partire
- **Review di base (le regole stanno qui)**: Holčapek et al. 2010 [HOL10] (regole di interpretazione API, perdite neutre per gruppo funzionale); Demarque et al. 2016 [DEM16] (reazioni di frammentazione ESI-MS/MS in positivo e negativo, carica-diretta, riarrangiamenti); Levsen et al. 2007 [LEV07] (121 composti modello in trappola ionica: perdite caratteristiche e specifiche per gruppo, eccezioni alla regola dell'elettrone pari); Karni & Mandelbaum 1980 [KAR80] (la regola dell'elettrone pari); Bowie 1990 [BOW90] (ioni negativi a elettroni pari, da verificare).
- **Attivazione**: Sleno & Volmer 2004 [SLE04] (CID a bassa e alta energia, collisioni singole e multiple, energia interna); McLuckey & Goeringer 1997 [MCL97] (riscaldamento lento in trappola); March 1997 [MAR97] (trappola quadrupolare, parametro q, stabilità); Schwartz et al. 2002 [SCH02] (trappola lineare: risonanza, isolamento e attivazione per MSn).
- **Attivazione a fascio ed energia**: Olsen et al. 2007 [OLS07] (HCD, intervallo di massa completo); Diedrich et al. 2013 [DIE13] (energia a gradini, peptidi); Kertesz et al. 2009 [KER09] (CE50 e curve di sopravvivenza per molecole piccole); Hopfgartner et al. 2004 [HOP04] (QqLIT per molecole piccole).
- **Meccanismi**: Wysocki et al. 2000 [WYS00] (protone mobile e localizzato); Cheng & Gross 2000 [CHE00] (frammentazione carica-remota); Tian & Kass 2008 [TIA08] (il protomere dipende dal solvente); Fabre et al. 2001 [FAB01] (retro-Diels-Alder in negativo in trappola); McLafferty 1959 [MCL59] (riarrangiamento, da verificare).
- **Strumenti di calcolo, letti come articoli (non come codice)**: alberi di frammentazione [BOC08][RAS11][DUH19]; MetFrag [WOL10][RUT16]; CFM-ID [ALL15][WAN21] (con regole scritte a mano per alcune classi); MS-FINDER e le nove regole di riarrangiamento dell'idrogeno [TSU16]; MAGMa per alberi MSn [RID12]; ChemFrag (regole + chimica quantistica) [SCU18]; MS2Analyzer (147 perdite neutre dalla letteratura con le sottostrutture) [MA14].
- **Spettri con SMILES noto e licenza aperta**: MassBank Europe [HOR10][NEU26] (record versionati con DOI della release; tipo di strumento, `FRAGMENTATION_MODE` CID/HCD, energia e livello MS nel record; licenza per record); GNPS [WAN16]; MoNA (licenza per record, da verificare); mzCloud solo come riferimento pubblicato, mai scaricato né copiato.
- **Valutazione**: CASMI 2016 [SCY17]; FDR con decoy per la ricerca spettrale [SCB17]; entropia spettrale [LI21]; scala di confidenza [SCY14]; review sulla ricerca MS/MS ad alta risoluzione [KIN18].

## 3. Principi (a)
- **Regola dell'elettrone pari**: uno ione a elettroni pari ([M+H]+, [M−H]−) si rompe di preferenza in uno ione a elettroni pari e una molecola neutra, non in un radicale e uno ione radicale [KAR80][LEV07]. Eccezioni documentate: aromatici con gruppi nitro (perdita di NO• o NO2•), sistemi coniugati che stabilizzano il radicale [LEV07]; altre (CH3• da metossiaromatici, X• da alogenoaromatici, chinoni) sono riportate in [HOL10][DEM16], da verificare sul testo caso per caso. Nel grafo: `elettroni` del candidato è già calcolato; una perdita radicale è ammessa solo se una regola la dichiara.
- **Perdite neutre per gruppo funzionale** (tabella completa in `regole-frammentazione-schema.md`): H2O da alcoli alifatici, acidi carbossilici, idrossili vicini a un sito basico; NH3 da ammine e ammidi primarie; CO dopo una prima perdita da acidi, esteri e ammidi (ione acilio → −CO) e, in negativo, da anelli fenolici condensati [FAB01]; CO2 da acidi carbossilici in negativo [LEV07][HOL10]; HCOOH (CH2O2) da acidi in positivo (caratteristica ma non specifica [LEV07]).
- **Altre perdite**: CH3OH da esteri metilici e metossili vicini alla carica; SO2 da solfonammidi aromatiche (riarrangiamento) e solfoni; SO3 da solfati e acidi solfonici in negativo [LEV07]; HF, HCl, HBr da alogenuri vicini a un H mobile; alcheni (C2H4, C3H6, C4H8) da N-etil, N-isopropil, N-terbutil ammine e ammidi e da esteri alchilici [HOL10][DEM16]. OH e NH2 aromatici non danno perdite che li rivelino [LEV07].
- **Retro-Diels-Alder (RDA)**: un anello cicloesenico (o eterociclo a sei termini con un doppio legame) si apre in un diene e un dienofilo; ioni diagnostici dei due anelli, in positivo e in negativo, con canali propri del negativo [FAB01][DEM16].
- **McLafferty e riarrangiamenti di idrogeno**: un H in γ a un carbonile migra all'ossigeno con rottura del legame α-β e perdita di un alchene [MCL59] (nato in EI; in ESI-CID compare come perdita di alchene da esteri e ammidi [DEM16]). In generale una rottura di legame sposta 0, ±1, ±2 H fra i due pezzi: MS-FINDER ne dà nove regole per eteroatomi e per MSn [TSU16]; `ms2_engine/rearrange.rs` ne fa già la versione semplice (k = ±1, ±2).
- **Carica-diretta contro carica-remota**: con un protone mobile la rottura parte dal sito carico (eterolisi, eliminazioni 1,2, attacchi nucleofili) [WYS00][DEM16]; con una carica fissa (ammonio quaternario, Na+, solfonato) e catene lunghe o anelli fusi la rottura avviene lontano dalla carica, come una termolisi [CHE00]. Regola pratica nel valutatore: con `ione = [M+Na]+` o un azoto quaternario, le perdite «carica-diretta» abbassano la priorità.
- **Siti di protonazione**: in ESI+ il protone va sul sito più basico in fase gassosa (ammine alifatiche > eteroaromatici azotati > ammidi, carbonili, eteri) [HOL10]; la struttura ionica può però riflettere la soluzione e cambiare con il solvente [TIA08]. Il sito decide quali perdite sono vicine alla carica (es. NH3 da ammina primaria protonata, H2O da β-idrossiammina).
- **Stabilità degli ioni prodotto**: ioni aromatici e benzilici (C7H7+, m/z 91,0542, struttura tropilio [RYL57], da verificare per l'ESI), ioni acilio R–C≡O+ (dopo perdita di H2O/CH3OH/NH3 da acidi, esteri, ammidi), ioni immonio CH2=N+R2 (rottura α a un'ammina) e ossonio [HOL10][DEM16]; uno ione molto stabile domina e spegne gli altri canali, quindi un frammento «atteso» assente non è prova contro la struttura.
- **Dipendenza dall'energia**: ogni canale ha una sua soglia; a energia bassa compaiono le perdite a bassa soglia (H2O, NH3, alcheni da riarrangiamento), a energia alta le rotture dirette e gli ioni piccoli; la curva di sopravvivenza del precursore e la CE50 sono riproducibili e distinguono isomeri [KER09]. Nel grafo le `misure[]` per energia sono già la curva di breakdown dell'arco.

## 4. CID in trappola e HCD: differenze (b)
| Aspetto | CID in trappola (risonanza) | HCD / CID a fascio (collisore) | Fonte |
|---|---|---|---|
| Come si attiva | eccitazione di risonanza alla frequenza secolare del precursore, decine di ms, molte collisioni lente con He | accelerazione nel collisore (multipolo) con N2/Ar, decine di eV nel laboratorio, collisioni multiple in µs | [MCL97][SCH02][SLE04][OLS07] |
| Regime | riscaldamento lento: si aprono i canali a soglia più bassa (riarrangiamenti) | energia più alta e distribuita: anche rotture dirette a soglia più alta | [MCL97][SLE04] |
| Frammenti secondari | pochi: il frammento non è in risonanza e si raffredda; servono MS3, MS4 per andare avanti | molti: i frammenti restano caldi e si rompono ancora (perdite composte in un solo spettro) | [MCL97][SLE04] |
| Cut-off di bassa massa | sì: con q di attivazione 0,25 non sono stabili i frammenti sotto ≈ 0,25/0,908 = 0,275 × m/z del precursore (Mathieu, q ∝ 1/m) | no cut-off fisico; resta il primo m/z della scansione scelto dall'utente | [MAR97] (stabilità), [OLS07] |
| Ioni piccoli e diagnostici | spesso persi (sotto il cut-off) | visti (immonio, acilio piccoli, C7H7+) | [OLS07] |
| Energia | «normalizzata» in % sulla m/z del precursore, poco legata agli eV | NCE in % (Orbitrap) o eV (Q-TOF, QqQ); la relazione NCE-m/z è del costruttore: da verificare | [LOP99] da verificare, [DIE13] |
| Energia a gradini | non usata | stepped NCE: tre energie sommate in un solo spettro (più tipi di frammenti) | [DIE13] |
| MSn | naturale (MS3…MS8): il grafo `msngrafo/1` nasce da qui | MS2 (in Orbitrap Tribrid, HCD anche come ultimo stadio dopo CID) | [SCH02][OLS07] |
| QqQ e QTRAP del corso | — | Q2 è CID a fascio (come HCD, a risoluzione unitaria); nel QTRAP l'MS3 si fa in Q3 come trappola lineare (cut-off di nuovo) | [HOP04][HAG02] da verificare |
- **Conseguenze per il valutatore**: (1) cut-off solo per `att = cid` di trappola: soglia `0,275 × m/z del precursore` modificabile (il q d'attivazione può essere letto dal file quando c'è; altrimenti 0,25), e i frammenti attesi sotto soglia sono «non osservabili», non «assenti»; (2) in HCD una perdita composta su un arco (H2O + CO) è spiegata come catena di due regole, con la regola intermedia marcata «non isolata»; (3) uno spettro stepped è una SOMMA: niente curva di breakdown da esso, l'arco riceve l'insieme delle energie.
- **Similitudine fra spettri**: spettri HCD e CID a fascio di un QqQ si somigliano più fra loro che con la CID in trappola [GRA11] (misurato sui peptidi; per le molecole piccole da verificare); una libreria CID di trappola non va cercata con spettri HCD senza dirlo.

## 5. Metodi e strumenti concreti
- **Valutatore a regole** in Rust su `mzlab_feat::chem::grafo::Mol` (già legge SMILES e molfile, `Mol::groups()` dà 31 gruppi funzionali): una regola si applica se i gruppi richiesti ci sono, la formula della perdita è sottoformula della formula del precursore e la differenza osservata cade nella tolleranza.
- **Atomi coinvolti**: per la regola con `sito` (es. «OH alifatico in β a N»), il valutatore cerca la sottostruttura con le funzioni di adiacenza di `Mol` (niente SMARTS completo: `grafo.rs` non ha SMARTS); lo schema accetta un SMARTS solo come testo documentale finché un lettore SMARTS minimo non esiste (PR propria).
- **Collegamento con i tagli**: `chem/strutture.rs` dà i tagli del SMILES che spiegano un nodo; la regola che spiega l'arco entrante nel nodo e il taglio compatibile si incrociano (stessa perdita), così il «legame rotto» proposto ha anche una regola citata.
- **Strumenti esterni solo per la convalida offline** (mai nel sito, mai nel repo): RDKit (BSD) per SMARTS di confronto nei test di parità; MetFrag/SIRIUS/CFM-ID eseguiti fuori, se serve un confronto, con i soli conteggi riportati.

## 6. Cosa dimostra, fuori da noi, che le regole sono giuste
- **Spettri pubblici con SMILES noto**: MassBank Europe, release fissata (`tools/librerie_pacchetti.py` già la scarica e controlla lo SHA-256), solo record CC0 o CC BY, divisi per `FRAGMENTATION_MODE` (CID/HCD) e tipo di strumento (ITFT = trappola + Orbitrap, QFT = quadrupolo + Orbitrap, QTOF).
- **Copertura**: frazione dell'intensità e del numero dei picchi spiegati da una catena di regole, con la tolleranza in ppm sul frammento (10, 20 sotto m/z 150, come `msngrafo`) e in mDa sulla perdita.
- **Decoy**: (1) SMILES rimescolato con la stessa formula (stessi atomi, legami diversi) per misurare quanto le regole spiegano per caso; (2) perdite spostate di una massa fittizia (+0,0137 Da, +1,0078 Da) per misurare la tolleranza; (3) spettri di un composto valutati con il SMILES di un altro della stessa massa nominale. Una regola utile spiega più nel vero che nel decoy; target-decoy come in [SCB17].
- **Isomeri di posizione**: coppie con la stessa formula e gruppi in posizioni diverse; le regole con `sito` devono separarle meglio delle sole perdite.
- **Soglie da fissare prima della misura** (piano §5): copertura media sul vero ≥ decoy + 0,20; nessuna regola con tasso decoy > 30 % delle sue applicazioni resta «priorità alta».

## 7. Licenze (e)
- **Codice da non copiare**: MetFrag LGPL 2.1 o successiva (pagina del progetto); SIRIUS AGPL v3 dalla 4.4.27 (repository); CFM-ID e MS-FINDER: licenza non trovata, da verificare prima di ogni confronto. Nessuna loro tabella di perdite, peso, energia di legame o parametro entra nel repo.
- **Fatti pubblicati**: le perdite neutre e i meccanismi descritti negli articoli sono fatti chimici; li scriviamo con parole e formule nostre, ricalcoliamo ogni massa con `mzlab-core::elements` e citiamo DOI/PMID.
- **Dati**: MassBank per record (CC0, CC BY, CC BY-SA, CC BY-NC…: il campo `LICENSE` del record decide); nel repo pubblico solo accessioni e conteggi, mai spettri NC o SA; gli spettri CC BY già in `tests/data/pubblici_gemini/` restano con la loro `FONTE.md`. GNPS e MoNA: licenza per libreria/record, da verificare. mzCloud: mai.

## 8. Bibliografia (PMID e DOI verificati su PubMed salvo «da verificare»)
- [HOL10] Holčapek M, Jirásko R, Lísa M. J Chromatogr A 2010;1217:3908-21. PMID 20303090, doi:10.1016/j.chroma.2010.02.049
- [DEM16] Demarque DP et al. Nat Prod Rep 2016;33:432-55. PMID 26673733, doi:10.1039/c5np00073d
- [LEV07] Levsen K et al. J Mass Spectrom 2007;42:1024-44. PMID 17605143, doi:10.1002/jms.1234
- [KAR80] Karni M, Mandelbaum A. Org Mass Spectrom 1980;15:53-64. doi:10.1002/oms.1210150202 (DOI da fonti secondarie: da verificare)
- [BOW90] Bowie JH. Mass Spectrom Rev 1990;9:349-79 (da verificare)
- [MCL59] McLafferty FW. Anal Chem 1959;31:82-87 (da verificare)
- [RYL57] Rylander PN, Meyerson S, Grubb HM. J Am Chem Soc 1957;79:842 (da verificare)
- [KIN07] Kind T, Fiehn O. BMC Bioinformatics 2007;8:105. PMID 17389044, doi:10.1186/1471-2105-8-105
- [SLE04] Sleno L, Volmer DA. J Mass Spectrom 2004;39:1091-112. PMID 15481084, doi:10.1002/jms.703
- [MCL97] McLuckey SA, Goeringer DE. J Mass Spectrom 1997;32:461-74 (volume e pagine confermati, DOI da verificare)
- [MAR97] March RE. J Mass Spectrom 1997;32:351-69 (da verificare)
- [LOP99] Lopez LL et al. Rapid Commun Mass Spectrom 1999;13:663-8 (energia normalizzata in trappola: da verificare)
- [SCH02] Schwartz JC, Senko MW, Syka JEP. J Am Soc Mass Spectrom 2002;13:659-69. PMID 12056566, doi:10.1016/S1044-0305(02)00384-7
- [OLS07] Olsen JV et al. Nat Methods 2007;4:709-12. PMID 17721543, doi:10.1038/nmeth1060
- [DIE13] Diedrich JK, Pinto AFM, Yates JR. J Am Soc Mass Spectrom 2013;24:1690-9. PMID 23963813, doi:10.1007/s13361-013-0709-7
- [KER09] Kertesz TM et al. J Am Soc Mass Spectrom 2009;20:1759-67. PMID 19616966, doi:10.1016/j.jasms.2009.06.002
- [HOP04] Hopfgartner G et al. J Mass Spectrom 2004;39:845-55. PMID 15329837
- [HAG02] Hager JW. Rapid Commun Mass Spectrom 2002;16:512-26 (da verificare)
- [WYS00] Wysocki VH et al. J Mass Spectrom 2000;35:1399-406. PMID 11180630
- [CHE00] Cheng C, Gross ML. Mass Spectrom Rev 2000;19:398-420. PMID 11199379
- [TIA08] Tian Z, Kass SR. J Am Chem Soc 2008;130:10842-3. PMID 18661998, doi:10.1021/ja802088u
- [FAB01] Fabre N et al. J Am Soc Mass Spectrom 2001;12:707-15. PMID 11401161, doi:10.1016/S1044-0305(01)00226-4
- [BOC08] Böcker S, Rasche F. Bioinformatics 2008;24:i49-55. PMID 18689839, doi:10.1093/bioinformatics/btn270
- [RAS11] Rasche F et al. Anal Chem 2011;83:1243-51. PMID 21182243, doi:10.1021/ac101825k
- [DUH19] Dührkop K et al. Nat Methods 2019;16:299-302. PMID 30886413, doi:10.1038/s41592-019-0344-8
- [WOL10] Wolf S et al. BMC Bioinformatics 2010;11:148. PMID 20307295, doi:10.1186/1471-2105-11-148
- [RUT16] Ruttkies C et al. J Cheminform 2016;8:3. PMID 26834843, doi:10.1186/s13321-016-0115-9
- [ALL15] Allen F, Greiner R, Wishart D. Metabolomics 2015;11:98-110. doi:10.1007/s11306-014-0676-4
- [WAN21] Wang F et al. Anal Chem 2021;93:11692-700. PMID 34403256, doi:10.1021/acs.analchem.1c01465
- [TSU16] Tsugawa H et al. Anal Chem 2016;88:7946-58. PMID 27419259, doi:10.1021/acs.analchem.6b00770
- [RID12] Ridder L et al. Rapid Commun Mass Spectrom 2012;26:2461-71. PMID 22976213, doi:10.1002/rcm.6364
- [SCU18] Schüler JA et al. J Mass Spectrom 2018;53:1104-15. PMID 30103269, doi:10.1002/jms.4278
- [MA14] Ma Y et al. Anal Chem 2014;86:10724-31. PMID 25263576, doi:10.1021/ac502818e
- [SCB17] Scheubert K et al. Nat Commun 2017;8:1494. PMID 29133785, doi:10.1038/s41467-017-01318-5
- [LI21] Li Y et al. Nat Methods 2021;18:1524-31. PMID 34857935, doi:10.1038/s41592-021-01331-z
- [SCY17] Schymanski EL et al. J Cheminform 2017;9:22. PMID 29086042, doi:10.1186/s13321-017-0207-1
- [SCY14] Schymanski EL et al. Environ Sci Technol 2014;48:2097-8. PMID 24476540, doi:10.1021/es5002105
- [HOR10] Horai H et al. J Mass Spectrom 2010;45:703-14. PMID 20623627, doi:10.1002/jms.1777
- [NEU26] Neumann S et al. Nucleic Acids Res 2026;54:D601-6. PMID 41217946, doi:10.1093/nar/gkaf1193
- [WAN16] Wang M et al. Nat Biotechnol 2016;34:828-37. PMID 27504778, doi:10.1038/nbt.3597
- [GRA11] de Graaf EL et al. J Proteome Res 2011;10:4334-41. PMID 21726076, doi:10.1021/pr200156b
- [KIN18] Kind T et al. Mass Spectrom Rev 2018;37:513-32. PMID 28436590, doi:10.1002/mas.21535
