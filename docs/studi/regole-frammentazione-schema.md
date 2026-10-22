# Regole di frammentazione: schema `fragrules/1` e tabella delle regole

Seguito di `regole-frammentazione-cid-hcd.md` (principi, CID/HCD, bibliografia con le chiavi `[XXX00]`). Qui la forma macchina-leggibile (c) e ogni regola con la sua citazione (d). Le masse sono monoisotopiche, ricalcolate dalle formule con le masse di `mzlab-core::elements` (H 1,00782503, C 12, N 14,00307400, O 15,99491462, S 31,97207100, P 30,97376163, F 18,99840322, Cl 34,96885268, Br 78,9183371): nella tabella dati vive solo la formula, la massa si calcola.

## 1. Forma del file (proposta)
File `crates/mzlab-core/data/fragrules-1.json`, chiavi in inglese come `classi-regole/1` (`crates/mzlab-feat/data/classi-1.json`); schema JSON in `docs/schemi/fragrules-1.json` nella PR che lo introduce. Nessun testo per l'utente nel file: la pagina mostra `id`, formula e citazione; le frasi visibili, se servono, sono chiavi `msngrafo.rule.*` in `lang/`.
- **Testa**: `{"format": "fragrules/1", "rules": [...], "globals": [...]}`; `rules` ordinate per `id`; nessuna chiave sconosciuta (il lettore la rifiuta con `err.fragrules.badKey`).
- **Regola** (`rules[]`), identità e chimica: `id` (minuscole e `_`, unico), `kind` (`loss` perdita neutra su un arco | `ion` ione diagnostico su un nodo | `cleavage` taglio di struttura con il pezzo calcolato dal SMILES: RDA, McLafferty, α), `formula` (perdita o ione, neutra; per `ion` la formula dello ione senza carica, la m/z toglie o aggiunge l'elettrone), `radical` (bool: perdita o ione a elettroni dispari).
- **Regola, condizioni**: `requires` (`all_groups` e `any_groups`: nomi di `Mol::groups()` in `crates/mzlab-feat/src/chem/gruppi.rs`; `site`: nome di un controllo di sottostruttura del valutatore, §3; `smarts`: testo solo documentale, non letto), `excludes` (gruppi che spengono la regola), `ions` (fra `[M+H]+`, `[M-H]-`, `[M+Na]+`, `[M+NH4]+`, `[M+HCOO]-`, `[M+Cl]-`, `[M]+`; vuoto = tutti quelli della polarità), `polarity` (`+`, `-`, `both`).
- **Regola, attivazione e ordine**: `activation` (`cid_trap`, `beam` = HCD e CID di collisore, `both`), `energy` (`low`, `mid`, `high`: dove il canale compare per primo), `charge` (`directed`, `remote`, `any`), `priority` (1 alta, 2 media, 3 bassa: ordine di prova e peso della prova), `after` (id di regole che devono precedere sullo stesso cammino, es. CO dopo l'acilio).
- **Regola, fonti**: `refs` (`[{key, pmid | null, doi | null, check}]`, `check` = `abstract` (il riassunto conferma il legame gruppo-perdita), `text` (da leggere sul testo), `todo` (riferimento da verificare)), `why` (una riga tecnica in inglese).
- **Regole globali** (`globals[]`): vincoli che non dipendono da un gruppo: `even_electron` (perdita radicale ammessa solo con una regola `radical: true`), `trap_cutoff` (soglia `q/0.908 × m/z del precursore`, `q` predefinito 0,25, solo `cid_trap`), `beam_composite` (in `beam` una perdita = somma di due regole sullo stesso arco, con la regola intermedia «non osservata»), `stepped_sum` (spettro a energie sommate: nessuna curva di breakdown), `loss_tolerance_mda` (tolleranza assoluta della perdita, predefinita 3 mDa).
- **Esempio** (una regola):
```json
{"id": "loss_co2_carboxylate", "kind": "loss", "formula": "CO2", "radical": false,
 "requires": {"all_groups": ["carboxylic_acid"], "any_groups": [], "site": null, "smarts": "[CX3](=O)[OX1-]"},
 "excludes": [], "ions": ["[M-H]-"], "polarity": "-", "activation": "both", "energy": "low",
 "charge": "directed", "priority": 1, "after": [],
 "refs": [{"key": "LEV07", "pmid": "17605143", "doi": "10.1002/jms.1234", "check": "abstract"}],
 "why": "Deprotonated carboxylic acids lose CO2; characteristic and specific in negative mode."}
```
- **Uscita nel grafo** (chiave facoltativa `regole` di `msngrafo/1`, presente solo se chiesta; senza, i byte non cambiano): per arco `{da, a, regole: [{id, priorita, fonte: [chiavi], verifica}]}`, per nodo `{nodo, ioni: [{id, ppm}]}`, per candidato la prova `"regola"` accanto a `perdita`/`isotopi`/`msn`.
- **Avvisi e conteggi**: `warn.msngrafo.belowCutoff` (frammento atteso sotto il cut-off), `warn.msngrafo.radicalLoss` (perdita radicale senza regola), conteggi `{archi_spiegati, archi_senza_regola, intensita_spiegata}`. Mai una scelta di formula: le regole sono prove (`msn-grafo.md` §0.2: nessuna proposta automatica).

## 2. Tabella delle regole (prima stesura, da tradurre in `fragrules-1.json`)
Colonne: id · tipo · formula (massa) · richiede · ioni/polarità · attivazione · energia · priorità · fonti (`check`). «nuovo gruppo» = nome da aggiungere a `Mol::groups()` nella PR del valutatore.
| id | tipo | formula (massa, Da) | richiede | ioni / pol. | att. | en. | pr. | fonti |
|---|---|---|---|---|---|---|---|---|
| loss_h2o_alcohol | loss | H2O (18,010565) | alcohol (alifatico) | + e − | both | low | 1 | [HOL10] text, [DEM16] text |
| loss_h2o_acid_pos | loss | H2O (18,010565) → acilio | carboxylic_acid | [M+H]+ | both | low | 1 | [HOL10] text |
| loss_ch2o2_acid_pos | loss | CH2O2 (46,005479) | carboxylic_acid | [M+H]+ | both | mid | 2 | [LEV07] abstract (caratteristica, non specifica) |
| loss_co2_carboxylate | loss | CO2 (43,989829) | carboxylic_acid | [M−H]− | both | low | 1 | [LEV07] abstract |
| loss_co_acylium | loss | CO (27,994915) | after: loss_h2o_acid_pos, loss_ch4o_ester, loss_nh3_amide | + | both | mid | 2 | [HOL10] text |
| loss_nh3_amine | loss | NH3 (17,026549) | amine_primary | [M+H]+ | both | low | 1 | [HOL10] text, [DEM16] text |
| loss_nh3_amide | loss | NH3 (17,026549) | amide (primaria, site `amide_nh2`) | [M+H]+ | both | mid | 2 | [HOL10] text |
| loss_nh3_ammonium_adduct | loss | NH3 (17,026549) | nessuno | [M+NH4]+ | both | low | 1 | [HOL10] text |
| loss_ch5n_n_methyl | loss | CH5N (31,042199) | amine_secondary, n_methyl | [M+H]+ | both | mid | 2 | [HOL10] text |
| loss_c2h7n_n_dimethyl | loss | C2H7N (45,057849) | amine_tertiary, n_methyl (site `n_dimethyl`) | [M+H]+ | both | mid | 2 | [HOL10] text |
| loss_c2h4_n_ethyl | loss | C2H4 (28,031300) | n_alkyl (site `n_ethyl`) | [M+H]+ | both | mid | 2 | [HOL10] text, [DEM16] text |
| loss_c3h6_n_isopropyl | loss | C3H6 (42,046950) | n_isopropyl | [M+H]+ | both | low | 1 | [HOL10] text |
| loss_c4h8_n_tbutyl | loss | C4H8 (56,062600) | n_tbutyl | [M+H]+ | both | low | 1 | [HOL10] text |
| loss_alkene_ester | cleavage | CnH2n dall'alchile (C2H4 28,031300 per etile) | ester (site `gamma_h_carbonyl`) | + | both | mid | 2 | [MCL59] text, [DEM16] text |
| loss_ch4o_ester | loss | CH4O (32,026215) | ester (site `methyl_ester`) o ether (site `methoxy_near_charge`) | + | both | mid | 2 | [HOL10] text |
| loss_c2h4o2_acetate | loss | C2H4O2 (60,021129) | ester (site `acetate`) | + | both | low | 2 | [HOL10] text |
| loss_c2h2o_acetyl | loss | C2H2O (42,010565) | amide o ester (site `acetyl_on_heteroatom`) | + | both | mid | 2 | [HOL10] text |
| loss_co_ring_neg | loss | CO (27,994915) | phenol, aromatic_ring | [M−H]− | both | mid | 3 | [FAB01] abstract |
| loss_co2_ring_neg | loss | CO2 (43,989829) | phenol, ketone (site `chromone`) | [M−H]− | both | mid | 3 | [FAB01] abstract |
| loss_c3o2_ring_neg | loss | C3O2 (67,989829) | phenol, ketone (site `chromone`) | [M−H]− | cid_trap | high | 3 | [FAB01] abstract |
| rda_six_ring | cleavage | pezzi del diene e del dienofilo dal SMILES | site `cyclohexene_ring` | both | both | mid | 2 | [FAB01] abstract, [DEM16] text |
| loss_so2_sulfonamide | loss | SO2 (63,961900) | sulfonamide, aromatic_ring | + e − | both | mid | 2 | [HOL10] text |
| loss_so3_sulfate_neg | loss | SO3 (79,956815) | nuovo gruppo `sulfate` o sulfonyl (site `so3h`) | [M−H]− | both | low | 1 | [LEV07] abstract |
| loss_hf | loss | HF (20,006228) | fluoro | + e − | both | mid | 3 | [HOL10] text |
| loss_hcl | loss | HCl (35,976678) | chloro | + e − | both | mid | 2 | [HOL10] text |
| loss_hbr | loss | HBr (79,926162) | bromo | + e − | both | mid | 2 | [HOL10] text |
| loss_no2_radical | loss | NO2• (45,992903), radical | nitro, aromatic_ring | + e − | both | mid | 1 | [LEV07] abstract |
| loss_no_radical | loss | NO• (29,997989), radical | nitro, aromatic_ring | + e − | both | mid | 2 | [LEV07] abstract |
| loss_ch3_radical_methoxyaryl | loss | CH3• (15,023475), radical | ether, aromatic_ring (site `methoxy_aryl`) | + e − | both | mid | 2 | [JIA24] abstract (+ e −, HR), [VES11] abstract (−, QqQ, Q-TOF, Orbitrap), [MEN13] abstract (+, trappola lineare) |
| loss_hcn_heteroaromatic | loss | HCN (27,010899) | heteroaromatic (azotato) | + | beam | high | 3 | [HOL10] text |
| loss_hnco_ureido | loss | HNCO (43,005814) | amide (site `ureido_nh2`) | + | both | mid | 2 | [HAO09] abstract (gruppo ureidico, CID) |
| loss_c2h3no_n_methylcarbamate | loss | C2H3NO (57,021464) | ester, amide (site `n_methylcarbamate`) | + | beam | low | 2 | [ZHO18] abstract (CH3NCO, 15 carbammati, QTRAP) |
| loss_h3po4_phosphate | loss | H3PO4 (97,976895) | nuovo gruppo `phosphate` | + | both | low | 1 | [HOL10] text |
| loss_hexose | loss | C6H10O5 (162,052823) | nuovo gruppo `o_glycoside` | + e − | both | low | 1 | [MA14] abstract (glicosilazioni), [HOL10] text |
| loss_glucuronide | loss | C6H8O6 (176,032088) | nuovo gruppo `glucuronide` | + e − | both | low | 1 | [MA14] abstract, [HOL10] text |
| loss_hcooh_formate_adduct | loss | CH2O2 (46,005479) | nessuno | [M+HCOO]− | both | low | 1 | [HOL10] text |
| loss_hcl_chloride_adduct | loss | HCl (35,976678) | nessuno | [M+Cl]− | both | low | 1 | [HOL10] text |
| ion_tropylium | ion | C7H7+ (m/z 91,054226) | aromatic_ring (site `benzyl`) | + | beam | high | 2 | [RYL57] text, [HOL10] text |
| ion_immonium_alpha | cleavage | CnH2n+2N+ dal lato dell'azoto (CH4N+ 30,033826; C2H6N+ 44,049476; C3H8N+ 58,065126) | amine (site `alpha_c_n`) | + | beam | high | 2 | [OLS07] abstract (immonio visti in HCD, peptidi), [HOL10] text |
| ion_acylium | cleavage | R–C≡O+ dal SMILES | ketone, ester o amide (site `acyl`) | + | both | mid | 2 | [DEM16] text |
| ion_sodium | ion | Na+ (m/z 22,989221) | nessuno | [M+Na]+ | both | high | 3 | [HOL10] text |
| remote_fixed_charge | cleavage | tagli lontani dalla carica | ions [M+Na]+ o N quaternario | + | beam | high | 3 | [CHE00] abstract |
- **Masse degli ioni**: m/z = massa della formula − 0,000549 (elettrone) per i positivi; C7H7+ 91,054775 − 0,000549 = 91,054226; CH4N+ 30,034374 − 0,000549 = 30,033826 (ricalcolare nel test, mai copiare).
- **Controesempi da mettere nei test**: OH e NH2 aromatici NON danno una perdita che li riveli [LEV07] (nessuna regola `loss_h2o_phenol` in positivo); l'assenza di una perdita non esclude il gruppo [MA14]; uno ione molto stabile spegne gli altri canali [HOL10].
- **Regole con fonti non verificate**: nessuna regola della tabella ha solo `todo` (riferimenti controllati su PubMed o Crossref); una regola aggiunta con solo `todo` ha priorità 3 e resta fuori dal conteggio «spiegato» della convalida (§3 del piano) finché una persona non conferma la fonte.
- **Perdita di HNCO da carbammati primari (O–C(=O)–NH2)**: nessuna fonte trovata; resta fuori dalla tabella, `ureido_nh2` copre solo R–NH–C(=O)–NH2.

## 3. Controlli di sottostruttura (`site`) da scrivere
Senza SMARTS (`chem/grafo.rs` non ne ha): funzioni con nome su `Mol` (atomi, legami, aromaticità scritta, H impliciti), una per `site`, con un test su una molecola generica ciascuna. Elenco: `amide_nh2`, `n_dimethyl`, `n_ethyl`, `gamma_h_carbonyl`, `methyl_ester`, `methoxy_near_charge` (≤ 3 legami dal sito basico), `acetate`, `acetyl_on_heteroatom`, `chromone`, `cyclohexene_ring`, `so3h`, `methoxy_aryl`, `ureido_nh2`, `n_methylcarbamate`, `benzyl`, `alpha_c_n`, `acyl`.

Il sito di protonazione (per `charge: directed`) viene da `mzlab-chem::protomer` solo come indizio: se il protomere non ha carica sul sito, la regola scende di una priorità, non sparisce [TIA08].

## 4. Esempi sintetici per i test (nessun composto reale)
- **A**: ammina primaria alifatica con un acido carbossilico a 4 atomi, SMILES `NCCCC(=O)O`: [M+H]+ → −NH3, −H2O, −CH2O2; [M−H]− → −CO2.
- **B**: estere etilico di un acido aromatico con N-isopropile, SMILES `CCOC(=O)c1ccc(cc1)CNC(C)C`: −C3H6 (n_isopropyl), −C2H4 (McLafferty dell'estere), C7H7+ attesa in `beam`; in `cid_trap` è sopra il cut-off ([M+H]+ a m/z 222,149: 0,275 × 222 ≈ 61), lo stesso ione sotto un precursore oltre m/z 331 darebbe `warn.msngrafo.belowCutoff`.
- **C**: nitroaromatico clorurato, SMILES `Clc1ccc(cc1)[N+](=O)[O-]`: −NO2• e −NO• ammessi (regole radicali), −HCl; un −Cl• senza regola dà `warn.msngrafo.radicalLoss`.
- **Decoy**: per ciascuno, lo stesso insieme di atomi rimescolato (es. A → `OCC(N)CC=O` con la stessa formula) deve spiegare meno archi.
