# Rosina B: fonti pubbliche per formato ed esperimento

Catalogo delle fonti da cui prendere i file di prova. Qui solo accession, licenza, peso, checksum e ridistribuibilità: i nomi dei file (che possono contenere nomi di persone o di campioni clinici) stanno solo nel repository privato (`dati/formati/INDICE.md`; `INDICE.tsv` di `ROSINA.md` D). Misurato il 19/10/2026.

## Regole di ridistribuzione
| Licenza della fonte | Nel privato | Nel pubblico (`tests/formati/`) |
|---|---|---|
| CC0 (PRIDE di solito, parte di MetaboLights e Zenodo) | sì | sì, ≤ 1 MB per file, dopo il sì di Federico, con `ATTRIBUZIONI.md` |
| CC-BY 4.0 (Zenodo, Workbench) | sì | sì, alle stesse condizioni, con l'attribuzione completa |
| CC-BY-SA, CC-BY-NC | sì | no |
| «EMBL-EBI Terms of Use» (molti studi MetaboLights e PRIDE vecchi) | sì (uso di ricerca) | no |
| Workbench senza licenza nel record | sì | no fino a verifica della licenza, studio per studio |
| Fixture di ProteoWizard e dei lettori liberi (repository Apache/MIT) | sì | no: i file dei produttori dentro un repository Apache non diventano Apache |
| Dati del laboratorio (`mzML/`, `dam/`, `HRMS/`) | sì | mai |
| Personale o clinico (GDPR) | solo se anonimo nel deposito | mai |

## Corpus privato attuale (191 file, 1,3 GB) per famiglia
Conteggi dalla tabella dell'indice privato (righe; «rif» = mzML o altro riferimento accanto). Licenze: CC0, CC-BY, EBI, MIT/Apache (fixture), altro (Panorama, senza licenza).

| Famiglia ed esperimento | Righe | Licenze | Rif | Accession |
|---|---|---|---|---|
| Thermo TSQ SRM (Vantage, Quantum Ultra, Quantiva, Altis) | 11 | CC0 6, EBI 5 | no | PXD019516, PXD057351, PXD064022, PXD006054, PXD019525, MTBLS7889 (polarità mista), MTBLS5229, MTBLS1807, MTBLS1752, MTBLS3887, MTBLS14496 |
| Thermo Orbitrap, Q Exactive, Exploris | 10 | CC0 9, EBI 1 | no | PXD077492, PXD064999, PXD067770, PXD059868, PXD078272, PXD074730, MTBLS14155, MTBLS13497, MTBLS825, MTBLS14724 |
| Thermo con PDA/UV | 4 | EBI 4 | no | MTBLS804, MTBLS2956, MTBLS775, MTBLS9839 |
| Thermo GC-MS/MS | 1 | EBI | no | MTBLS5358 |
| mzML Fusion Lumos MS3 | 1 | CC0 | sì | PXD060469 |
| SCIEX QTRAP 4000/4500/5500/6500/6500+, API 4000, Triple Quad 5500 | 34 | CC-BY 16, CC0 6, EBI 11 | 3 (MTBLS1381) | Zenodo 22807392 (20 campioni), 20759805, 15679243, 7845394, 12635820, 14243265, 20593417, 14418483, 17974433 (canali ausiliari); PXD035159, PXD001506, PXD005017, PXD006494, PXD067154; MTBLS14185, MTBLS11713, MTBLS1381 |
| SCIEX PIS, NL, EPI, EMS | 17 | CC0, CC-BY | 9 (Zenodo 7897576) | Zenodo 7897576, 12635820; PXD024331, PXD022088 |
| SCIEX `.wiff` + `.wiff2` (SCIEX OS, QTRAP 5500, 6500+) | 6 | CC-BY | no | Zenodo 22116234, 21792170 |
| SCIEX TripleTOF 5600/6600, ZenoTOF 7600 (anche EAD), X500, QStar | 21 | CC0 11, EBI 7, CC-BY 2 | 2 | PXD014141, PXD022400, PXD043474, PXD054948, PXD080006, PXD066214, PXD063182; MTBLS10565, MTBLS14832, MTBLS10170; Zenodo 7735897, 4961023 |
| SCIEX Panorama (Skyline) | 2 | altro | no | PXD011113 |
| Agilent `.D` LC/MS (QqQ, Q-TOF, DAD) e GC/MS | 21 | EBI 7, CC0 3, altro 11 | 8 | ST004922, ST004061, ST003656, ST003651, ST000188, ST000185, ST000768; MTBLS253, MTBLS13437, MTBLS1133, MTBLS599, MTBLS874, MTBLS13731, MTBLS10408, MTBLS787, MTBLS7740, MTBLS13991 |
| Waters `.raw` (TQ, G2, G2-S, Synapt) | 13 | EBI 8, CC-BY 2, CC0 1, altro 2 | 2 (ST004453) | ST004453; MTBLS12109, MTBLS685, MTBLS2612, MTBLS661, MTBLS1584, MTBLS1198, MTBLS178, MTBLS957; PXD052561; Zenodo |
| Shimadzu `.lcd`, `.qgd` | 14 | EBI 6, CC-BY 3, CC0 3, altro 2 | no | MTBLS7425, MTBLS2376, MTBLS14423, MTBLS432, MTBLS1960, MTBLS5861, MTBLS5383, MTBLS13996, MTBLS14089; ST004089; Zenodo |
| netCDF/AIA GC-MS | 3 | EBI 2, CC0 1 | `.D` della stessa corsa | MTBLS7740, MTBLS13991, MTBLS8819 |
| mzXML | 3 | CC0 2, EBI 1 | no | PXD070375, PXD005665, PXD003625 |
| Bruker timsTOF TDF (DDA, diaPASEF, MRM), TSF, miniTDF | 9 | CC0 5, CC-BY 4 | no | PXD074950 (WAL), PXD054441, PXD080079, PXD050530; Zenodo |
| Bruker BAF (solariX, maXis), MALDI flex | 6 | CC0 3, CC-BY 3 | no | PXD074784, PXD011687, PXD022404; Zenodo |
| Fixture ProteoWizard (Thermo, Agilent 6560, Bruker TDF/BAF/YEP/flex, Waters HDMS/SONAR, UIMF) e mzML con mobilità | 47 | Apache/MIT (repository) | sì quasi tutte | repository ProteoWizard e lettori liberi |
| mzML numpress | 3 | Apache/BSD | sì | repository OpenMS, pymzML |

## ELENCO-1 (da scaricare: chat locale, ≈ 0,8 GB in tutto)
Scelto per chiudere le celle vuote di `ROSINA.md` F.4 e per dare un riferimento (mzML degli autori o oracolo) ai formati che non ne hanno. Le righe complete (URL, nomi dei file) vanno alla chat locale per messaggio e in `mzlab-dati/coordinamento/dati_esterni/rosina/ELENCO-1.tsv`, mai qui.

| # | Fonte | Strumento, esperimento | Byte | Checksum | Licenza | Scopo |
|---|---|---|---|---|---|---|
| 1 | Workbench ST003942 | Waters Xevo TQ Absolute, MRM, raw + mzML | 24,9 M + 23 M | n/d | da verificare | V2 Waters MRM |
| 2 | Workbench ST003984 | Waters Xevo TQ-S, MRM, raw + mzML | 38,9 M + 26,6 M | n/d | da verificare | V2 Waters MRM |
| 3 | Workbench ST004286 | Agilent 6490 QqQ, `.d` + mzML (un run ~10 MB per range HTTP) | 72,2 M + 35,2 M | n/d | da verificare | V2 Agilent QqQ |
| 4 | MTBLS7394 | Waters Synapt G2, MSe + lock spray + mobilità | 35,2 M | n/d | EBI | lock mass, deriva |
| 5 | MTBLS12691 | Shimadzu LCMS-8060, MRM | 12 M | sha256 b187ac65… | EBI | V1 Shimadzu QqQ |
| 6 | MTBLS13204 | Shimadzu LCMS-9030 Q-TOF, pos/neg | 54,3 M | sha256 a71abdeb… | EBI | `shimadzu-tof`, polarità |
| 7 | Zenodo 18633076 | Shimadzu LCMS-2020 + PDA | 87,6 M | md5 976d9c74… | CC-BY 4.0 | PDA Shimadzu |
| 8 | Workbench ST001609 | Shimadzu GCMS-QP2010 Ultra, `.qgd` (zip annidato) | 2,0 M | md5 03e00619… | da verificare | `.qgd` + `.cdf` esportato? |
| 9 | MTBLS427 | Agilent 6560 IM-QTOF, profilo, ESI− | 24,3 M | sha256 pubblicato | EBI | decisione 6560 |
| 10 | PXD043691 | Thermo TSQ Quantum Access, SRM | 64,8 M | SHA-1 4e851a06… | CC0 | TSQ vecchio in `opentfraw` |
| 11 | MTBLS2956 | Thermo Q Exactive + UltiMate 3000, UV 520 nm | 52,9 M | sha256 c0e98389… | EBI | UV/PDA Thermo |
| 12 | Zenodo 7897576 | SCIEX QTRAP 6500, NL 77, `.wiff` + `.scan` + mzML | 2,7 M + 10,3 M + 11,0 M | md5 | CC0 | V2 NL in più |
| 13 | MTBLS13023 | SCIEX ZenoTOF, MRM-HR | ≈ 8,3 M + 21 M | sha256 fc39e44a…, f3a15977… | EBI | `sciex-tof` |
| 14 | Zenodo 10889321 | SCIEX ZenoTOF 7600, `.wiff` + `.scan` + `.wiff2` | 7,3 M + 61,2 M + 14,5 M | md5 | CC-BY 4.0 | `sciex-tof`, rifiuto `.wiff2` |
| 15 | PXD037766 | Bruker timsTOF Pro 2, Synchro-PASEF | 43,6 M | non pubblicato | CC0 | compressione 2, 1/K0 |
| 16 | PXD058337 | Bruker timsTOF HT, DDA (GPF) | 29,9 M | non pubblicato | CC0 | compressione 2, 1/K0 |
| 17 | PXD051519 | Agilent 6530A Q-TOF nanoLC chip, DDA | 47,2 M | `checksum.txt` PRIDE | CC0 | Q-TOF DDA |
| 18 | Zenodo 22673685 | SCIEX, LC-MS a doppia polarità, `.wiff` + `.scan` | 1,8 M + 90,9 M | md5 aef1947d…, bb646a1d… | da verificare | polarità mista SCIEX |

## Fonti da esplorare (non ancora nel corpus)
| Fonte | Cosa cercare | Accesso | Nota |
|---|---|---|---|
| MassIVE | conteggio dei formati e strumenti (manca nel censimento A); file Waters/Agilent con mzML | FTP `massive.ucsd.edu` (non raggiungibile dal cloud) | chat locale (LOC-1) |
| Panorama Public | SRM/MRM di SCIEX, Thermo, Agilent, Waters con i documenti Skyline (aree come V5) | HTTPS, CC-BY 4.0 dichiarata dal portale (da verificare per progetto) | ottimo V5: aree integrate dagli autori |
| NORMAN DSFP | LC-HRMS ambientale (stesso dominio del laboratorio) | portale, mzML/mzXML convertiti | spesso senza il nativo: utile per mzML altrui |
| GNPS/MassIVE MSV | DDA metabolomica di molti strumenti | FTP | come MassIVE |
| Metabolomics Workbench (`studydownload`) | studi con nativo + mzML (`*_mzml.zip`) per ogni produttore | HTTPS (timeout lunghi) | già la fonte migliore di V2 |
| Test di ProteoWizard (`pwiz/data/vendor_readers`) | nativo + mzML di riferimento per ogni lettore | git (grande) | resta privato; V2 per casi rari (YEP, UIMF, BAF) |
| SCIEX, Thermo, Agilent, Waters «demo data» | file d'esempio dei software (licenza del produttore) | registrazione | solo privato, solo se la licenza lo permette |
| Laboratorio (3200 QTRAP) | file costruiti apposta: periodi multipli, MS3, ER, polarità mista, sMRM, DAD | Analyst al laboratorio | Federico decide; restano nel privato |
