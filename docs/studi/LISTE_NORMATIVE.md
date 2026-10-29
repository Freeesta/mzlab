# Current EU and US regulatory chemical lists in mzLab, recognised by InChIKey

Study for the NTS suspect-screening area. Federico decided on 29/10/2026 that this is the first piece of that area, and that it must work in both the site and the app, with byte parity. This study is not product code. The build prompts are in `docs/studi/prompt_liste_normative/`, and the decisions that only Federico can take are in §9.

Every statement about a law cites its source and was retrieved on **2026-10-29**, unless a row says otherwise. "UNVERIFIED" means the point could not be confirmed from an official page. Two access limits shaped the method:
- **EUR-Lex** blocks scripts. EU legal texts were therefore read from the Publications Office (Cellar: `https://publications.europa.eu/resource/celex/<CELEX>`), and consolidation dates come from its SPARQL endpoint.
- **echa.europa.eu** answers 403 to automated fetches. ECHA facts below come from search snippets of echa.europa.eu pages and are marked "(snippet)".

eCFR texts were read through the versioned API (`https://www.ecfr.gov/api/versioner/v1/full/2026-10-01/title-NN.xml?part=…`). That URL carries the date of the version, so it is a pinned input on its own.

## 1. What already exists in mzLab, and what this study builds on

| Piece | Where | State | Role here |
|---|---|---|---|
| `.mzsus` v1 suspect index (mass-sorted rows, InChIKey-14, formula, source bits, manifest JSON) | `crates/mzlab-feat/src/suspects.rs`, `docs/agenti/sospetti.md`, builder `tools/genera_sospetti.py` | done; pages.yml builds it from pinned Zenodo inputs into `static/mzfinder/sospetti/` (zlib pieces + `catalogo.json`) | container for the regulatory index (§5.2); the packaging pattern is reused |
| Attributio regulatory lists: `catalog.rs` (28 lists, 6 groups), schema **A1** (20 columns, one row per list × InChIKey first block), B1 `match_campaign` (full InChIKey, first block flagged), B2 `screen_campaign` (5 adducts, 5 ppm, classes A-D/X), B3 `confirm`, P3 `cross_table` | `crates/mzlab-feat/src/attributio/reglists/`, `docs/agenti/attributio.md` | logic done (RG-03, parity 1e-9 on synthetic snapshots); **construction from the sources (RG-04) and the native builders with `Fetch` (RG-06) still to do**; lists today are snapshots loaded from a campaign folder; admin only (`uipAdminOn()`) | the A1 schema is the per-list text format; B1 is the structure match |
| Flow step `annot.regulatory` (algorithms `list_membership`, `suspect_screen`, `ms2_confirm`; requires `hr`) | `crates/mzlab-feat/src/steps/ident_annot.rs` | descriptor only, in `BUCHI` (no native executor) | gets its executor in prompt C |
| `screen.suspects` (NTS PR3) | not in the repository on 2026-10-29 (no file, branch or doc mentions it) | planned | the generic mass screen of any `.mzsus`; the design below fixes its contract for lists |
| Molecule records `molecules/1` (`esterni.cas`, `inchikey`) | `crates/mzlab-core/src/molecules/` | done | not used for lists: a list is not a molecule collection (thousands of rows, many laws per row) |

**Consequence.** The builder is RG-04 plus the list half of RG-06, as already planned in `docs/studi/ATTRIBUTIO.md` §6. Federico's decision Q5 put construction from the sources in native Rust (`mzlab-cli`, a `Fetch` trait with a disk cache, 5 requests/s per host, exponential retry), not in Python. AGENTS.md principle 2 points the same way: no new function is born in Python. The brief's "a tool in `tools/`" therefore becomes a `mzlab-cli` subcommand, plus at most a one-line call in CI (decision D2, §9). Prompt A must be announced to the COO as covering RG-04, so that nobody opens it twice.

## 2. Sources

Column key:
- **Ids** = identifiers carried by the official rows.
- **Redistrib.** = can mzLab ship rows derived from the source:
  - **OJ** = EU legal text, reusable with acknowledgement under Commission Decision 2011/833/EU (only the OJ is authentic).
  - **PD** = US federal work, public domain under 17 USC 105.
- **Status**: in force / adopted, not yet applicable / proposal / draft.

### 2.1 European Union

| List id (catalogue) | Legal basis and version | Entries | Official machine-readable form | Ids | Redistrib. | Cadence | Status |
|---|---|---|---|---|---|---|---|
| `EU_PS_2026_805` | Dir. 2008/105/EC Annex I Part A as amended by **Dir. (EU) 2026/805** of 30.3.2026 (OJ L 20.4.2026, http://data.europa.eu/eli/dir/2026/805/oj); consolidation 2026-05-10; transposition by 21.12.2027 | 1-45 kept, 46-70 new (EE2, E2, E1, 5 neonicotinoids, 3 macrolides, 4 pyrethroids, carbamazepine, diclofenac, ibuprofen, BPA, glyphosate, nicosulfuron, Ag, triclosan, **65 = sum of 25 PFAS** incl. TFA, 70 = sum of pesticides); alachlor, atrazine, CCl4, chlorfenvinphos, simazine, TCBs move to Annex II Part C | XHTML table of the OJ text (Cellar) | name, CAS, EU no. | OJ | revision every ~6 years | in force, transposition pending |
| `EU_PS_2013_39` | Dir. 2008/105/EC Annex I as amended by Dir. 2013/39/EU (the text Member States apply until transposition of 2026/805) | 45 | OJ XHTML | name, CAS, EU no. | OJ | — | applies until 21.12.2027 |
| `EU_WL_2025_439` | Commission Implementing Decision **(EU) 2025/439** of 28.2.2025 (repeals 2022/1307), http://data.europa.eu/eli/dec_impl/2025/439/oj | ~27 (fipronil, clindamycin, ofloxacin, metformin + guanylurea, 4 UV filters, 6PPD + 6PPD-quinone, abamectin, etoxazole, 10 azoles, fluoxetine, propranolol, oxytetracycline, tetracycline) | OJ XHTML | name, CAS, EC, max LOQ | OJ | 2-3 years | in force |
| `EU_GWD` | Dir. 2006/118/EC Annex I (and II) as amended by 2026/805 | Annex I: nitrates, pesticides, sum of PFAS, **sum of 4 PFAS** (PFHxS, PFOS, PFOA, PFNA), carbamazepine, sulfamethoxazole, primidone, nrMs, TCE+PCE | OJ XHTML | names, some CAS | OJ | — | in force, transposition pending; UNVERIFIED: minimum list of Annex II in the 2026 consolidation; groundwater watch list: no implementing act found |
| `EU_DWD_2020_2184` | Dir. (EU) 2020/2184 Annex I Part B + Annex III B.3; no amending act found (consolidation 20201223) | ~30 chemical parameters; "Sum of PFAS" = **20** named PFAS (C4-C13 carboxylic and sulfonic acids); BPA 2.5 µg/L; pesticides 0.1/0.5 µg/L | OJ XHTML | names; PFAS by name only | OJ | — | in force |
| `EU_DWD_WL` | Commission Decision (EU) 2022/679 (OJ 27.4.2022) | 2: 17β-estradiol (50-28-2), nonylphenol (84852-15-3) | OJ XHTML | name, CAS | OJ | — | in force; UNVERIFIED: an update in preparation |
| `EU_POPS_2019_1021` (new id) | Reg. (EU) 2019/1021 Annexes I-IV, consolidation 02019R1021-20260930 (chlorpyrifos added by Del. Reg. (EU) 2026/1423) | Annex I Part A ~30 entries + Annex III/IV; many groups (SCCPs, PBDEs, PCB, PCN, "PFOA/PFHxS, salts and related compounds") | OJ XHTML | name, CAS (often "and others"), EC | OJ | delegated acts after each COP | in force; MCCPs (C(2026)6262, sent to Council 11.9.2026) and LC-PFCAs not yet in the consolidation (UNVERIFIED: OJ publication) |
| `STOCKHOLM_POPS` | Stockholm Convention Annexes A/B/C, listings up to **COP-12** (2025: SC-12/9 chlorpyrifos, SC-12/10 MCCPs, SC-12/12 LC-PFCAs), https://www.pops.int/TheConvention/ThePOPs/AllPOPs/tabid/2509/Default.aspx | A 35, B 2, C 7 | HTML only | names, no CAS | treaty text, no licence stated (UNVERIFIED) | biennial COP | international law; EU application via 2019/1021 |
| `SVHC` | REACH Art. 59, ECHA Candidate List, https://echa.europa.eu/candidate-list-table | **253** entries after 4.2.2026 (snippet; a registry snippet says "273 unique entries": UNVERIFIED) | XLSX/CSV export (snippet); moving to ECHA CHEM, old table until Dec 2026 | name, EC, CAS, reason, date | **ECHA legal notice: no replication "in whole or in substantial part" of ECHA databases without written permission** (snippet) | January and June | in force |
| `REACH_XIV` | Reg. 1907/2006 Annex XIV, consolidation 02006R1907-20260622; last amendment Reg. (EU) 2022/586 | 59 | OJ XHTML | name, EC, CAS | OJ | rare | in force |
| `REACH_XVII` | Reg. 1907/2006 Annex XVII, same consolidation; entries up to 83 (79 PFHxA and related, 2024/2462; 82 PFAS in firefighting foams by substructure, 2025/1988; 83 2,4-DNT, 2026/859); appendices 1-6 list CMRs, etc. | 83 entries + appendices (~1 000 named substances, UNVERIFIED count) | OJ XHTML | name, EC, CAS | OJ | several acts a year | in force; universal PFAS restriction: RAC opinion 2.3.2026, SEAC draft 11.3.2026, final SEAC opinion UNVERIFIED (snippets) → `proposal`, not shipped as law |
| `CLP_ANNEX_VI` | Reg. 1272/2008 Annex VI Table 3, consolidation 02008R1272-20270101 (ATP up to Del. Reg. 2025/1222) | ~4 700 (UNVERIFIED) | OJ XHTML (ECHA's XLSX is unofficial and ECHA-licensed: not used) | index no., name, EC, CAS | OJ | ATP ~yearly | in force |
| `EU_PPP` | Reg. 1107/2009 + Impl. Reg. 540/2011; EU Pesticides Database API `https://api.datalake.sante.service.ec.europa.eu/sante/pesticides/active-substances?format=json&api-version=v3.0` | 4 796 rows, **1 485** unique substances: 421 approved (73 micro-organisms, 28 basic), 980 not approved, 84 pending; CAS on 4 726 rows; no structures | JSON API (paged) | name, CAS, status, dates | **CC BY 4.0** (https://data.europa.eu/data/datasets/pesticides); the portal says the data have "no legal value" | continuous | in force (the citation is 540/2011, the data are the DB) |
| `EU_MRL_396_2005` (new, optional) | Reg. 396/2005 MRLs, bulk download at https://ec.europa.eu/food/plant/pesticides/eu-pesticides-database/start/screen/mrls/download | residue definitions ("sum of … expressed as …") | JSON/CSV/XML | names | CC BY 4.0 | several acts a month | in force; residue definitions ≠ structures: phase 2 |
| `VET_BANNED` / `EU_VET_37_2010_T1` (new) | Reg. (EU) 37/2010 Table 2 (prohibited) and Table 1 (allowed with MRL), consolidation 02010R0037-20260816 | Table 2 ~10; Table 1 ~700 rows (UNVERIFIED) | OJ XHTML only (no official machine-readable form found) | **names only** | OJ | several acts a year | in force |
| `EU_FCM_10_2011` (new) | Reg. (EU) 10/2011 Annex I Table 1, consolidation 02011R0010-20260714, last amendment 2026/2412 (28.10.2026) | ~850 rows (UNVERIFIED) | OJ XHTML | FCM no., CAS, name, SML | OJ | ~yearly | in force |
| `EU_FCM_BISPHENOLS` (new) | Reg. (EU) 2024/3190: BPA and hazardous bisphenols banned in FCM (hazard = CLP Annex VI Cat. 1A/1B: a rule, not a list) | BPA + derivatives named in the act | OJ XHTML | names, CAS | OJ | — | in force; transitions to 20.7.2026 / 20.1.2028 |
| `EU_COSM_II` … `_VI` (new) | Reg. 1223/2009 Annexes II-VI, consolidation 02009R1223-20260518 | II ~1 700, III ~300 (UNVERIFIED) | OJ XHTML; CosIng XLS/CSV (format and licence UNVERIFIED) | name, CAS, EC | OJ (CosIng: UNVERIFIED) | several acts a year | in force |
| `EU_ROHS` (new) | Dir. 2011/65/EU Annex II as amended by Del. Dir. (EU) 2015/863, consolidation 02011L0065-20260701 | 10 (Pb, Hg, Cd, Cr(VI), PBB, PBDE, DEHP, BBP, DBP, DIBP) | OJ XHTML | names | OJ | rare | in force; 6 of 10 are elements/groups |
| `EU_FOOD_CONT_2023_915` (new) | Reg. (EU) 2023/915 maximum levels of contaminants in food (PFAS, PAHs, dioxins, mycotoxins, …) | UNVERIFIED | OJ XHTML | names | OJ | several acts a year | in force; facts not checked in this study |
| `EU_BIOCIDES` (new) | Reg. (EU) 528/2012 Union list of approved actives (ECHA page; last amendments 2026/447, 2026/1165) | ~940 substance × product-type rows (third party, UNVERIFIED) | ECHA page (403) | name, EC, CAS | ECHA terms | continuous | in force; phase 2 |
| `EU_TOYS` | Dir. 2009/48/EC Appendix C (last 2026/192); Reg. (EU) 2025/2509 applies from 1.8.2030 | small | OJ XHTML | name, CAS | OJ | — | in force (2025/2509 adopted, not applicable) |
| Bathing Water Dir. 2006/7/EC | Annex I: intestinal enterococci, *E. coli* only | 0 chemicals | — | — | — | — | **not a chemical list: excluded** |
| `NORMAN_SUSDAT` (cross-reference, not law) | Zenodo 15741861, NORMAN-SLE-S0.0.6.1 (2025-06-25), concept DOI 10.5281/zenodo.2664077; already pinned in `tools/genera_sospetti.py` | ~120 000 substances (InChIKey file 108 671 lines) | CSV 124 MB | Name, CAS_RN, SMILES, StdInChIKey, **MS_Ready_SMILES/InChIKey**, DTXSID, PubChem_CID, `Source` (SLE codes) | CC BY 4.0 | yearly | no regulatory-tag column; EU-related SLE lists: S13 EUCOSMETICS, S28 EUBIOCIDES, S32 REACH2017, S54 EFSAPRI, S86 TATTOOINK, S100 PFASREACH, S115 EIEQSDWLGC; none for the Candidate List, POPs, DWD or the 2025 watch list |

Not studied (UNVERIFIED, phase 2): Industrial Emissions (2010/75/EU, Reg. (EU) 2024/1244 portal list), MSFD 2008/56/EC (refers to WFD and food lists), ECHA CHEM export formats.

### 2.2 United States

| List id | Legal basis and version | Entries | Official machine-readable form | Ids | Redistrib. | Cadence | Status |
|---|---|---|---|---|---|---|---|
| `US_CWA_PP` | CWA §307(a); 40 CFR 423 Appendix A "126 Priority Pollutants" (numbering 001-129, 017/049/050 removed 1981) | 126 | eCFR API XML; EPA PDF (2014) without CAS | names only (eCFR has typos, shifted parentheticals) | PD | static since 1981 | in force; groups: chlordane, Aroclors, toxaphene, asbestos, 13 metals, total cyanide |
| `US_CWA_TOXIC` | 40 CFR 401.15 (44 FR 44502, 30.7.1979; amended 1981) | 65, mostly classes ("Phthalate esters", "PAHs") | eCFR API XML | names | PD | static | in force; few single structures: phase 2 |
| `US_SDWA_MCL` | SDWA §1412; 40 CFR 141.61 (organics, with a CAS column) and 141.62; PFAS NPDWR 89 FR 32744 (26.4.2024), correction 89 FR 49102 | ~90 incl. microbes and radionuclides (UNVERIFIED total); PFAS: PFOA, PFOS 4 ng/L; PFHxS, PFNA, HFPO-DA 10 ng/L; Hazard Index (with PFBS) | eCFR API XML | name, **CAS of the anions** for PFAS (PFOA 45285-51-6, PFOS 45298-90-6, PFHxS 108427-53-8, PFNA 72007-68-2, PFBS 45187-15-3, HFPO-DA 22499-17-6) | PD | — | in force as codified; proposals of 20.5.2026 (extension to 2031, 91 FR 29425; rescission of PFHxS/PFNA/HFPO-DA/HI, 91 FR 29413) **not final** |
| `US_UCMR5` | 40 CFR 141.40, 86 FR 73131 (27.12.2021), 87 FR 3679 | 29 PFAS + lithium | eCFR API XML | name, CAS | PD | 5-year cycle | in force (sampling 2023-2025) |
| `US_UCMR6_PROP` | proposed 1.7.2026, 91 FR 39952, docket EPA-HQ-OW-2023-0469 | 30 (7 ultrashort organofluorines incl. TFA, TFMS, PFPrA, PFPrS, PFEtS, PFMOAA, bistriflimide; 3 phorate/chlorpyrifos metabolites; 13 SVOCs; 7 VOCs) | EPA HTML table | name, CAS | PD | — | **proposal** |
| `EPA_CCL5` | SDWA §1412(b)(1); final 14.11.2022, FR Doc 2022-23963 | 66 chemicals + 3 groups (PFAS structural definition, cyanotoxins, DBPs) | EPA HTML table | name, **CAS, DTXSID** | PD | ~5 years | final (a candidate list: imposes no requirement) |
| `EPA_CCL6_DRAFT` | draft 6.4.2026, 91 FR 17186 | 75 chemicals + 4 groups | EPA HTML | name, CAS | PD | — | **draft** (final expected Nov 2026) |
| `US_CERCLA` | CERCLA §102(a); 40 CFR 302.4; PFOA/PFOS + salts and structural isomers by 89 FR 39124 (eff. 8.7.2024); kept by EPA 17.9.2025; upheld by D.C. Cir. 18.8.2026 (law-firm summaries only: UNVERIFIED) | ~1 390 rows, **684 distinct CAS** | eCFR API XML | name, CAS | PD | rare | in force |
| `US_TRI` | EPCRA §313; 40 CFR 372.65; RY2026 | 810 chemicals + 34 categories; PFAS added each year under NDAA FY2020 | EPA spreadsheet + PDF per year; eCFR XML | name, CAS; categories "N" codes | PD | yearly | in force; total PFAS count UNVERIFIED |
| `US_FDA_189` | 21 CFR 189 subparts C and D | 19 (coumarin, safrole, cyclamate, dulcin, NDGA, thiourea, …) | eCFR API XML | names | PD | rare | in force |
| `US_FDA_SAF` (optional) | FDA "Substances Added to Food" inventory (updated 9.8.2026) | 3 970 | Excel | CAS, name, use, 21 CFR | PD | continuous | inventory, not a prohibition: phase 2 |
| `US_CPSC_1307` | CPSIA §108; 16 CFR 1307.3 (82 FR 49982, 27.10.2017; 83 FR 34764) | 8 phthalates (DEHP, DBP, BBP, DINP, DIBP, DPENP, DHEXP, DCHP) | eCFR API XML | names | PD | rare | in force; lead (15 USC 1278a, 16 CFR 1303) is an element: not screened |
| `US_TSCA_PFAS_8A7` (reference) | 40 CFR 705.3 (structural definition; FR Doc 2023-22094; submission period from 31.1.2027 at the latest, 91 FR 18786) + EPA "Public List of TSCA PFAS for 8(a)(7)" (16.5.2024, ≥1 462 PFAS, 770 active) | 1 462 | XLSX | name, CAS | PD | — | the law is a definition; the list is EPA's non-exhaustive aid |
| `US_TSCA_ACTIVE` (optional) | TSCA §8(b) Inventory, July 2026 | 42 698 active / 86 741 total; CSV: 70 774 public rows with CAS (36 522 active), **18 201 UVCB**, 15 967 confidential rows without CAS | CSV zip 2.5 MB | CAS, name, UVCB flag; no structures | PD for EPA text; CAS RN / CA Index Names possibly ACS-licensed (UNVERIFIED) | 6 months | an inventory of commerce, not a contaminant list: excluded from v1 (D6) |
| `US_PEST_TOL` | FFDCA §408; 40 CFR 180 subpart C | one section per active ingredient | eCFR XML only; no bulk CSV | names (CAS in EPA indexes, HTML/PDF) | PD | continuous | phase 2 |
| `CA_PROP65` | Cal. H&S Code §25249.8; 27 CCR 27001; list of 31.7.2026, https://oehha.ca.gov/proposition-65/proposition-65-list | ~885 rows with CAS (**800 distinct**), ~110 without CAS (groups such as "aflatoxins"); rows repeat per endpoint | Excel, CSV, PDF | name, CAS, endpoint, date | **"Copyright © State of California"; reuse terms UNVERIFIED** (state works are not covered by 17 USC 105) | as needed, at least yearly | in force (state law) |
| `EPA_PFASSTRUCT` (reference, not law) | CompTox list PFASSTRUCT V6 (Jan 2026) | 21 028 | CompTox CSV/SDF export (columns UNVERIFIED) | DTXSID, CAS, name, SMILES, InChIKey | EPA: "free of all copyright restrictions" | ~yearly | not a law; it can serve as cross-reference and for the PFAS definition demo |
| Health Advisories | — | — | — | — | — | — | not enforceable: excluded |

### 2.3 CAS → structure sources (offline build only, never at analysis time)

| Source | File | Size | Gives | Licence |
|---|---|---|---|---|
| NORMAN SusDat S0.0.6.1 | `susdat_2025-06-03-092022.csv` (already pinned by SHA-256 in `tools/genera_sospetti.py`) | 124 MB | CAS → StdInChIKey **and MS-ready InChIKey**, DTXSID, CID | CC BY 4.0 |
| PubChem FTP `Compound/Extras/` | `CID-Identifiers.tsv.gz` (CAS, DTXSID, ChEBI, EC → CID; 1 462 843 CAS rows) | 94 MB | CAS → CID | NCBI "imposes no restrictions on the use or distribution of the data"; depositors may keep rights (https://www.ncbi.nlm.nih.gov/home/about/policies/) |
| PubChem FTP | `CID-Parent.gz` (salt/mixture → parent) | 521 MB | salt → parent | same |
| PubChem PUG REST (build time, through the `Fetch` cache) | `/compound/cid/<list>/property/InChIKey,MolecularFormula,MonoisotopicMass,IsomericSMILES/CSV` | small | CID → key, formula, mass | same; avoids the 6.9 GB `CID-InChI-Key.gz` |
| EPA CompTox / DSSTox | gaftp dumps (2019-2021), figshare 10.23645/epacomptox.5588566 | 200 MB | DTXSID → CAS, InChIKey, MS-ready SMILES | "free of all copyright restrictions" (no CC0 tag found) |
| ChEBI | SDF/flat files | — | ChEBI → structure | CC BY 4.0 |

## 3. Identity: what a row is, and which identifier matches

**Recommendation.** The match key is the **InChIKey first block of the MS-ready structure** (desalted, neutralised, stereo removed): the `inchikey_skeleton` of schema A1. The row also keeps the full InChIKey of the form the law names, and the identifiers as printed in the law (CAS, EC, FCM no., index no., name).

Why this key:
1. LC-MS does not see stereo, the counter-ion or the protonation state. The second and third blocks of the InChIKey carry exactly those (stereo, isotopes, protonation), so the first block is the identity that a feature can support. B1 already works this way: a full-key match is "InChIKey", a first-block match is "first block", flagged.
2. CAS is not a structure. One CAS can mean a mixture, a salt or a UVCB, and one structure has several CAS. CAS is therefore an input identifier for resolution, never a match key. EC numbers behave the same way.
3. Name and formula are never a match (B1 already refuses them). A formula or a mass is how a feature reaches the candidates (`screen.suspects`), and every structure with that formula is shown as an isomer group (§6).

Resolution order for a listed entry (each row records it in `resolved_by`):
1. Structure given by the law itself (rare, e.g. a structural formula in an annex): `law`.
2. CAS or EC → NORMAN SusDat (pinned) → StdInChIKey and MS-ready InChIKey: `susdat`.
3. CAS → PubChem CID (`CID-Identifiers`) → parent CID (`CID-Parent`) → InChIKey, formula and mass (PUG REST through the cache): `pubchem`.
4. DTXSID given by the list (CCL 5) → DSSTox: `dsstox`.
5. A curated row of `normative/overrides.tsv` (reviewed in a PR, with a reason): `curated`.
6. Disagreement between sources (one CAS → two different skeletons), or nothing found: `unresolved`. The entry stays in the list file and in the coverage count, never in the screening index.

How each awkward case is handled:

| Case | Handling | Example |
|---|---|---|
| **Salt, anion, ester-as-listed** | resolved to the neutral parent skeleton; the row keeps the listed CAS and `form` = `salt`, `anion` or `parent`; both acid and anion CAS of one law collapse into one skeleton | SDWA lists PFOA as anion 45285-51-6 and CERCLA as acid 335-67-1: same skeleton, two listed forms |
| **Named group whose members the law enumerates** | one row per member, `group_entry` = the entry text | DWD "Sum of PFAS" (20 named); 2026/805 entry 65 (25 PFAS); UCMR 5 (29) |
| **Open group or structural definition** | no screening rows; the entry is counted under `groups` in the coverage, its text is shown in the report ("this list also contains entries defined by structure or as groups that mzLab does not screen by mass") | REACH XVII entry 82 (≥1 CF3/CF2), TSCA 705.3 PFAS, CCL 5 PFAS group, "PFHxA, its salts and related substances", "Chlorinated phenols"; a substructure test of a candidate structure against a definition is phase 2 (it needs a SMARTS-like matcher in `mzlab-mol`) |
| **UVCB, technical mixtures, reaction masses** | no screening rows unless the law names constituents; counted as `uvcb` | SCCPs "85535-84-8 and others", technical endosulfan, toxaphene, Aroclors (the law names the mixture, not congeners) |
| **Elements, inorganic ions, metals** | counted as `inorganic`, not screened (no LC-ESI organic identity) | Pb, Cd, Hg, Cr(VI), nitrate, cyanide, chlorate |
| **Isotopologues** | not listed by any law; an isotopically labelled internal standard has the same first block, so a structure match on a labelled standard is flagged by the B1 rule, and a mass match cannot happen (different mass) | — |
| **Stereoisomers listed separately** | one skeleton, several listed rows; the report shows them all | permethrin isomers |
| **Missing structure** (name only: 37/2010, CWA PP) | resolution by curated name → CAS (step 5), never by automatic name matching; otherwise `unresolved` | veterinary residues |

Every list publishes its coverage in the provenance (§5.3): entries, screenable rows, groups, uvcb, inorganic, unresolved. The report repeats it, so a reader knows what "screened against" covered.

## 4. One combined index, many laws per row

**Recommendation: one combined package, built from per-list text files.**
- **Source of truth, one file per list.** Each list is a CSV in schema A1 plus five appended columns: `entry_ref` (article/annex/table/entry no.), `ec`, `dtxsid`, `form`, `status`. `read_list` reads columns by name and ignores the extra ones, so Attributio's parity does not move. One file per list keeps a review diff readable: an update PR shows exactly which entries a new act added or removed.
- **Screening index.** One `.mzsus` with a `listings` extension holds the union of all lists: one row per skeleton (§5.2). A compound on many laws is **one row with many memberships**. Each membership is (list, entry, form, group_entry), so the report can list every law that names the compound without repeating the mass search.
- **The user chooses lists** (all in force by default; proposals, drafts and references opt-in) through a list mask, as `LibSet` does for libraries. The mask enters `params_sha256`, so `prov/1` knows which lists ran.

Rejected options:
- One `.mzsus` per law: N searches per feature, and the "on 7 lists" view would have to be stitched in JS (logic outside Rust).
- Putting lists inside the general SusDat index: it would mix "a law names this" with "someone suspects this", and it would grow mzFinder's download.

**Tags per list (in the manifest, not per row):** `jurisdiction` (`EU`, `US`, `US-CA`, `INT`), `instrument` (`Directive (EU) 2026/805`), `citation` (`Dir. 2008/105/EC Annex I Part A as amended by Dir. (EU) 2026/805`), `annex`, `status` (`in_force`, `adopted_not_applicable`, `proposal`, `draft`, `reference`), `in_force_from`, `applies_from`, `version_date` (consolidation or list date), `source_url`, `retrieved`, `sha256` (of the bytes read), `licence`, `attribution`, `coverage`. The catalogue (`catalog.rs`) gains a group `US federal` and a group `US state`, and the new ids of §2. Thailand and Italy stay as they are; they are campaign lists, not part of this package.

## 5. Build, data release and the browser

### 5.1 The builder (prompt A)
`mzlab-cli normative build --cache DIR --out normative/` (native Rust, `Fetch` trait with a disk cache, 5 requests/s per host, exponential retry, a fake `Fetch` in tests: RG-06's design). It has two parts:
- **One reader per source shape.** These are the RG-04 names: `eu_water`, `eu_chem`, `eu_ppp`, `voci`, `svhc`, `international`, plus `us_ecfr` and `us_epa_html` (new). They read Cellar XHTML, eCFR XML, EPA HTML tables and the Pesticides JSON. Each reader turns its source into entries.
- **`resolve`** (§3 order) writes the A1+ CSV of each list and `normative/provenance.json`.

Inputs are pinned:
- every fetched URL is stored in the cache with its SHA-256, and `provenance.json` records it;
- eCFR URLs carry the version date;
- Cellar URLs carry the consolidated CELEX (e.g. `02008L0105-20260510`).

With a warm cache the build is deterministic (no clock: `retrieved` is the date stored with the cached file), and the same cache gives the same bytes. The whole build never runs at analysis time.

### 5.2 `.mzsus` `listings` extension (no new reader version)
The v1 Rust reader ignores bytes after the formula blob, and the header field `flags` is 0 today. The regulatory index sets **flag bit 0 = LISTINGS** and appends, each section padded to 8:

| Section | Content |
|---|---|
| ext head (16 B) | u32 `nl` lists, u32 `m` memberships, u32 `ne` entries, u32 bytes of the entry blob |
| full keys | `n` × 27 B, the full InChIKey of the row's representative listed form (spaces when none) |
| names | `n+1` × u32 offsets + UTF-8 blob: the display name |
| membership offsets | `n+1` × u32 into memberships |
| memberships | `m` × (u16 list index, u8 `form`, u8 bits: 1 group member, 2 isomer stated, 4 CAS as listed is an anion/salt; u32 entry index) |
| entries | `ne+1` × u32 offsets + blob of one compact JSON object per entry: `{"ref":"Annex I Part A, no. 65","name":"…","cas":"…","ec":"…","attr":"AA-EQS 0.0044 µg/L"}` |

- **Manifest.** The manifest JSON gains `normative/1`: lists with their tags (§4) and coverage; `sources` keeps its v1 meaning.
- **Source bits.** The u8 `sources` column gets bit 8 = `SRC_NORMATIVE`.
- **Readers.**
  - `reader_min` stays 1. mzFinder and the Python reference keep reading the file as a plain suspect list.
  - The Rust reader gains `Suspects::listings()` (`None` without the flag).
  - The Python reference reader `tools/sospetti_riferimento.py` gains the same, for the parity test.

### 5.3 Where the data live and how the site and the app get them (prompt B)
**Recommendation: the per-list CSVs and `provenance.json` are committed in the public repository under `normative/`.** The binary package is derived from them in CI.
- **Why commit them.** Text built from EU legal texts (OJ, Decision 2011/833/EU), US federal sources (public domain), CC BY 4.0 sources (EU Pesticides DB, NORMAN) and NCBI data is redistributable with attribution. The committed text makes every legal change reviewable as a diff, and it lets `pages.yml` build the package with no network and no cache-miss failure at deploy.
- **Size.** About 15 000 rows × ~300 B ≈ 4-5 MB of CSV without TSCA. The package after zlib is about 1.5 MB (estimate, to be measured in prompt B).
- **Not committed (bring your own file):**
  - the **ECHA Candidate List**: ECHA's terms forbid replicating its databases in whole or substantial part without permission;
  - **Prop 65** (terms UNVERIFIED).
  These are offered as "Import an official file": the user downloads ECHA's XLSX/CSV or OEHHA's CSV, and the same Rust readers (in the `.wasm` and in the app) build a private list on the user's machine. It never leaves the computer and is stored in OPFS (D4, D5).
- **CAS numbers.** CAS RNs appear as printed in the law (a citation of the act), and no CAS Index Names are copied. ACS terms on CAS RN redistribution are UNVERIFIED: see the risks (§8).
- **Site.** `pages.yml` runs `mzlab-cli normative pack normative/ site/static/normative/` (no network): zlib pieces of 4 MiB with SHA-256 plus `catalogo.json` (`normative/1`), the same pattern as `static/mzfinder/sospetti/`. The page downloads the package **on first use** of the Regulatory lists (same origin: CSP `connect-src 'self'`) into the Cache Storage `mzlab-libs`. After that it works offline. The Library panel shows the version date of every list and a "Check for updates" control (compares `catalogo.json`).
- **The `.wasm` caps never rise.** Lists are data, not code. Only the `listings` reader and the screening code enter a `.wasm`; prompt C measures before and after and uses a separate module if the host is over its cap.
- **App (desktop and CLI).**
  - The same package. The desktop app fetches it from the site URL on first use into its data folder, or reads one the user picked.
  - The CLI takes `--set screen.lists=PATH` (a package folder or a `.mzsus`).
  - Parity: the same package bytes and the same feature table give byte-identical results in the `.wasm` and native (prompt C test).

### 5.4 The update routine (prompt D)
- **Monthly check.** A scheduled agent run (or a person) runs `mzlab-cli normative check`. It compares what is pinned with what is current:
  - the consolidated CELEX and its date (SPARQL);
  - the eCFR version date of every part;
  - the Pesticides API's last-modified date;
  - the EPA page dates;
  - the NORMAN Zenodo version.
  It prints the lists that changed.
- **Event triggers** besides the monthly check: Candidate List (January, June), new Watch List decisions, POPs delegated acts after a COP, UCMR 6 or CCL 6 final, PFAS NPDWR final rules, a new Prop 65 list.
- **When something changed:**
  1. `normative build` with the new pins;
  2. a PR whose body is the generated summary (per list: entries added and removed, the new coverage);
  3. a human review of `overrides.tsv` changes;
  4. merge by the COO.
- **Integrity.** A list whose source vanished or failed to parse keeps its last good version with `stale: true`. It is never silently emptied.

## 6. Matching in Rust (prompt C)

Two paths. Both keep the evidence and give no verdict.
1. **Structure path (`annot.regulatory` / `list_membership`).** A feature already has a candidate structure from `spectra.match` (library hit) or `annot.level`, carrying its InChIKey. B1 `match_campaign` compares it with the memberships:
   - same full key → match basis `InChIKey`;
   - same first block only → `first block` (stereo, charge or isotope may differ);
   - name or formula alone → never.
   This is the existing B1 rule; prompt C gives the step its native executor, reading the package.
2. **Mass path (`screen.suspects`, NTS PR3).** Feature m/z and polarity → neutral mass per adduct. The adducts are B2's five ([M+H]+, [M+NH4]+, [M+Na]+, [M-H]-, [M+Cl]-) or the user's adduct list. Each neutral mass gets `Suspects::search(mass, ppm)` (default 5 ppm). Each hit is grouped **by formula**: "C8HF15O2: 3 listed structures (PFOA, its branched isomers…)". The listed structures that share the formula are an explicit **isomer group** in the output, never a single name. Each hit also carries Δppm, the adduct, the memberships and the coverage note.
   - Output: `screen/1` rows (`feature`, `adduct`, `delta_ppm`, `formula`, `candidates[]` with `skeleton`, `inchikey`, `name`, `memberships[]`). Every hit is a candidate at Level 5 (mass and formula only, Schymanski), and Level 4 if the formula is also supported by `formula.ms1`.
   - Requires `hr`, as `annot.regulatory` does (principle 1, unit-resolution world: an m/z is a candidate; at unit resolution every nominal mass hits dozens of listed substances, so the step is not offered on LR data).

Parameters are all in the `step/1` descriptor with `--set`:

| Parameter | Value |
|---|---|
| `ppm` | 5 |
| `adducts` | from the list type `adducts` |
| `lists` | the mask: `in_force` by default |
| `include` | `proposal` / `draft` / `reference`, off by default |
| `window` | Da, the existing `annot.regulatory` pair |

## 7. Wording (English, the only language of Attributio panels; the NTS UI keys go to `lang/it.js` and `lang/en.js`)

Principle: mzLab says what a candidate matches, never what a sample contains or whether it complies.

| Where | Text |
|---|---|
| Column header | "Listed in" |
| Cell (mass path) | "Candidate formula C8HF15O2 matches 3 listed substances" |
| Cell (structure path) | "Candidate structure matches a listed substance (InChIKey)" / "(first block: stereo, charge or isotope may differ)" |
| Membership line | "Directive (EU) 2026/805, Annex I Part A, entry 65 (sum of 25 PFAS), version of 2026-05-10" |
| Report header | "Screened against: EU priority substances, Directive 2008/105/EC Annex I as amended by Directive (EU) 2026/805 (consolidated 2026-05-10, retrieved 2026-10-29, SHA-256 1a2b…); … 14 lists in force, 0 proposals." |
| Coverage line | "This list has 70 entries: 61 screened by structure, 6 groups defined by structure or as mixtures, 3 inorganic; not screened by mass." |
| Disclaimer (report and panel footer) | "A match means that a candidate for this feature has the mass, formula or structure of a substance named in a regulatory list. It is not an identification, it says nothing about concentration, and it is not an assessment of compliance with any law. Lists are reproduced from the official texts named above; only the official publications are authentic. Confirm candidates with reference standards before any regulatory use." |

Words never used: "detected", "present", "non-compliant", "exceeds", "violation", "contaminated". The legal limit text of an entry (e.g. "AA-EQS 0.0044 µg/L") may be shown as the law's text. It is never compared with a measured value: mzLab has no concentration in this step (D8).

## 8. Risks

| Risk | Mitigation |
|---|---|
| ECHA terms (Candidate List) | ship no ECHA-derived rows; import of the user's own file; ask ECHA for permission (D4) |
| ACS rights on CAS RN | CAS printed as in the law, no CA Index Names; NORMAN SusDat redistributes CAS under CC BY 4.0, and common practice is similar; still UNVERIFIED → D9 |
| Prop 65 terms | import of the user's own file until OEHHA terms are read (D5) |
| Wrong CAS → structure (salts, mixtures, retired CAS) | the §3 order, disagreements → `unresolved`, curated overrides reviewed in PR, coverage printed |
| A law changes and mzLab lags | version and retrieval date in every report line; the monthly check; `stale` flag |
| Users read a match as a finding | wording of §7, Level 5/4 labels, HR only, isomer groups explicit |
| Cellar / EPA page layout changes | readers fail loudly with the URL; the last good version stays |
| Size growth (TSCA, MRLs, Food Contaminants) | optional lists, measured in prompt B; lists are data and never touch `.wasm` caps |
| Overlap with RG-04 / RG-06 | prompt A *is* RG-04 + list part of RG-06; the COO marks them in ATTRIBUTIO.md |

**Could not verify:**
- any Candidate List update after 4.2.2026, and 253 vs 273;
- the final SEAC opinion on universal PFAS;
- OJ publication of the MCCP and LC-PFCA POPs acts;
- Annex II GWD minima and a groundwater watch list;
- the CosIng format and licence;
- an official machine-readable 37/2010 or 10/2011;
- the counts of CLP Annex VI, 10/2011, biocides and REACH XVII appendices;
- ECHA CHEM export formats;
- the total SDWA MCL count;
- UCMR 6 dates;
- the D.C. Circuit CERCLA ruling (secondary sources only);
- the CompTox export columns;
- a CC0 tag on DSSTox dumps;
- OEHHA reuse terms;
- ACS terms on CAS RN lists;
- the Stockholm Convention entry-into-force dates;
- IED, MSFD and 2023/915 details.

## 9. Decisions only Federico can take

| # | Question | Recommendation |
|---|---|---|
| D1 | Commit the per-list CSVs built from public law texts (EU OJ, US federal, CC BY sources) to the **public** repository under `normative/`, with the package built by CI? | **Yes** (reviewable diffs, deterministic deploy; no lab data) |
| D2 | Build the lists with native Rust in `mzlab-cli` (RG-04 / RG-06 as already decided in Q5) instead of a Python script in `tools/`? | **Yes** |
| D3 | Regulatory screening visible to every user in the NTS area (not only behind `uipAdminOn()`), HR data only? | **Yes** |
| D4 | ECHA Candidate List: write to ECHA for permission to redistribute; meanwhile offer "Import ECHA's official file" (built locally, never uploaded)? | **Yes** to both |
| D5 | Prop 65: same "import the official file" route until OEHHA's conditions of use are read? | **Yes** |
| D6 | Leave the TSCA Inventory (a commerce inventory, 18 201 UVCB, CAS-name licensing unclear) out of v1? | **Yes** (phase 2 as an optional reference list) |
| D7 | Include proposals and drafts (UCMR 6, CCL 6, universal PFAS restriction, the 2026 PFAS NPDWR rescission) as opt-in lists tagged `proposal`/`draft`, off by default? | **Yes** |
| D8 | Show the legal limit text of an entry (never compared with any measurement)? | **Yes**, as quoted law text |
| D9 | Accept shipping CAS RN as printed in the laws, or ask ACS / drop CAS from shipped files (keeping it only in the build cache)? | **Ship as printed in the law**, and drop if ACS objects |

## 10. Build prompts (order)
In `docs/studi/prompt_liste_normative/`:
1. **A**: the builder.
2. **B**: the packaging and data release; it needs A.
3. **C**: the matching in `screen.suspects` and `annot.regulatory`, the Library panel and the report wording. It needs B's package format, so it can start after A with a fixture.
4. **D**: the update routine; it needs B.

A can start now. C should be coordinated with the NTS PR3 chat if that chat starts first: C *defines* the list contract of `screen.suspects` and must not run in parallel with a chat that owns the same step file.
