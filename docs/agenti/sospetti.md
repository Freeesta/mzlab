# Indice dei sospetti `.mzsus` v1 (mzFinder, offline) e sua estensione `listings` (liste normative)
Elenco di composti noti ordinato per massa monoisotopica, per le ipotesi H1 di mzFinder come LISTA di sospetti: un hit di massa è un candidato di livello 5/4 con le sue prove, mai un nome dato alla feature. Nessuna rete né API esterna a runtime. Mai nel repository né nel primo caricamento del sito: lo scarica solo mzFinder.

## Fonti (tutte CC BY 4.0, versioni e SHA-256 fissati in `tools/genera_sospetti.py`, `SOURCES`)
- NORMAN SusDat (Zenodo 15741861, NORMAN-SLE-S0.0.6.1): colonne `StdInChIKey`, `Molecular_Formula`, `Monoiso_Mass`.
- PubChemLite for Exposomics (Zenodo 22953038, 3.3.0): `FirstBlock`, `MolecularFormula`, `MonoisotopicMass`, `PubMed_Count`, `Patent_Count`. Il CSV (200 MB) si scarica e si scarta.
- Transformations in PubChem (Zenodo 19150892, 0.2.3): `PubChem_all_transformations.csv` + `Transformations_CID_Info_all.csv`.
- Un file diverso dall'impronta fissata ferma la costruzione. BioTransformer, enviPath e CTS non entrano mai nell'indice (solo liste importate).

## Costruzione
`python3 tools/genera_sospetti.py [--fonte DIR] --out FILE [--pacchetto DIR]`. Righe fuse per InChIKey-14 (solo A-Z): massa e formula della prima fonte (SusDat, PubChemLite, Transformations), fonti in unione, conteggi al massimo, id di reazione il più basso. Scartate le righe senza chiave valida, massa o formula (conteggiate nel manifest: `letti` / `usati`). Ordine stabile (massa, InChIKey-14): stesse fonti = stesso file byte per byte. Con `--pacchetto` scrive anche `DIR/cas/` (sidecar `mzlab-cas/1` dei CAS di SusDat e `catalogo.json`; formato in `docs/agenti/normative.md`).
`--pacchetto`: il file grezzo compresso con zlib (livello 9) a pezzi di 4 MiB, ciascuno con SHA-256, più `catalogo.json` (formato `sospetti/1`: dimensioni, SHA-256 del file grezzo, fonti con licenza e DOI, attribuzione). Il browser decomprime con `DecompressionStream("deflate")` e passa i byte grezzi al lettore. La CI di Pages lo mette in `static/mzfinder/sospetti/`.

## File grezzo (little-endian, ogni sezione comincia a un multiplo di 8)
Testata (32 byte): magic `MZSU`; u16 versione (1); u16 versione minima del lettore (1); u32 righe `n`; u32 formule `nf`; u32 reazioni `nr`; u32 byte del manifest; u32 bandierine (bit 0 = estensione `listings`, altrimenti 0); u32 riservato (0). Seguono, ciascuna riempita a 8:
| Sezione | Contenuto |
|---|---|
| manifest | JSON UTF-8: `formato`, `righe`, `attribuzione`, `fonti[]` (id, nome, versione, licenza, doi, url, istituzione, bit, sha256 dei file, letti, usati) |
| masse | `n` × f64 (Da monoisotopici, crescenti) |
| chiavi | `n` × 14 byte ASCII, InChIKey-14 |
| formule | `n` × u32, indice nella tabella delle formule |
| fonti | `n` × u8, bit: 1 SusDat, 2 PubChemLite, 4 Transformations, 8 liste normative |
| letteratura | `n` × u16 (PubMed, saturato a 65535) |
| brevetti | `n` × u16 (saturato) |
| reazioni | `nr` × (u32 indice di riga, u32 id), ordinate per riga; id = riga (da 1) della prima reazione del file di Transformations in cui il composto compare; assente = nessuna |
| offset formule | `nf+1` × u32 nel blob |
| blob formule | formule ASCII, ordinate, senza separatori |

## Lettore
`leggi_sospetti(percorso_o_byte)` → `Suspects` (`.manifest`, `.n`, `.hit(i)`, `.cerca(massa, ppm)`): finestra `|m − massa| ≤ massa·ppm·1e-6` con ricerca binaria, righe in ordine di massa e chiave. Un file senza magic o più nuovo di quanto il lettore sappia è rifiutato (`ValueError`). Il test è `tests/test_sospetti.py` (mini-insieme sintetico di chiavi inventate).

## Lettore Rust (`crates/mzlab-feat/src/suspects.rs`)
`Suspects::open(&[u8])` legge il file grezzo (già decompresso) in righe `Hit` ordinate per massa (`mass`, `key` 14 byte, `formula` indice per `Suspects::formula`, `sources` bit `SRC_*`, `pubmed`, `patents`, `reaction`); `search(mass, ppm) -> &[Hit]` è una ricerca binaria con la stessa finestra del lettore Python. Errori: `Corrupt` e `Version`; nessuna dipendenza oltre il crate.
Il test `crates/mzlab-feat/tests/suspects.rs` confronta le risposte con `tools/sospetti_riferimento.py` (si salta senza python3; `MZLAB_GOLDEN_STRICT=1` fallisce) e misura la ricerca su 10⁶ righe sintetiche.
## Passo Flow `screen.suspects@1` (`crates/mzlab-feat/src/flow_screen_suspects/`)
Cerca per massa neutra e addotto (gli addotti e i ppm di `attributio::reglists::screen_types`) le feature dell'allineamento HR in uno o più file di sospetti dell'utente: `.mzsus` (`Suspects::search`) o CSV (`name`, `inchikey` o `smiles`, `formula` o `mass`, `rt`, `fragments` separati da `;`). Chiave = primo blocco dell'InChIKey; i risultati con la stessa formula sono un gruppo di isomeri, mai un solo nome.
Uscita `screen/1` (`docs/schemi/screen-1.json`): `suspect_files` (nome, SHA-256, manifest del `.mzsus` o «user CSV»), `hits[]` per feature × addotto × formula con `candidates[]`, `memberships[]` (`list`, `entry_ref`, `form`, `group`, `attr`; `[]` senza l'estensione `listings`), livello 5 o 4 (formula in `formulas/1`). Stessi byte nel CLI e nel `.wasm` `table` (`flow_screen_suspects`).
Con un pacchetto normativo (`.mzsus` con `listings`) i parametri `lists` (id separati da virgola; vuoto = tutte) e `include` (`proposal`, `draft`, `reference`; vuoto = solo `in_force` e `adopted_not_applicable`) fanno la maschera (`mzlab_feat::regulatory::Package`): una riga che nessuna lista tenuta nomina non è candidata; il nome e la chiave intera vengono da `listings`; `suspect_files[].lists` porta il manifest di ogni lista tenuta (versione, SHA-256, copertura), quindi la maschera entra in `params_sha256` e in `prov/1`.
Le liste importate (porta `imported_lists`, `normative.md`) sono sorgenti in più dopo i file di sospetti, con la maschera vuota; le loro appartenenze si aggiungono a quelle del pacchetto sullo stesso candidato.
Test: `crates/mzlab-cli/src/flow_screen_suspects_reg_tests.rs` (pacchetto inventato `crates/mzlab-cli/tests/data/normative_screen/`).

## Estensione `listings` (liste normative, bandierina bit 0; `reader_min` resta 1)
Dopo il blob delle formule, ogni sezione riempita a 8: testata (16 byte: u32 liste `nl`, u32 appartenenze `m`, u32 voci `ne`, u32 byte del blob delle voci); chiavi complete (`n` × 27 byte ASCII, l'InChIKey della forma elencata rappresentativa, spazi se manca); nomi (`n+1` × u32 offset, poi blob UTF-8); offset delle appartenenze (`n+1` × u32); appartenenze (`m` × [u16 indice di lista, u8 forma, u8 bit, u32 indice di voce]); voci (`ne+1` × u32 offset, poi blob di un oggetto JSON compatto per voce: `{"ref","name","cas","ec","attr"}`).
Forma: 0 non indicata, 1 `as_listed`, 2 `parent`, 3 `salt`, 4 `anion`. Bit: 1 membro di gruppo, 2 isomero indicato, 4 il CAS elencato è un anione o un sale. L'indice di lista punta a `normative/1.lists` del manifest (ordine del catalogo); le liste sono etichettate lì (`tags`, `coverage`, `csv_sha256`). Il manifest ha anche `fonti` con la sola fonte `normative` (bit 8).
`Suspects::listings() -> Option<&Listings>` (`None` senza la bandierina; `list_count`, `full_key`, `name`, `memberships`, `entry`, `entry_count`) e `Suspects::listings_json()` (tutta l'estensione come un solo JSON: stessi byte nel `.wasm` `normative_listings` e in `mzlab-cli normative listings`). I file v1 senza bandierina e mzFinder non cambiano. Il lettore di riferimento Python è `tools/sospetti_riferimento.py` (`write_listings`, `read_listings`); il test `crates/mzlab-feat/tests/suspects.rs` confronta `listings_json` con il suo.
Il costruttore è `mzlab-cli normative pack` (`docs/agenti/normative.md`): una riga per scheletro, massa e formula dalla CSV, appartenenze ordinate per (lista, `entry_ref`), byte deterministici.
