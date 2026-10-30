# Liste normative UE e USA: costruttore (`mzlab-cli normative build`)

Area: `crates/mzlab-cli/src/normative/` (solo nativo, `#[cfg(not(target_family = "wasm"))]`), dati in `normative/` (`sources.json`, `overrides.tsv`; `lists/`, `provenance.json` e `unresolved.tsv` sono l'uscita), prove in `crates/mzlab-cli/tests/normative_build.rs` con fixture inventate in `tests/data/normative/`. Studio: `docs/studi/LISTE_NORMATIVE.md`. Le liste sono DATI scaricati su clic (pacchetto B), mai codice nei `.wasm`.

## Uso
- `mzlab-cli normative build --cache DIR --out normative/ [--only ID...] [--update-pins] [--no-resolve] [--allow-large] [--sources FILE] [--overrides FILE]`. `--cache` tiene un file per URL (`DIR/<sha256 URL>` più `.json` con URL, data, SHA-256, byte); con la cache calda non c'è nessuna richiesta e l'uscita è identica byte per byte (l'orologio si legge solo al primo scarico).
- Uscita: 0 fatto; 1 una lista è `stale` (tiene l'ultimo CSV buono, `stale:true` e `error` in `provenance.json`); 2 uso o `sources.json`; 3 pin diverso; 4 scarico sopra 90 MB (`--allow-large`); 5 fonte, lettura o risolutore in errore; 6 file. Chiavi `err.normative.usage|sources|pin|large|fetch|parse|io`.
- Una lista mai si svuota in silenzio: 0 voci è un errore di lettura.

## Fonti e pin
- `normative/sources.json` (`normative-sources/1`): `resolve` (SusDat su Zenodo, `CID-Identifiers.tsv.gz` e `CID-Parent.gz` di PubChem, modello PUG REST per le proprietà, DSSTox opzionale) e `lists[]` con `id`, `reader`, `urls[{url, sha256}]`, `params` e le etichette dello studio §4 (`jurisdiction`, `instrument`, `citation`, `annex`, `status`, `in_force_from`, `applies_from`, `version_date`, `licence`, `attribution`).
- Il pin è lo SHA-256 dei byte letti (per l'API Pesticidi a pagine, lo SHA-256 degli SHA-256 delle pagine); `null` la prima volta. Un byte diverso ferma la costruzione (uscita 3); `--update-pins` riscrive `sources.json` dopo il controllo di chi cura.
- URL `file:PERCORSO`: un file scaricato a mano dall'utente (SVHC dell'ECHA, Proposition 65 dell'OEHHA), messo in cache come gli altri.
- Niente scarichi dal sito ECHA: REACH XIV/XVII e CLP Annex VI si leggono dai testi consolidati su Cellar (fase 2); il lettore `svhc` esiste solo per il file dell'utente.

## Lettori (`readers.rs`)
- `eu_water`, `eu_chem`, `us_epa_html`: tabella guidata dalle intestazioni (`params.columns`: `ref`, `name`, `cas`, `ec`, `dtxsid`, `attr[{match,label}]`; `annexes`; `prefix`), con `colspan` e `rowspan` (`markup.rs`, `table_rd.rs`); XHTML di Cellar con `Accept: application/xhtml+xml`.
- `us_ecfr`: XML del versioner eCFR (gzip), `section` per intestazione, modo `lines` («12. Nome») o tabella.
- `eu_ppp`: API Pesticidi v3.0 a pagine (`nextLink`); microrganismo = `uvcb`, gruppo = `group`.
- `voci`: file di voci trascritte (`entry_ref;name;cas;ec;attr;kind;group_entry;act`, atto citato su ogni riga, `inchikey` opzionale = `law`).
- `svhc`, `ca_prop65`: CSV dell'utente, separatore indovinato.
- Ogni voce è `{entry_ref, name, cas, ec, dtxsid, attr, group_entry, kind}`; `kind` ∈ structure | group | uvcb | inorganic (`entry.rs`). Un CAS si trova per forma; la cifra di controllo serve solo a dire a chi cura che un numero può essere un refuso.

## `resolve` (`resolve.rs`)
Ordine: `law` (InChIKey dato dall'atto), `susdat` (StdInChIKey e MS_Ready), `pubchem` (CAS → CID → genitore → proprietà PUG in lotti di 100, solo per i CAS che SusDat non ha), `dsstox` (solo dove la lista dà un DTXSID), `curated` (`overrides.tsv`: `list_id;entry_ref;cas;inchikey;reason`), altrimenti `unresolved`. Chiave: primo blocco dell'InChIKey MS-ready (`inchikey_skeleton`). Un CAS con due scheletri è `unresolved`, con entrambi in `unresolved.tsv`. `form`: salt (punto nello SMILES, carica positiva, CID elencato ≠ genitore), anion, parent, as_listed.

## Uscita (`build.rs`)
- `lists/<ID>.csv`: schema A1 più `entry_ref;ec;dtxsid;form;status`, separatore `;`, righe ordinate per (`entry_ref`, scheletro, nome), leggibile da `reglists::schema::read_list`. `status` è il tipo di riga: `screenable`, `group`, `uvcb`, `inorganic`, `unresolved`. Formula e massa vengono da SusDat se la forma registrata è lo scheletro MS-ready, altrimenti si calcolano dallo SMILES MS-ready (`mzlab_feat::chem`).
- `provenance.json` (`normative-prov/1`): per lista etichette, URL con SHA-256 e data, copertura `{entries, screenable, groups, uvcb, inorganic, unresolved}`, `csv_sha256`, `stale`.
- `unresolved.tsv`: `list_id, entry_ref, name, cas, reason`; le liste non ricostruite (`--only`) tengono le loro righe.
- `normative/lists/` non si versa nel repository da questo costruttore: lo fa il pacchetto B dopo la decisione D1.

## Liste di `sources.json`
EU_PS_2013_39, EU_PS_2026_805, EU_WL_2025_439, EU_DWD_WL, EU_GWD, EU_POPS_2019_1021, EU_FCM_10_2011, EU_TOYS, EU_PPP, US_CWA_PP, US_CWA_TOXIC, US_SDWA_MCL, US_UCMR5, US_CERCLA, US_UCMR6_PROP, EPA_CCL5, EPA_CCL6_DRAFT. Nel catalogo (`reglists/catalog.rs`) anche `CA_PROP65` (gruppo «US state») e `SVHC`, che si costruiscono dal file dell'utente.
Fase 2 (non costruite): REACH XIV/XVII e CLP Annex VI da Cellar, COSM, RoHS, bisfenoli dei FCM, 37/2010 Tab. 1, TSCA, CosIng.
