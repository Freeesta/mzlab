# Rapporto T8 — Letteratura Scientifica su Siti di Reazione e Coppie di Isomeri

- **Traccia**: T8 (Letteratura siti di reazione e coppie di isomeri con tempo di ritenzione RP-LC)
- **Data compilazione**: 16 Ottobre 2026
- **Autore**: Sotto-agente A7 (mzLab / Gemini)
- **File generati**:
  - `docs/letteratura_cc/siti_sperimentali.csv` (39 siti e intermedi su 10 molecole)
  - `docs/letteratura_cc/coppie_isomeri_rt.csv` (12 coppie reali di isomeri posizionali con RT misurati in RP-LC)
  - `docs/letteratura_cc/RAPPORTO.md` (questo documento di sintesi e validazione)

---

## 1. Obiettivi e Metodologia

Il presente rapporto documenta l'evidenza sperimentale peer-reviewed relativa a:
1. **Siti primari di attacco da radicale ossidrile ($ullet	ext{OH}$)** o idrossilazione in processi avanzati di ossidazione (AOPs: fotocatalisi $	ext{TiO}_2$, $	ext{UV/H}_2	ext{O}_2$, Fenton, foto-Fenton, radiolisi) per le **10 molecole nuove di §6**:
   - *Diclofenac*
   - *Sulfametossazolo*
   - *Atrazina*
   - *Bisfenolo A*
   - *Caffeina*
   - *Triclosan*
   - *Ibuprofene*
   - *Metoprololo*
   - *Benzofenone-3*
   - *Imidacloprid*
2. **Coppie reali di isomeri di prodotti di trasformazione (TP)** posizionali separati cromatograficamente in fase inversa (RP-LC: colonne C18 ad alta prestazione) con tempi di ritenzione ($t_R$) quantitativi esatti riportati nei testi o nei dati supplementari aperti.
3. **Verifica rigorosa dei Digital Object Identifier (DOI)**: ciascun DOI citato è stato verificato in tempo reale mediante interrogazione HTTP diretta alle API Crossref (`https://api.crossref.org/works/<doi>`), verificando titolo esatto, anno di pubblicazione e testata scientifica, escludendo qualunque dato ipotetico o non comprovato.

---

## 2. Sintesi delle Evidenze Sperimentali per le 10 Molecole (§6)

### 2.1 Diclofenac
- **Sito C5 (anello fenilacetico, *para* al gruppo amminico secondario)**: principale sito di addizione elettrofila da $ullet	ext{OH}$, che porta alla formazione di **5-idrossidiclofenac** ($m/z\ 312\ [	ext{M+H}]^+$, $	ext{C}_{14}	ext{H}_{11}	ext{Cl}_2	ext{NO}_3$). Confermato con standard di riferimento puro e cromatografia LC-MS/MS (*Pérez-Estrada et al., 2005*, doi:10.1021/es050794n; *Calza et al., 2006*, doi:10.1016/j.apcatb.2006.04.021). Confidenza: **alta**.
- **Sito C4' (anello 2,6-diclorofenilico, *para* a NH)**: attacco competitivo che genera **4'-idrossidiclofenac**, anch'esso identificato mediante standard autentico. Confidenza: **alta**.
- **Sito C3' (anello diclorurato, *meta* a NH)**: idrossilazione secondaria rilevata via HRMS/MS2 (**3'-idrossidiclofenac**). Confidenza: **media**.
- **Ponte difenilamminico**: deidrogenazione/ossidazione del ponte amminico con ciclizzazione intramolecolare a derivati carbazolici o formazione di chinon-immine reattive. Confidenza: **media**.

### 2.2 Sulfametossazolo (SMX)
- **Anello anilinico (posizioni C3 e C5, *orto* a $-	ext{NH}_2$)**: addizione elettrofila radicalica favorita dall'elevata densità elettronica conferita dal gruppo amminico primario, con produzione di **3-idrossisulfametossazolo** ($m/z\ 270\ [	ext{M+H}]^+$, $	ext{C}_{10}	ext{H}_{11}	ext{N}_3	ext{O}_4	ext{S}$) (*Trovó et al., 2009*, doi:10.1016/j.watres.2009.04.006). Confidenza: **media**.
- **Azoto amminico primario ($-	ext{NH}_2$)**: ossidazione progressiva via radicale $ullet	ext{OH}$ a idrossilammina ($-	ext{NHOH}$) e successivamente a derivato nitroso e nitroico (**4-nitro-sulfametossazolo**, $m/z\ 284$). Confidenza: **alta** (standard autentico).
- **Legame solfonammidico $	ext{S–N}$**: scissione omolitica/idrolitica indotta da attacco radicalico con rilascio di **acido solfanilico** e **3-ammino-5-metilisossazolo**. Confidenza: **alta**.
- **Anello isossazolico (C4)**: attacco al doppio legame con successiva apertura dell'anello eterociclico. Confidenza: **media**.

### 2.3 Atrazina
- **Catena alchilica $N$-etile**: astrazione di idrogeno sul carbonio $lpha$ alifatico ($-	ext{NH}–	ext{CH}_2	ext{CH}_3$) seguita da deochilazione ossidativa con eliminazione di acetaldeide; genera **deetilatrazina** (DEA, $	ext{C}_6	ext{H}_{10}	ext{ClN}_5$) (*Konstantinou et al., 2001*, doi:10.1021/es001271c). Confidenza: **alta**.
- **Catena alchilica $N$-isopropile**: analoga deochilazione ossidativa con rilascio di acetone, formando **deisopropilatrazina** (DIA, $	ext{C}_5	ext{H}_8	ext{ClN}_5$). Confidenza: **alta**.
- **Sito $	ext{C–Cl}$ sul nucleo $s$-triazinico**: declorurazione con idrossilazione nucleofila/omolitica a **idrossiatrazina** (HA, $	ext{C}_8	ext{H}_{15}	ext{N}_5	ext{O}$). Confidenza: **alta**.
- **Intermedi alifatici idrossilati stabili**: rilevamento di **1-idrossietilatrazina** (CNIT, $	ext{C}_8	ext{H}_{14}	ext{ClN}_5	ext{O}$) mediante UHPLC-HRMS Orbitrap (*Molecules 2022*, doi:10.3390/molecules27249021). Confidenza: **media**.

### 2.4 Bisfenolo A (BPA)
- **Anello fenolico (posizioni C3 e C5, *orto* all'ossidrile $-	ext{OH}$)**: addizione elettrofila preferenziale di $ullet	ext{OH}$ con formazione di **3-idrossibisfenolo A** (catecolo-BPA, $	ext{C}_{15}	ext{H}_{16}	ext{O}_3$), rapidamente convertibile nel corrispondente bisfenolo A-3,4-chinone genotossico (*Applied Catalysis B 2014*, doi:10.1016/j.apcatb.2014.05.007). Confidenza: **alta**.
- **Doppia idrossilazione**: generazione di **3,3'-diidrossibisfenolo A** ($	ext{C}_{15}	ext{H}_{16}	ext{O}_4$, $m/z\ 259\ [	ext{M-H}]^-$). Confidenza: **media**.
- **Ponte isopropilidene centrale**: scissione ossidativa radicalica del ponte quaternario con liberazione di **4-idrossiacetofenone** ($	ext{C}_8	ext{H}_8	ext{O}_2$), **4-isopropenilfenolo** e **idrochinone** ($	ext{C}_6	ext{H}_6	ext{O}_2$). Confidenza: **alta**.

### 2.5 Caffeina
- **Carbonio C8 dell'anello imidazolico**: sito ad altissima reattività elettrofila; l'addizione di $ullet	ext{OH}$ sul doppio legame $	ext{C8=N9}$ forma l'intermedio addotto radicalico che evolve rapidamente in **1,3,7-trimetilacido urico** (8-idrossicaffeina, TMU, $	ext{C}_8	ext{H}_{10}	ext{N}_4	ext{O}_3$) (*Dalmázio et al., 2005*, doi:10.1021/es047985v). Confidenza: **alta**.
- **Gruppi metilici $N1$, $N3$, $N7$**: demetilazione ossidativa competitiva con formazione delle tre dimetilxantine isomeriche:
  - $N1$-demetilazione $	o$ **teobromina** (3,7-dimetilxantina, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$)
  - $N3$-demetilazione $	o$ **paraxantina** (1,7-dimetilxantina, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$)
  - $N7$-demetilazione $	o$ **teofillina** (1,3-dimetilxantina, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$).
  Tutte confermate con standard autentici in fotocatalisi e studi biochimici. Confidenza: **alta**.
- **Apertura anello imidazolico**: degradazione a **6-ammino-5-[formilmetilammino]-1,3-dimetiluracile** ($	ext{C}_7	ext{H}_{10}	ext{N}_4	ext{O}_3$). Confidenza: **media**.

### 2.6 Triclosan
- **Anello fenolico (posizione C6, *orto* all'ossidrile e *para* all'etere)**: idrossilazione aromatica a **4-cloro-2-(2,4-diclorofenossi)benzene-1,5-diolo** (idrossi-triclosan, $	ext{C}_{12}	ext{H}_7	ext{Cl}_3	ext{O}_3$) (*Schröder et al., 2020*, doi:10.3390/catal10121468). Confidenza: **media**.
- **Ponte etere difenilico**: rottura radicalica/fotolitica dell'etere con rilascio quantitativo di **2,4-diclorofenolo** (2,4-DCP, $	ext{C}_6	ext{H}_4	ext{Cl}_2	ext{O}$) e **4-clorocatecolo** ($	ext{C}_6	ext{H}_5	ext{ClO}_2$) (*Latch et al., 2005*, doi:10.1897/04-243r.1). Confidenza: **alta**.
- **Ciclizzazione intramolecolare fotochimica**: condensazione con espulsione di $	ext{HCl}$ a **2,8-diclorodibenzo-$p$-diossina** (2,8-DCDD, $	ext{C}_{12}	ext{H}_6	ext{Cl}_2	ext{O}_2$), confermata con standard puro. Confidenza: **alta**.

### 2.7 Ibuprofene
- **Carbonio terziario C2 della catena isobutilica**: astrazione favorita dell'idrogeno terziario (energia di legame $	ext{C–H}$ più bassa, $\sim 390	ext{ kJ/mol}$) con formazione di **2-idrossiibuprofene** ($	ext{C}_{13}	ext{H}_{18}	ext{O}_3$) (*Chemosphere 2019*, doi:10.1016/j.chemosphere.2018.10.053; *J. Xenobiotics 2024*, doi:10.3390/jox15010005). Confidenza: **alta**.
- **Carbonio secondario benzilico C1**: astrazione dell'idrogeno benzilico stabilizzato per risonanza con formazione di **1-idrossiibuprofene** ($	ext{C}_{13}	ext{H}_{18}	ext{O}_3$). Confidenza: **alta**.
- **Carbonio primario terminale C3**: idrossilazione e successiva ossidazione a **carbossiibuprofene** ($	ext{C}_{13}	ext{H}_{16}	ext{O}_4$). Confidenza: **alta**.
- **Catena dell'acido propionico**: decarbossilazione foto-ossidativa che produce **4-isobutilacetofenone** (4-IBAP, $	ext{C}_{12}	ext{H}_{16}	ext{O}$). Confidenza: **alta**.

### 2.8 Metoprololo
- **Posizione benzilica $	ext{C}_lpha$ della catena 2-metossietilica**: astrazione radicalica principale con idrossilazione ad **$lpha$-idrossimetoprololo** ($	ext{C}_{15}	ext{H}_{25}	ext{NO}_4$, $m/z\ 284\ [	ext{M+H}]^+$) (*Benner & Ternes, 2009*, doi:10.1021/es900280e; *J. Hazard. Mater. 2010*, doi:10.1016/j.jhazmat.2010.03.013). Confidenza: **alta**.
- **Anello aromatico (*orto* all'ossigeno etereo)**: addizione elettrofila sull'anello fenilico attivato che produce isomeri fenolici (**ar-idrossimetoprololo**, $	ext{C}_{15}	ext{H}_{25}	ext{NO}_4$). Confidenza: **media**.
- **Gruppo metossile terminale**: $O$-demetilazione ossidativa con formazione di **$O$-demetilmetoprololo** ($	ext{C}_{14}	ext{H}_{23}	ext{NO}_3$). Confidenza: **alta**.
- **Gruppo amminico secondario**: deochilazione ossidativa della catena isopropilamminica a **deisopropilmetoprololo** ($	ext{C}_{12}	ext{H}_{19}	ext{NO}_3$). Confidenza: **media**.

### 2.9 Benzofenone-3 (Oxybenzone, BP-3)
- **Gruppo metossilico C4**: $O$-demetilazione radicalica che produce **2,4-diidrossibenzofenone** (Benzofenone-1, BP-1, $	ext{C}_{13}	ext{H}_{10}	ext{O}_3$) (*J. Environ. Manage. 2016*, doi:10.1016/j.jenvman.2015.11.047; *J. Hazard. Mater. 2021*, doi:10.1016/j.jhazmat.2020.123591). Confidenza: **alta**.
- **Anello fenilico non sostituito (posizione C2', *orto*)**: idrossilazione aromatica a **2,2'-diidrossi-4-metossibenzofenone** (Benzofenone-8, BP-8, $	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidenza: **alta**.
- **Anello fenilico non sostituito (posizione C4', *para*)**: idrossilazione a **2,4'-diidrossi-4-metossibenzofenone** ($	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidenza: **media**.
- **Anello sostituito (posizione C5, *orto* all'ossidrile fenolico e *meta* al carbonile)**: addizione elettrofila di $ullet	ext{OH}$ a **2,5-diidrossi-4-metossibenzofenone** ($	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidenza: **media**.

### 2.10 Imidacloprid
- **Anello imidazolidinico C5**: idrossilazione alifatica diretta che produce **5-idrossiimidacloprid** ($	ext{C}_9	ext{H}_{10}	ext{ClN}_5	ext{O}_3$) (*Kitsiou et al., 2009*, doi:10.1016/j.apcatb.2008.07.018). Confidenza: **alta** (identificato con standard, LC-MS/MS e NMR).
- **Disidratazione anello imidazolidinico**: evoluzione rapida del 5-idrossi derivato a **imidacloprid-olefina** ($	ext{C}_9	ext{H}_8	ext{ClN}_5	ext{O}_2$). Confidenza: **alta**.
- **Gruppo nitroimminico**: idrolisi e ossidazione radicalica del sostituente $=	ext{N–NO}_2$ a urea ciclica (**imidacloprid-urea**, $	ext{C}_9	ext{H}_{10}	ext{ClN}_3	ext{O}$). Confidenza: **alta**.
- **Ponte metilenico $-	ext{CH}_2–$**: scissione ossidativa tra il nucleo piridinico e l'anello imidazolidinico con formazione di **acido 6-cloronicotinico** (6-CNA, $	ext{C}_6	ext{H}_4	ext{ClNO}_2$). Confidenza: **alta**.

---

## 3. Tabella di Verifica Incrociata dei DOI Citati (Crossref HTTP API)

Tutti i 22 DOI inseriti nei file di traccia T8 sono stati verificati in modo deterministico interrogando direttamente il database Crossref (`https://api.crossref.org/works/<doi>`).

| DOI | Anno | Rivista / Container | Titolo Ufficiale Verificato | Esito Controllo |
|---|---|---|---|---|
| `10.1021/es050794n` | 2005 | *Environmental Science &amp; Technology* | Photo-Fenton Degradation of Diclofenac:  Identification of Main Intermediates and Degradation Pathway | **Conforme** |
| `10.1016/j.apcatb.2006.04.021` | 2006 | *Applied Catalysis B: Environmental* | Photocatalytic degradation study of diclofenac over aqueous TiO2 suspensions | **Conforme** |
| `10.1016/j.watres.2009.04.006` | 2009 | *Water Research* | Degradation of sulfamethoxazole in water by solar photo-Fenton. Chemical and toxicological evaluation | **Conforme** |
| `10.1021/es001271c` | 2001 | *Environmental Science &amp; Technology* | Photocatalytic Degradation of Selected s-Triazine Herbicides and Organophosphorus Insecticides over Aqueous TiO<sub>2</sub> Suspensions | **Conforme** |
| `10.3390/molecules27249021` | 2022 | *Molecules* | Real-Time Monitoring of the Atrazine Degradation by Liquid Chromatography and High-Resolution Mass Spectrometry: Effect of Fenton Process and Ultrasound Treatment | **Conforme** |
| `10.1016/j.apcatb.2014.05.007` | 2014 | *Applied Catalysis B: Environmental* | Formation of genotoxic quinones during bisphenol A degradation by TiO2 photocatalysis and UV photolysis: A comparative study | **Conforme** |
| `10.1021/es047985v` | 2005 | *Environmental Science &amp; Technology* | Advanced Oxidation of Caffeine in Water:  On-Line and Real-Time Monitoring by Electrospray Ionization Mass Spectrometry | **Conforme** |
| `10.3390/catal10121468` | 2020 | *Catalysts* | Photocatalytic Transformation of Triclosan. Reaction Products and Kinetics | **Conforme** |
| `10.1897/04-243r.1` | 2005 | *Environmental Toxicology and Chemistry* | Aqueous photochemistry of triclosan: Formation of 2,4-dichlorophenol, 2,8-dichlorodibenzo-<i>p</i>-dioxin, and oligomerization products | **Conforme** |
| `10.1016/j.chemosphere.2018.10.053` | 2019 | *Chemosphere* | Photocatalytic degradation of ibuprofen in water using TiO2/UV and g-C3N4/visible light: Study of intermediate degradation products by liquid chromatography coupled to high-resolution mass spectrometry | **Conforme** |
| `10.1021/es900280e` | 2009 | *Environmental Science &amp; Technology* | Ozonation of Metoprolol: Elucidation of Oxidation Pathways and Major Oxidation Products | **Conforme** |
| `10.1016/j.jhazmat.2010.03.013` | 2010 | *Journal of Hazardous Materials* | UV-C radiation based methods for aqueous metoprolol elimination | **Conforme** |
| `10.1016/j.jenvman.2015.11.047` | 2016 | *Journal of Environmental Management* | Heterogeneous photocatalytic degradation of the endocrine-disrupting chemical Benzophenone-3: Parameters optimization and by-products identification | **Conforme** |
| `10.1016/j.jhazmat.2020.123591` | 2021 | *Journal of Hazardous Materials* | Benzophenone-3 degradation via UV/H2O2 and UV/persulfate reactions | **Conforme** |
| `10.1016/j.apcatb.2008.07.018` | 2009 | *Applied Catalysis B: Environmental* | Heterogeneous and homogeneous photocatalytic degradation of the insecticide imidacloprid in aqueous solutions | **Conforme** |
| `10.3390/jox15010005` | 2024 | *Journal of Xenobiotics* | Removal of Ibuprofen in Water by Bioaugmentation with Labrys neptuniae CSW11 Isolated from Sewage Sludge—Assessment of Biodegradation Pathway Based on Metabolite Formation and Genomic Analysis | **Conforme** |
| `10.1155/2018/2085059` | 2018 | *Journal of Analytical Methods in Chemistry* | HPLC Method for Quantification of Caffeine and Its Three Major Metabolites in Human Plasma Using Fetal Bovine Serum Matrix to Evaluate Prenatal Drug Exposure | **Conforme** |
| `10.1021/tx9802365` | 1999 | *Chemical Research in Toxicology* | Metabolic Activation of Diclofenac by Human Cytochrome P450 3A4:  Role of 5-Hydroxydiclofenac | **Conforme** |
| `10.1016/j.arabjc.2021.103635` | 2022 | *Arabian Journal of Chemistry* | Aromatic free Fenton process for rapid removal of phenol from refinery wastewater in an oscillatory baffled reactor | **Conforme** |
| `10.55730/1300-0144.6374` | 2026 | *Turkish Journal of Medical Sciences* | Antiobesity, antidiabetic, and antioxidant activities of selected Rubia species (Rubiaceae) and LC–MS/MS-based chemical profiling ​ | **Conforme** |
| `10.1021/es050824m` | 2005 | *Environmental Science &amp; Technology* | Aqueous Atmospheric Chemistry:  Formation of 2,4-Dinitrophenol upon Nitration of 2-Nitrophenol and 4-Nitrophenol in Solution | **Conforme** |
| `10.3390/ijms262010063` | 2025 | *International Journal of Molecular Sciences* | Sodium Hypochlorite-Assisted Photooxidation of Salicylic Acid: Degradation Kinetics, Formation, and Ecotoxicological Assessment of Intermediates | **Conforme** |

---

## 4. Coppie di Isomeri di Trasformazione con Tempi di Ritenzione Misurati in RP-LC

Sono state identificate e validate **12 coppie reali di isomeri posizionali** (stessa formula chimica neutra e stessa massa monoisotopica esatta) derivanti da processi degradativi o metabolici, separate mediante cromatografia a fase inversa (colonne RP-C18) con parametri analitici rigorosamente tracciati.

### Sintesi delle 12 Coppie Verificate (`coppie_isomeri_rt.csv`):
1. **Ibuprofene**: 2-idrossiibuprofene ($t_R = 7.64	ext{ min}$) vs 1-idrossiibuprofene ($t_R = 7.84	ext{ min}$) su Halo C-18 ($50 	imes 4.6	ext{ mm}$, $2.7\ \mu	ext{m}$). L'ossidrile terziario alifatico in C2 riduce l'interazione idrofobica più dell'ossidrile secondario benzilico in C1 (*J. Xenobiotics 2024*, doi:10.3390/jox15010005).
2. **Caffeina**: Teobromina ($t_R = 3.8	ext{ min}$) vs Paraxantina ($t_R = 4.8	ext{ min}$) su Zorbax SB-Aq ($2.1 	imes 100	ext{ mm}$, $3.5\ \mu	ext{m}$) con gradiente tampone fosfato/ACN (*J. Anal. Methods Chem. 2018*, doi:10.1155/2018/2085059).
3. **Caffeina**: Paraxantina ($t_R = 4.8	ext{ min}$) vs Teofillina ($t_R = 5.5	ext{ min}$) sulla medesima colonna Zorbax SB-Aq. Risoluzione critica di una coppia tradizionalmente coeluente su fasi C18 convenzionali isocratiche.
4. **Caffeina**: Teobromina ($t_R = 3.8	ext{ min}$) vs Teofillina ($t_R = 5.5	ext{ min}$) ($\Delta t_R = 1.7	ext{ min}$).
5. **Diclofenac**: 4'-idrossidiclofenac ($t_R = 25.4	ext{ min}$) vs 5-idrossidiclofenac ($t_R = 26.8	ext{ min}$) su Supelcosil LC-18 ($250 	imes 4.6	ext{ mm}$, $5\ \mu	ext{m}$) con gradiente metanolo/acqua acidificata (*Chem. Res. Toxicol. 1999*, doi:10.1021/tx9802365).
6. **Fenolo**: Idrochinone ($t_R = 5.5	ext{ min}$) vs Catecolo ($t_R = 10.9	ext{ min}$) su Nucleosil 100-5 C18 ($150 	imes 4.6	ext{ mm}$, $5\ \mu	ext{m}$) in isocratica tampone/MeOH/THF (*Arab. J. Chem. 2022*, doi:10.1016/j.arabjc.2021.103635). L'idrochinone eluisce per primo a causa della forte solvatazione polare rispetto alla chelazione intramolecolare parziale del catecolo.
7. **Fenolo**: Idrochinone ($t_R = 5.5	ext{ min}$) vs Resorcina ($t_R = 9.5	ext{ min}$) sulla medesima colonna Nucleosil C18.
8. **Fenolo**: Resorcina ($t_R = 9.5	ext{ min}$) vs Catecolo ($t_R = 10.9	ext{ min}$) ($\Delta t_R = 1.4	ext{ min}$).
9. **Acido benzoico**: Acido 4-idrossibenzoico ($t_R = 10.5	ext{ min}$) vs Acido salicilico (2-idrossibenzoico, $t_R = 21.8	ext{ min}$) su Poroshell 120 EC-C18 ($150 	imes 2.1	ext{ mm}$, $2.7\ \mu	ext{m}$) (*Turk. J. Med. Sci. 2026*, doi:10.55730/1300-0144.6374). L'acido salicilico è trattenuto oltre il doppio del tempo a causa del legame idrogeno intramolecolare tra $-	ext{OH}$ in *orto* e carbossile, che scherma la polarità verso la fase mobile acquosa.
10. **Nitrobenzene**: 4-nitrofenolo ($t_R = 5.1	ext{ min}$) vs 2-nitrofenolo ($t_R = 9.2	ext{ min}$) su LiChrospher 100 RP-18 ($125 	imes 4	ext{ mm}$, $5\ \mu	ext{m}$) (*Environ. Sci. Technol. 2005*, doi:10.1021/es050824m). L'isomero *orto* forma un forte ponte a idrogeno intramolecolare $	ext{OH}\cdots	ext{O=N}$, aumentando drasticamente l'idrofobicità apparente su C18.
11. **Nitrobenzene**: 2,4-dinitrofenolo ($t_R = 8.3	ext{ min}$) vs 2,6-dinitrofenolo ($t_R = 9.4	ext{ min}$) sulla medesima colonna LiChrospher RP-18. L'isomero 2,6 possiede due gruppi nitro in orto all'ossidrile, massimizzando la chelazione intramolecolare e il ritardo di ritenzione.
12. **Clorobenzene / Acido salicilico**: 2,6-diclorofenolo ($t_R = 10.682	ext{ min}$) vs 2,4-diclorofenolo ($t_R = 11.063	ext{ min}$) su colonna C18 core-shell ($100 	imes 3	ext{ mm}$, $2.6\ \mu	ext{m}$) con gradiente acqua/metanolo (*Int. J. Mol. Sci. 2025*, doi:10.3390/ijms262010063).

---

## 5. Considerazioni Critiche sui Limiti dei Dati Analitici

1. **Riconoscimento di isomeri posizionali in assenza di standard**:
   - In spettrometria di massa a bassa o alta risoluzione senza frammentazione specifica, isomeri di idrossilazione aromatica (es. 2-OH vs 3-OH-carbamazepina, oppure 2-OH vs 4-OH-SMX) generano precursori $m/z$ e profili di frammentazione tandem quasi indistinguibili.
   - Solo la separazione cromatografica su fase stazionaria idonea o il confronto con standard analitici commerciali/sintetizzati consente un'assegnazione univoca del sito (confidenza *alta*).
2. **Effetto del legame idrogeno intramolecolare sull'ordine di eluizione**:
   - Nei sostituenti orientati in *orto* con donatori e accettori di legami a idrogeno (es. acido salicilico, 2-nitrofenolo, 2-clorofenolo), il legame idrogeno intramolecolare scherma i gruppi polari dall'ambiente acquoso. Di conseguenza, contrariamente all'intuizione topologica generica, **l'isomero orto risulta spesso molto più lipofilo ed eluisce dopo l'isomero para o meta** in cromatografia a fase inversa C18.
3. **Cinetica di reazione e prodotti transienti vs persistenti**:
   - Nei processi AOPs, molti addotti primari di radicale $ullet	ext{OH}$ (es. addotti cicloesadienilici o radicali idrossialchilici) hanno vite medie inferiori al microsecondo e subiscono disidratazione, eliminazione o ulteriore ossidazione rapida (es. 5-idrossiimidacloprid $	o$ olefina; catecolo $	o$ orto-chinone). Gli studi analitici misurano i prodotti stabili finali o intermedi sufficientemente longevi per l'analisi LC.
4. **Valore per il motore di calcolo mzLab**:
   - I dati quantitativi raccolti in `coppie_isomeri_rt.csv` e `siti_sperimentali.csv` forniscono una base di test oggettiva sia per il modulo quantomeccanico di orientazione Fukui (T2/T7) sia per i test di ordinamento di ritenzione RP-LC (T5).
