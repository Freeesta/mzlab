# Regulatory lists (`normative/`)

One CSV per list in `lists/` (schema A1 plus `entry_ref;ec;dtxsid;form;status`, separator `;`), the provenance of every input in `provenance.json`, the entries that could not be tied to a structure in `unresolved.tsv`, and the pins and hand-checked overrides in `sources.json` and `overrides.tsv`. The package the site and the app download is built from this text: `mzlab-cli normative pack normative/ OUT/`.

**Only the official publications are authentic.** These files are a working copy for screening: a list match is a candidate with its evidence, never a verdict. Entries without a CAS number in the law's text, groups, UVCB substances and inorganic compounds are listed with their `status` and are not screenable by mass.

## Sources, licences and attributions
| Lists | Source | Licence | Attribution |
|---|---|---|---|
| `EU_PS_2013_39`, `EU_PS_2026_805`, `EU_WL_2025_439`, `EU_DWD_WL`, `EU_GWD`, `EU_POPS_2019_1021`, `EU_FCM_10_2011`, `EU_TOYS` | EUR-Lex consolidated texts (Official Journal of the European Union) | Reuse with acknowledgement under Commission Decision 2011/833/EU | Source: EUR-Lex, European Union, https://eur-lex.europa.eu |
| `EU_PPP` | EU Pesticides Database (no legal value) | CC BY 4.0 | Source: European Commission, EU Pesticides Database, https://ec.europa.eu/food/plant/pesticides/eu-pesticides-database |
| `US_CWA_PP`, `US_CWA_TOXIC`, `US_SDWA_MCL`, `US_UCMR5`, `US_CERCLA` | Electronic Code of Federal Regulations | Public domain (17 USC 105) | Source: eCFR, https://www.ecfr.gov |
| `EPA_CCL5`, `EPA_CCL6_DRAFT`, `US_UCMR6_PROP` | US EPA | Public domain (17 USC 105) | Source: US EPA, https://www.epa.gov/ccl and https://www.epa.gov/dwucmr |
| Chemical identity (InChIKey, MS-ready structure, formula, mass) | NORMAN SusDat NORMAN-SLE-S0.0.6.1, doi:10.5281/zenodo.15741861 | CC BY 4.0 | NORMAN Network, Suspect List Exchange |
| Chemical identity of CAS numbers SusDat does not have | PubChem, NCBI/NLM/NIH (`CID-Identifiers`, `CID-Parent`, PUG REST) | Public domain | NCBI PubChem |

CAS numbers are shown as printed in the law (a citation of the act); no CAS Index Names are copied.

## Not included
The ECHA Candidate List (ECHA's terms forbid replicating its databases in whole or in substantial part) and California Proposition 65 (terms not verified) are never committed: the user imports the official file and it is read on their computer. The TSCA Inventory and the phase 2 lists (REACH XIV/XVII, CLP Annex VI, COSM, RoHS, bisphenols of the FCM, 37/2010 Table 1, CosIng) are not part of this release.

## Rebuild
```
mzlab-cli normative build --cache DIR --out normative/ [--allow-large]
mzlab-cli normative pack normative/ OUT/
```
`build` needs the network the first time (every URL goes into `--cache` with its SHA-256 and date; a changed source stops the build until `--update-pins`); with a warm cache it is offline and byte-identical. `pack` never uses the network. Details: `docs/agenti/normative.md`.
