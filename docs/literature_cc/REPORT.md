# Report T8 — Scientific Literature on Reaction Sites and Isomer Pairs

- **Track**: T8 (Literature on reaction sites and isomer pairs with RP-LC retention time)
- **Compilation date**: 16 October 2026
- **Author**: Sub-agent A7 (mzLab / Gemini)
- **Generated files**:
  - `docs/literature_cc/experimental_sites.csv` (39 sites and intermediates across 10 molecules)
  - `docs/literature_cc/isomer_pairs_rt.csv` (12 real pairs of positional isomers with RTs measured by RP-LC)
  - `docs/literature_cc/REPORT.md` (this summary and validation document)

---

## 1. Objectives and Methodology

This report documents the peer-reviewed experimental evidence on:
1. **Primary sites of hydroxyl radical attack ($ullet	ext{OH}$)** or hydroxylation in advanced oxidation processes (AOPs: $	ext{TiO}_2$ photocatalysis, $	ext{UV/H}_2	ext{O}_2$, Fenton, photo-Fenton, radiolysis) for the **10 new molecules of §6**:
   - *Diclofenac*
   - *Sulfamethoxazole*
   - *Atrazine*
   - *Bisphenol A*
   - *Caffeine*
   - *Triclosan*
   - *Ibuprofen*
   - *Metoprolol*
   - *Benzophenone-3*
   - *Imidacloprid*
2. **Real pairs of positional isomers of transformation products (TPs)** separated chromatographically by reversed phase (RP-LC: high-performance C18 columns) with exact quantitative retention times ($t_R$) reported in the main text or in open supplementary data.
3. **Rigorous verification of Digital Object Identifiers (DOIs)**: every cited DOI was verified in real time by direct HTTP query to the Crossref API (`https://api.crossref.org/works/<doi>`), checking the exact title, publication year and journal, and excluding any hypothetical or unsubstantiated data.

---

## 2. Summary of the Experimental Evidence for the 10 Molecules (§6)

### 2.1 Diclofenac
- **C5 site (phenylacetic ring, *para* to the secondary amino group)**: main site of electrophilic addition by $ullet	ext{OH}$, leading to **5-hydroxydiclofenac** ($m/z\ 312\ [	ext{M+H}]^+$, $	ext{C}_{14}	ext{H}_{11}	ext{Cl}_2	ext{NO}_3$). Confirmed with a pure reference standard and LC-MS/MS chromatography (*Pérez-Estrada et al., 2005*, doi:10.1021/es050794n; *Calza et al., 2006*, doi:10.1016/j.apcatb.2006.04.021). Confidence: **high**.
- **C4' site (2,6-dichlorophenyl ring, *para* to NH)**: competing attack generating **4'-hydroxydiclofenac**, also identified with an authentic standard. Confidence: **high**.
- **C3' site (dichlorinated ring, *meta* to NH)**: secondary hydroxylation detected by HRMS/MS2 (**3'-hydroxydiclofenac**). Confidence: **medium**.
- **Diphenylamine bridge**: dehydrogenation/oxidation of the amine bridge with intramolecular cyclization to carbazole derivatives or formation of reactive quinone imines. Confidence: **medium**.

### 2.2 Sulfamethoxazole (SMX)
- **Anilinic ring (positions C3 and C5, *ortho* to $-	ext{NH}_2$)**: radical electrophilic addition favored by the high electron density conferred by the primary amino group, producing **3-hydroxysulfamethoxazole** ($m/z\ 270\ [	ext{M+H}]^+$, $	ext{C}_{10}	ext{H}_{11}	ext{N}_3	ext{O}_4	ext{S}$) (*Trovó et al., 2009*, doi:10.1016/j.watres.2009.04.006). Confidence: **medium**.
- **Primary amino nitrogen ($-	ext{NH}_2$)**: progressive oxidation by the $ullet	ext{OH}$ radical to hydroxylamine ($-	ext{NHOH}$) and then to the nitroso and nitro derivative (**4-nitro-sulfamethoxazole**, $m/z\ 284$). Confidence: **high** (authentic standard).
- **Sulfonamide bond $	ext{S–N}$**: homolytic/hydrolytic cleavage induced by radical attack, releasing **sulfanilic acid** and **3-amino-5-methylisoxazole**. Confidence: **high**.
- **Isoxazole ring (C4)**: attack on the double bond followed by opening of the heterocyclic ring. Confidence: **medium**.

### 2.3 Atrazine
- **$N$-ethyl alkyl chain**: hydrogen abstraction at the aliphatic $lpha$ carbon ($-	ext{NH}–	ext{CH}_2	ext{CH}_3$) followed by oxidative de-ethylation with elimination of acetaldehyde; generates **deethylatrazine** (DEA, $	ext{C}_6	ext{H}_{10}	ext{ClN}_5$) (*Konstantinou et al., 2001*, doi:10.1021/es001271c). Confidence: **high**.
- **$N$-isopropyl alkyl chain**: analogous oxidative de-isopropylation with release of acetone, forming **deisopropylatrazine** (DIA, $	ext{C}_5	ext{H}_8	ext{ClN}_5$). Confidence: **high**.
- **$	ext{C–Cl}$ site on the $s$-triazine core**: dechlorination with nucleophilic/homolytic hydroxylation to **hydroxyatrazine** (HA, $	ext{C}_8	ext{H}_{15}	ext{N}_5	ext{O}$). Confidence: **high**.
- **Stable hydroxylated aliphatic intermediates**: detection of **1-hydroxyethylatrazine** (CNIT, $	ext{C}_8	ext{H}_{14}	ext{ClN}_5	ext{O}$) by UHPLC-HRMS Orbitrap (*Molecules 2022*, doi:10.3390/molecules27249021). Confidence: **medium**.

### 2.4 Bisphenol A (BPA)
- **Phenolic ring (positions C3 and C5, *ortho* to the hydroxyl $-	ext{OH}$)**: preferential electrophilic addition of $ullet	ext{OH}$ with formation of **3-hydroxybisphenol A** (catechol-BPA, $	ext{C}_{15}	ext{H}_{16}	ext{O}_3$), readily convertible to the corresponding genotoxic bisphenol A-3,4-quinone (*Applied Catalysis B 2014*, doi:10.1016/j.apcatb.2014.05.007). Confidence: **high**.
- **Double hydroxylation**: generation of **3,3'-dihydroxybisphenol A** ($	ext{C}_{15}	ext{H}_{16}	ext{O}_4$, $m/z\ 259\ [	ext{M-H}]^-$). Confidence: **medium**.
- **Central isopropylidene bridge**: radical oxidative cleavage of the quaternary bridge releasing **4-hydroxyacetophenone** ($	ext{C}_8	ext{H}_8	ext{O}_2$), **4-isopropenylphenol** and **hydroquinone** ($	ext{C}_6	ext{H}_6	ext{O}_2$). Confidence: **high**.

### 2.5 Caffeine
- **C8 carbon of the imidazole ring**: site of very high electrophilic reactivity; addition of $ullet	ext{OH}$ to the $	ext{C8=N9}$ double bond forms the radical adduct intermediate that rapidly evolves into **1,3,7-trimethyluric acid** (8-hydroxycaffeine, TMU, $	ext{C}_8	ext{H}_{10}	ext{N}_4	ext{O}_3$) (*Dalmázio et al., 2005*, doi:10.1021/es047985v). Confidence: **high**.
- **$N1$, $N3$, $N7$ methyl groups**: competitive oxidative demethylation with formation of the three isomeric dimethylxanthines:
  - $N1$-demethylation $	o$ **theobromine** (3,7-dimethylxanthine, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$)
  - $N3$-demethylation $	o$ **paraxanthine** (1,7-dimethylxanthine, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$)
  - $N7$-demethylation $	o$ **theophylline** (1,3-dimethylxanthine, $	ext{C}_7	ext{H}_8	ext{N}_4	ext{O}_2$).
  All confirmed with authentic standards in photocatalysis and biochemical studies. Confidence: **high**.
- **Imidazole ring opening**: degradation to **6-amino-5-[formylmethylamino]-1,3-dimethyluracil** ($	ext{C}_7	ext{H}_{10}	ext{N}_4	ext{O}_3$). Confidence: **medium**.

### 2.6 Triclosan
- **Phenolic ring (position C6, *ortho* to the hydroxyl and *para* to the ether)**: aromatic hydroxylation to **4-chloro-2-(2,4-dichlorophenoxy)benzene-1,5-diol** (hydroxy-triclosan, $	ext{C}_{12}	ext{H}_7	ext{Cl}_3	ext{O}_3$) (*Schröder et al., 2020*, doi:10.3390/catal10121468). Confidence: **medium**.
- **Diphenyl ether bridge**: radical/photolytic cleavage of the ether releasing **2,4-dichlorophenol** (2,4-DCP, $	ext{C}_6	ext{H}_4	ext{Cl}_2	ext{O}$) and **4-chlorocatechol** ($	ext{C}_6	ext{H}_5	ext{ClO}_2$) (*Latch et al., 2005*, doi:10.1897/04-243r.1). Confidence: **high**.
- **Photochemical intramolecular cyclization**: condensation with loss of $	ext{HCl}$ to **2,8-dichlorodibenzo-$p$-dioxin** (2,8-DCDD, $	ext{C}_{12}	ext{H}_6	ext{Cl}_2	ext{O}_2$), confirmed with a pure standard. Confidence: **high**.

### 2.7 Ibuprofen
- **Tertiary C2 carbon of the isobutyl chain**: favored abstraction of the tertiary hydrogen (lowest $	ext{C–H}$ bond energy, $\sim 390	ext{ kJ/mol}$) with formation of **2-hydroxyibuprofen** ($	ext{C}_{13}	ext{H}_{18}	ext{O}_3$) (*Chemosphere 2019*, doi:10.1016/j.chemosphere.2018.10.053; *J. Xenobiotics 2024*, doi:10.3390/jox15010005). Confidence: **high**.
- **Secondary benzylic C1 carbon**: abstraction of the resonance-stabilized benzylic hydrogen with formation of **1-hydroxyibuprofen** ($	ext{C}_{13}	ext{H}_{18}	ext{O}_3$). Confidence: **high**.
- **Terminal primary C3 carbon**: hydroxylation and subsequent oxidation to **carboxyibuprofen** ($	ext{C}_{13}	ext{H}_{16}	ext{O}_4$). Confidence: **high**.
- **Propionic acid chain**: photo-oxidative decarboxylation producing **4-isobutylacetophenone** (4-IBAP, $	ext{C}_{12}	ext{H}_{16}	ext{O}$). Confidence: **high**.

### 2.8 Metoprolol
- **Benzylic $	ext{C}_lpha$ position of the 2-methoxyethyl chain**: main radical abstraction with hydroxylation to **$lpha$-hydroxymetoprolol** ($	ext{C}_{15}	ext{H}_{25}	ext{NO}_4$, $m/z\ 284\ [	ext{M+H}]^+$) (*Benner & Ternes, 2009*, doi:10.1021/es900280e; *J. Hazard. Mater. 2010*, doi:10.1016/j.jhazmat.2010.03.013). Confidence: **high**.
- **Aromatic ring (*ortho* to the ether oxygen)**: electrophilic addition on the activated phenyl ring producing phenolic isomers (**ar-hydroxymetoprolol**, $	ext{C}_{15}	ext{H}_{25}	ext{NO}_4$). Confidence: **medium**.
- **Terminal methoxy group**: oxidative $O$-demethylation with formation of **$O$-demethylmetoprolol** ($	ext{C}_{14}	ext{H}_{23}	ext{NO}_3$). Confidence: **high**.
- **Secondary amino group**: oxidative de-isopropylation of the isopropylamine chain to **deisopropylmetoprolol** ($	ext{C}_{12}	ext{H}_{19}	ext{NO}_3$). Confidence: **medium**.

### 2.9 Benzophenone-3 (Oxybenzone, BP-3)
- **C4 methoxy group**: radical $O$-demethylation producing **2,4-dihydroxybenzophenone** (Benzophenone-1, BP-1, $	ext{C}_{13}	ext{H}_{10}	ext{O}_3$) (*J. Environ. Manage. 2016*, doi:10.1016/j.jenvman.2015.11.047; *J. Hazard. Mater. 2021*, doi:10.1016/j.jhazmat.2020.123591). Confidence: **high**.
- **Unsubstituted phenyl ring (position C2', *ortho*)**: aromatic hydroxylation to **2,2'-dihydroxy-4-methoxybenzophenone** (Benzophenone-8, BP-8, $	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidence: **high**.
- **Unsubstituted phenyl ring (position C4', *para*)**: hydroxylation to **2,4'-dihydroxy-4-methoxybenzophenone** ($	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidence: **medium**.
- **Substituted ring (position C5, *ortho* to the phenolic hydroxyl and *meta* to the carbonyl)**: electrophilic addition of $ullet	ext{OH}$ to **2,5-dihydroxy-4-methoxybenzophenone** ($	ext{C}_{14}	ext{H}_{12}	ext{O}_4$). Confidence: **medium**.

### 2.10 Imidacloprid
- **Imidazolidine ring C5**: direct aliphatic hydroxylation producing **5-hydroxyimidacloprid** ($	ext{C}_9	ext{H}_{10}	ext{ClN}_5	ext{O}_3$) (*Kitsiou et al., 2009*, doi:10.1016/j.apcatb.2008.07.018). Confidence: **high** (identified with standard, LC-MS/MS and NMR).
- **Imidazolidine ring dehydration**: rapid evolution of the 5-hydroxy derivative to **imidacloprid-olefin** ($	ext{C}_9	ext{H}_8	ext{ClN}_5	ext{O}_2$). Confidence: **high**.
- **Nitroimine group**: hydrolysis and radical oxidation of the $=	ext{N–NO}_2$ substituent to a cyclic urea (**imidacloprid-urea**, $	ext{C}_9	ext{H}_{10}	ext{ClN}_3	ext{O}$). Confidence: **high**.
- **Methylene bridge $-	ext{CH}_2–$**: oxidative cleavage between the pyridine core and the imidazolidine ring with formation of **6-chloronicotinic acid** (6-CNA, $	ext{C}_6	ext{H}_4	ext{ClNO}_2$). Confidence: **high**.

---

## 3. Cross-Verification Table of the Cited DOIs (Crossref HTTP API)

All 22 DOIs entered in the T8 track files were verified deterministically by querying the Crossref database directly (`https://api.crossref.org/works/<doi>`).

| DOI | Year | Journal / Container | Verified Official Title | Check Result |
|---|---|---|---|---|
| `10.1021/es050794n` | 2005 | *Environmental Science &amp; Technology* | Photo-Fenton Degradation of Diclofenac:  Identification of Main Intermediates and Degradation Pathway | **Compliant** |
| `10.1016/j.apcatb.2006.04.021` | 2006 | *Applied Catalysis B: Environmental* | Photocatalytic degradation study of diclofenac over aqueous TiO2 suspensions | **Compliant** |
| `10.1016/j.watres.2009.04.006` | 2009 | *Water Research* | Degradation of sulfamethoxazole in water by solar photo-Fenton. Chemical and toxicological evaluation | **Compliant** |
| `10.1021/es001271c` | 2001 | *Environmental Science &amp; Technology* | Photocatalytic Degradation of Selected s-Triazine Herbicides and Organophosphorus Insecticides over Aqueous TiO<sub>2</sub> Suspensions | **Compliant** |
| `10.3390/molecules27249021` | 2022 | *Molecules* | Real-Time Monitoring of the Atrazine Degradation by Liquid Chromatography and High-Resolution Mass Spectrometry: Effect of Fenton Process and Ultrasound Treatment | **Compliant** |
| `10.1016/j.apcatb.2014.05.007` | 2014 | *Applied Catalysis B: Environmental* | Formation of genotoxic quinones during bisphenol A degradation by TiO2 photocatalysis and UV photolysis: A comparative study | **Compliant** |
| `10.1021/es047985v` | 2005 | *Environmental Science &amp; Technology* | Advanced Oxidation of Caffeine in Water:  On-Line and Real-Time Monitoring by Electrospray Ionization Mass Spectrometry | **Compliant** |
| `10.3390/catal10121468` | 2020 | *Catalysts* | Photocatalytic Transformation of Triclosan. Reaction Products and Kinetics | **Compliant** |
| `10.1897/04-243r.1` | 2005 | *Environmental Toxicology and Chemistry* | Aqueous photochemistry of triclosan: Formation of 2,4-dichlorophenol, 2,8-dichlorodibenzo-<i>p</i>-dioxin, and oligomerization products | **Compliant** |
| `10.1016/j.chemosphere.2018.10.053` | 2019 | *Chemosphere* | Photocatalytic degradation of ibuprofen in water using TiO2/UV and g-C3N4/visible light: Study of intermediate degradation products by liquid chromatography coupled to high-resolution mass spectrometry | **Compliant** |
| `10.1021/es900280e` | 2009 | *Environmental Science &amp; Technology* | Ozonation of Metoprolol: Elucidation of Oxidation Pathways and Major Oxidation Products | **Compliant** |
| `10.1016/j.jhazmat.2010.03.013` | 2010 | *Journal of Hazardous Materials* | UV-C radiation based methods for aqueous metoprolol elimination | **Compliant** |
| `10.1016/j.jenvman.2015.11.047` | 2016 | *Journal of Environmental Management* | Heterogeneous photocatalytic degradation of the endocrine-disrupting chemical Benzophenone-3: Parameters optimization and by-products identification | **Compliant** |
| `10.1016/j.jhazmat.2020.123591` | 2021 | *Journal of Hazardous Materials* | Benzophenone-3 degradation via UV/H2O2 and UV/persulfate reactions | **Compliant** |
| `10.1016/j.apcatb.2008.07.018` | 2009 | *Applied Catalysis B: Environmental* | Heterogeneous and homogeneous photocatalytic degradation of the insecticide imidacloprid in aqueous solutions | **Compliant** |
| `10.3390/jox15010005` | 2024 | *Journal of Xenobiotics* | Removal of Ibuprofen in Water by Bioaugmentation with Labrys neptuniae CSW11 Isolated from Sewage Sludge—Assessment of Biodegradation Pathway Based on Metabolite Formation and Genomic Analysis | **Compliant** |
| `10.1155/2018/2085059` | 2018 | *Journal of Analytical Methods in Chemistry* | HPLC Method for Quantification of Caffeine and Its Three Major Metabolites in Human Plasma Using Fetal Bovine Serum Matrix to Evaluate Prenatal Drug Exposure | **Compliant** |
| `10.1021/tx9802365` | 1999 | *Chemical Research in Toxicology* | Metabolic Activation of Diclofenac by Human Cytochrome P450 3A4:  Role of 5-Hydroxydiclofenac | **Compliant** |
| `10.1016/j.arabjc.2021.103635` | 2022 | *Arabian Journal of Chemistry* | Aromatic free Fenton process for rapid removal of phenol from refinery wastewater in an oscillatory baffled reactor | **Compliant** |
| `10.55730/1300-0144.6374` | 2026 | *Turkish Journal of Medical Sciences* | Antiobesity, antidiabetic, and antioxidant activities of selected Rubia species (Rubiaceae) and LC–MS/MS-based chemical profiling ​ | **Compliant** |
| `10.1021/es050824m` | 2005 | *Environmental Science &amp; Technology* | Aqueous Atmospheric Chemistry:  Formation of 2,4-Dinitrophenol upon Nitration of 2-Nitrophenol and 4-Nitrophenol in Solution | **Compliant** |
| `10.3390/ijms262010063` | 2025 | *International Journal of Molecular Sciences* | Sodium Hypochlorite-Assisted Photooxidation of Salicylic Acid: Degradation Kinetics, Formation, and Ecotoxicological Assessment of Intermediates | **Compliant** |

---

## 4. Isomer Pairs of Transformation with Retention Times Measured by RP-LC

**12 real pairs of positional isomers** (same neutral chemical formula and same exact monoisotopic mass) arising from degradation or metabolic processes were identified and validated, separated by reversed-phase chromatography (RP-C18 columns) with rigorously documented analytical parameters.

### Summary of the 12 Verified Pairs (`isomer_pairs_rt.csv`):
1. **Ibuprofen**: 2-hydroxyibuprofen ($t_R = 7.64	ext{ min}$) vs 1-hydroxyibuprofen ($t_R = 7.84	ext{ min}$) on Halo C-18 ($50 	imes 4.6	ext{ mm}$, $2.7\ \mu	ext{m}$). The tertiary aliphatic hydroxyl at C2 reduces hydrophobic interaction more than the secondary benzylic hydroxyl at C1 (*J. Xenobiotics 2024*, doi:10.3390/jox15010005).
2. **Caffeine**: Theobromine ($t_R = 3.8	ext{ min}$) vs Paraxanthine ($t_R = 4.8	ext{ min}$) on Zorbax SB-Aq ($2.1 	imes 100	ext{ mm}$, $3.5\ \mu	ext{m}$) with a phosphate buffer/ACN gradient (*J. Anal. Methods Chem. 2018*, doi:10.1155/2018/2085059).
3. **Caffeine**: Paraxanthine ($t_R = 4.8	ext{ min}$) vs Theophylline ($t_R = 5.5	ext{ min}$) on the same Zorbax SB-Aq column. Critical resolution of a pair traditionally co-eluting on conventional isocratic C18 phases.
4. **Caffeine**: Theobromine ($t_R = 3.8	ext{ min}$) vs Theophylline ($t_R = 5.5	ext{ min}$) ($\Delta t_R = 1.7	ext{ min}$).
5. **Diclofenac**: 4'-hydroxydiclofenac ($t_R = 25.4	ext{ min}$) vs 5-hydroxydiclofenac ($t_R = 26.8	ext{ min}$) on Supelcosil LC-18 ($250 	imes 4.6	ext{ mm}$, $5\ \mu	ext{m}$) with a methanol/acidified water gradient (*Chem. Res. Toxicol. 1999*, doi:10.1021/tx9802365).
6. **Phenol**: Hydroquinone ($t_R = 5.5	ext{ min}$) vs Catechol ($t_R = 10.9	ext{ min}$) on Nucleosil 100-5 C18 ($150 	imes 4.6	ext{ mm}$, $5\ \mu	ext{m}$) under isocratic buffer/MeOH/THF (*Arab. J. Chem. 2022*, doi:10.1016/j.arabjc.2021.103635). Hydroquinone elutes first because of strong polar solvation, compared with the partial intramolecular chelation of catechol.
7. **Phenol**: Hydroquinone ($t_R = 5.5	ext{ min}$) vs Resorcinol ($t_R = 9.5	ext{ min}$) on the same Nucleosil C18 column.
8. **Phenol**: Resorcinol ($t_R = 9.5	ext{ min}$) vs Catechol ($t_R = 10.9	ext{ min}$) ($\Delta t_R = 1.4	ext{ min}$).
9. **Benzoic acid**: 4-Hydroxybenzoic acid ($t_R = 10.5	ext{ min}$) vs Salicylic acid (2-hydroxybenzoic acid, $t_R = 21.8	ext{ min}$) on Poroshell 120 EC-C18 ($150 	imes 2.1	ext{ mm}$, $2.7\ \mu	ext{m}$) (*Turk. J. Med. Sci. 2026*, doi:10.55730/1300-0144.6374). Salicylic acid is retained more than twice as long because of the intramolecular hydrogen bond between the *ortho* $-	ext{OH}$ and the carboxyl group, which shields the polarity from the aqueous mobile phase.
10. **Nitrobenzene**: 4-nitrophenol ($t_R = 5.1	ext{ min}$) vs 2-nitrophenol ($t_R = 9.2	ext{ min}$) on LiChrospher 100 RP-18 ($125 	imes 4	ext{ mm}$, $5\ \mu	ext{m}$) (*Environ. Sci. Technol. 2005*, doi:10.1021/es050824m). The *ortho* isomer forms a strong intramolecular hydrogen bond $	ext{OH}\cdots	ext{O=N}$, dramatically increasing the apparent hydrophobicity on C18.
11. **Nitrobenzene**: 2,4-dinitrophenol ($t_R = 8.3	ext{ min}$) vs 2,6-dinitrophenol ($t_R = 9.4	ext{ min}$) on the same LiChrospher RP-18 column. The 2,6 isomer has two nitro groups *ortho* to the hydroxyl, maximizing intramolecular chelation and the retention delay.
12. **Chlorobenzene / Salicylic acid**: 2,6-dichlorophenol ($t_R = 10.682	ext{ min}$) vs 2,4-dichlorophenol ($t_R = 11.063	ext{ min}$) on a core-shell C18 column ($100 	imes 3	ext{ mm}$, $2.6\ \mu	ext{m}$) with a water/methanol gradient (*Int. J. Mol. Sci. 2025*, doi:10.3390/ijms262010063).

---

## 5. Critical Considerations on the Limits of the Analytical Data

1. **Recognition of positional isomers in the absence of standards**:
   - In low- or high-resolution mass spectrometry without specific fragmentation, aromatic hydroxylation isomers (e.g. 2-OH vs 3-OH-carbamazepine, or 2-OH vs 4-OH-SMX) generate precursor $m/z$ values and tandem fragmentation profiles that are almost indistinguishable.
   - Only chromatographic separation on a suitable stationary phase or comparison with commercial/synthesized analytical standards allows unambiguous site assignment (*high* confidence).
2. **Effect of intramolecular hydrogen bonding on elution order**:
   - In *ortho*-oriented substituents bearing hydrogen-bond donors and acceptors (e.g. salicylic acid, 2-nitrophenol, 2-chlorophenol), the intramolecular hydrogen bond shields the polar groups from the aqueous environment. Consequently, contrary to generic topological intuition, **the ortho isomer is often much more lipophilic and elutes after the para or meta isomer** in C18 reversed-phase chromatography.
3. **Reaction kinetics and transient vs persistent products**:
   - In AOP processes, many primary $ullet	ext{OH}$ radical adducts (e.g. cyclohexadienyl adducts or hydroxyalkyl radicals) have half-lives below one microsecond and undergo dehydration, elimination or further rapid oxidation (e.g. 5-hydroxyimidacloprid $	o$ olefin; catechol $	o$ ortho-quinone). Analytical studies measure the final stable products or intermediates long-lived enough for LC analysis.
4. **Value for the mzLab calculation engine**:
   - The quantitative data collected in `isomer_pairs_rt.csv` and `experimental_sites.csv` provide an objective test basis both for the Fukui orientation quantum-mechanical module (T2/T7) and for the RP-LC retention-ordering tests (T5).
