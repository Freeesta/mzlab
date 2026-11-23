# Rosina B: public sources per format and experiment

Catalogue of the sources for test files. Only accession, licence, size, checksum and redistributability are listed here: file names (which can contain names of people or clinical samples) live only in the private repo (`dati/formati/INDICE.md`; the `INDICE.tsv` of `ROSINA.md` D). Measured 2026-10-19.

## Redistribution rules
| Source licence | In the private repo | In the public repo (`tests/formati/`) |
|---|---|---|
| CC0 (PRIDE usually, part of MetaboLights and Zenodo) | yes | yes, <= 2 MB per file, proposed by the reader's DEV chat, after the maintainer's yes, with `ATTRIBUZIONI.md` |
| CC-BY 4.0 (Zenodo, Workbench) | yes | yes, same conditions, with full attribution |
| CC-BY-SA, CC-BY-NC | yes | no |
| "EMBL-EBI Terms of Use" (many MetaboLights and old PRIDE studies) | yes (research use) | no |
| Undeclared licence (Workbench with no licence in the record, Zenodo with no licence) | yes | no |
| Fixtures of ProteoWizard and of the free readers (Apache/MIT repositories) | yes | no: vendor files inside an Apache repository do not become Apache |
| Lab data (`mzML/`, `dam/`, `HRMS/`) | yes | never |
| Personal or clinical (GDPR) | only if anonymous in the deposit | never |

## Current private corpus (191 files, 1.3 GB) by family
Counts from the private index table (rows; "ref" = mzML or other reference next to the file). Licences: CC0, CC-BY, EBI, MIT/Apache (fixtures), other (Panorama, no licence).

| Family and experiment | Rows | Licences | Ref | Accession |
|---|---|---|---|---|
| Thermo TSQ SRM (Vantage, Quantum Ultra, Quantiva, Altis) | 11 | CC0 6, EBI 5 | no | PXD019516, PXD057351, PXD064022, PXD006054, PXD019525, MTBLS7889 (mixed polarity), MTBLS5229, MTBLS1807, MTBLS1752, MTBLS3887, MTBLS14496 |
| Thermo Orbitrap, Q Exactive, Exploris | 10 | CC0 9, EBI 1 | no | PXD077492, PXD064999, PXD067770, PXD059868, PXD078272, PXD074730, MTBLS14155, MTBLS13497, MTBLS825, MTBLS14724 |
| Thermo with PDA/UV | 4 | EBI 4 | no | MTBLS804, MTBLS2956, MTBLS775, MTBLS9839 |
| Thermo GC-MS/MS | 1 | EBI | no | MTBLS5358 |
| mzML Fusion Lumos MS3 | 1 | CC0 | yes | PXD060469 |
| SCIEX QTRAP 4000/4500/5500/6500/6500+, API 4000, Triple Quad 5500 | 34 | CC-BY 16, CC0 6, EBI 11 | 3 (MTBLS1381) | Zenodo 22807392 (20 samples), 20759805, 15679243, 7845394, 12635820, 14243265, 20593417, 14418483, 17974433 (auxiliary channels); PXD035159, PXD001506, PXD005017, PXD006494, PXD067154; MTBLS14185, MTBLS11713, MTBLS1381 |
| SCIEX PIS, NL, EPI, EMS | 17 | CC0, CC-BY | 9 (Zenodo 7897576) | Zenodo 7897576, 12635820; PXD024331, PXD022088 |
| SCIEX `.wiff` + `.wiff2` (SCIEX OS, QTRAP 5500, 6500+) | 6 | CC-BY | no | Zenodo 22116234, 21792170 |
| SCIEX TripleTOF 5600/6600, ZenoTOF 7600 (also EAD), X500, QStar | 21 | CC0 11, EBI 7, CC-BY 2 | 2 | PXD014141, PXD022400, PXD043474, PXD054948, PXD080006, PXD066214, PXD063182; MTBLS10565, MTBLS14832, MTBLS10170; Zenodo 7735897, 4961023 |
| SCIEX Panorama (Skyline) | 2 | other | no | PXD011113 |
| Agilent `.D` LC/MS (QqQ, Q-TOF, DAD) and GC/MS | 21 | EBI 7, CC0 3, other 11 | 8 | ST004922, ST004061, ST003656, ST003651, ST000188, ST000185, ST000768; MTBLS253, MTBLS13437, MTBLS1133, MTBLS599, MTBLS874, MTBLS13731, MTBLS10408, MTBLS787, MTBLS7740, MTBLS13991 |
| Waters `.raw` (TQ, G2, G2-S, Synapt) | 13 | EBI 8, CC-BY 2, CC0 1, other 2 | 2 (ST004453) | ST004453; MTBLS12109, MTBLS685, MTBLS2612, MTBLS661, MTBLS1584, MTBLS1198, MTBLS178, MTBLS957; PXD052561; Zenodo |
| Shimadzu `.lcd`, `.qgd` | 14 | EBI 6, CC-BY 3, CC0 3, other 2 | no | MTBLS7425, MTBLS2376, MTBLS14423, MTBLS432, MTBLS1960, MTBLS5861, MTBLS5383, MTBLS13996, MTBLS14089; ST004089; Zenodo |
| netCDF/AIA GC-MS | 3 | EBI 2, CC0 1 | `.D` of the same run | MTBLS7740, MTBLS13991, MTBLS8819 |
| mzXML | 3 | CC0 2, EBI 1 | no | PXD070375, PXD005665, PXD003625 |
| Bruker timsTOF TDF (DDA, diaPASEF, MRM), TSF, miniTDF | 9 | CC0 5, CC-BY 4 | no | PXD074950 (WAL), PXD054441, PXD080079, PXD050530; Zenodo |
| Bruker BAF (solariX, maXis), MALDI flex | 6 | CC0 3, CC-BY 3 | no | PXD074784, PXD011687, PXD022404; Zenodo |
| ProteoWizard fixtures (Thermo, Agilent 6560, Bruker TDF/BAF/YEP/flex, Waters HDMS/SONAR, UIMF) and mzML with mobility | 47 | Apache/MIT (repository) | yes, almost all | ProteoWizard repository and free readers |
| mzML numpress | 3 | Apache/BSD | yes | OpenMS and pymzML repositories |

## ELENCO-1 (to download: local chat, about 0.8 GB in all)
Chosen to fill the empty cells of `ROSINA.md` F.4 and to give a reference (authors' mzML or oracle) to the formats that have none. Complete rows (URL, file names) go to the local chat by message and into `mzlab-dati/coordinamento/dati_esterni/rosina/ELENCO-1.tsv`, never here.

| # | Source | Instrument, experiment | Bytes | Checksum | Licence | Purpose |
|---|---|---|---|---|---|---|
| 1 | Workbench ST003942 | Waters Xevo TQ Absolute, MRM, raw + mzML | 24.9 M + 23 M | n/a | undeclared (no redistribution) | V2 Waters MRM |
| 2 | Workbench ST003984 | Waters Xevo TQ-S, MRM, raw + mzML | 38.9 M + 26.6 M | n/a | undeclared (no redistribution) | V2 Waters MRM |
| 3 | Workbench ST004286 | Agilent 6490 QqQ, `.d` + mzML (one run ~10 MB by HTTP range) | 72.2 M + 35.2 M | n/a | undeclared (no redistribution) | V2 Agilent QqQ |
| 4 | MTBLS7394 | Waters Synapt G2, MSe + lock spray + mobility | 35.2 M | n/a | EBI | lock mass, drift |
| 5 | MTBLS12691 | Shimadzu LCMS-8060, MRM | 12 M | sha256 b187ac65... | EBI | V1 Shimadzu QqQ |
| 6 | MTBLS13204 | Shimadzu LCMS-9030 Q-TOF, pos/neg | 54.3 M | sha256 a71abdeb... | EBI | `shimadzu-tof`, polarity |
| 7 | Zenodo 18633076 | Shimadzu LCMS-2020 + PDA | 87.6 M | md5 976d9c74... | CC-BY 4.0 | Shimadzu PDA |
| 8 | Workbench ST001609 | Shimadzu GCMS-QP2010 Ultra, `.qgd` (nested zip) | 2.0 M | md5 03e00619... | undeclared (no redistribution) | `.qgd` + exported `.cdf`? |
| 9 | MTBLS427 | Agilent 6560 IM-QTOF, profile, ESI- | 24.3 M | sha256 published | EBI | 6560 decision |
| 10 | PXD043691 | Thermo TSQ Quantum Access, SRM | 64.8 M | SHA-1 4e851a06... | CC0 | old TSQ in `opentfraw` |
| 11 | MTBLS2956 | Thermo Q Exactive + UltiMate 3000, UV 520 nm | 52.9 M | sha256 c0e98389... | EBI | Thermo UV/PDA |
| 12 | Zenodo 7897576 | SCIEX QTRAP 6500, NL 77, `.wiff` + `.scan` + mzML | 2.7 M + 10.3 M + 11.0 M | md5 | CC0 | extra V2 NL |
| 13 | MTBLS13023 | SCIEX ZenoTOF, MRM-HR | about 8.3 M + 21 M | sha256 fc39e44a..., f3a15977... | EBI | `sciex-tof` |
| 14 | Zenodo 10889321 | SCIEX ZenoTOF 7600, `.wiff` + `.scan` + `.wiff2` | 7.3 M + 61.2 M + 14.5 M | md5 | CC-BY 4.0 | `sciex-tof`, `.wiff2` refusal |
| 15 | PXD037766 | Bruker timsTOF Pro 2, Synchro-PASEF | 43.6 M | not published | CC0 | compression 2, 1/K0 |
| 16 | PXD058337 | Bruker timsTOF HT, DDA (GPF) | 29.9 M | not published | CC0 | compression 2, 1/K0 |
| 17 | PXD051519 | Agilent 6530A Q-TOF nanoLC chip, DDA | 47.2 M | PRIDE `checksum.txt` | CC0 | Q-TOF DDA |
| 18 | Zenodo 22673685 | SCIEX, dual-polarity LC-MS, `.wiff` + `.scan` | 1.8 M + 90.9 M | md5 aef1947d..., bb646a1d... | undeclared (no redistribution) | SCIEX mixed polarity |

## Sources to explore (not yet in the corpus)
| Source | What to look for | Access | Note |
|---|---|---|---|
| MassIVE | file extensions per dataset (instruments are counted in `ROSINA.md` A.4); Waters/Agilent files with mzML | FTP `massive.ucsd.edu` (unreachable from the cloud) | local chat |
| Panorama Public | SRM/MRM of SCIEX, Thermo, Agilent, Waters with Skyline documents (areas as V5) | HTTPS, CC-BY 4.0 declared by the portal (verify per project) | great V5: areas integrated by the authors |
| NORMAN DSFP | environmental LC-HRMS (same domain as the lab) | portal, converted mzML/mzXML | often without the native file: useful for third-party mzML |
| GNPS/MassIVE MSV | metabolomics DDA from many instruments | FTP | like MassIVE |
| Metabolomics Workbench (`studydownload`) | studies with native + mzML (`*_mzml.zip`) for every vendor | HTTPS (long timeouts) | already the best V2 source |
| ProteoWizard tests (`pwiz/data/vendor_readers`) | native + reference mzML for every reader | git (large) | stays private; V2 for rare cases (YEP, UIMF, BAF) |
| SCIEX, Thermo, Agilent, Waters "demo data" | sample files shipped with the software (vendor licence) | registration | private only, only if the licence allows |
| Lab (3200 QTRAP) | purpose-built files: multiple periods, MS3, ER, mixed polarity, sMRM, DAD | Analyst in the lab | the maintainer decides; they stay private |
