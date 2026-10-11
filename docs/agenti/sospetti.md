# Indice dei sospetti `.mzsus` v1 (mzFinder, offline)
Stato attuale. Elenco di composti noti ordinato per massa monoisotopica, per le ipotesi H1 di mzFinder come LISTA di sospetti: un hit di massa è un candidato di livello 5/4 con le sue prove, mai un nome dato alla feature. Nessuna rete né API esterna a runtime. Mai nel repository né nel primo caricamento del sito: lo scarica solo mzFinder acceso.

## Fonti (tutte CC BY 4.0, versioni e SHA-256 fissati in `tools/genera_sospetti.py`, `SOURCES`)
- NORMAN SusDat (Zenodo 15741861, NORMAN-SLE-S0.0.6.1): colonne `StdInChIKey`, `Molecular_Formula`, `Monoiso_Mass`.
- PubChemLite for Exposomics (Zenodo 22953038, 3.3.0): `FirstBlock`, `MolecularFormula`, `MonoisotopicMass`, `PubMed_Count`, `Patent_Count`. Il CSV (200 MB) si scarica e si scarta.
- Transformations in PubChem (Zenodo 19150892, 0.2.3): `PubChem_all_transformations.csv` + `Transformations_CID_Info_all.csv`.
- Un file diverso dall'impronta fissata ferma la costruzione. BioTransformer, enviPath e CTS non entrano mai nell'indice (solo liste importate).

## Costruzione
`python3 tools/genera_sospetti.py [--fonte DIR] --out FILE [--pacchetto DIR]`. Righe fuse per InChIKey-14 (solo A-Z): massa e formula della prima fonte (SusDat, PubChemLite, Transformations), fonti in unione, conteggi al massimo, id di reazione il più basso. Scartate le righe senza chiave valida, massa o formula (conteggiate nel manifest: `letti` / `usati`). Ordine stabile (massa, InChIKey-14): stesse fonti = stesso file byte per byte.
`--pacchetto`: il file grezzo compresso con zlib (livello 9) a pezzi di 4 MiB, ciascuno con SHA-256, più `catalogo.json` (formato `sospetti/1`: dimensioni, SHA-256 del file grezzo, fonti con licenza e DOI, attribuzione). Il browser decomprime con `DecompressionStream("deflate")` e passa i byte grezzi al lettore. La CI di Pages lo mette in `static/mzfinder/sospetti/`.

## File grezzo (little-endian, ogni sezione comincia a un multiplo di 8)
Testata (32 byte): magic `MZSU`; u16 versione (1); u16 versione minima del lettore (1); u32 righe `n`; u32 formule `nf`; u32 reazioni `nr`; u32 byte del manifest; u32 bandierine (0); u32 riservato (0). Seguono, ciascuna riempita a 8:
| Sezione | Contenuto |
|---|---|
| manifest | JSON UTF-8: `formato`, `righe`, `attribuzione`, `fonti[]` (id, nome, versione, licenza, doi, url, istituzione, bit, sha256 dei file, letti, usati) |
| masse | `n` × f64 (Da monoisotopici, crescenti) |
| chiavi | `n` × 14 byte ASCII, InChIKey-14 |
| formule | `n` × u32, indice nella tabella delle formule |
| fonti | `n` × u8, bit: 1 SusDat, 2 PubChemLite, 4 Transformations |
| letteratura | `n` × u16 (PubMed, saturato a 65535) |
| brevetti | `n` × u16 (saturato) |
| reazioni | `nr` × (u32 indice di riga, u32 id), ordinate per riga; id = riga (da 1) della prima reazione del file di Transformations in cui il composto compare; assente = nessuna |
| offset formule | `nf+1` × u32 nel blob |
| blob formule | formule ASCII, ordinate, senza separatori |

## Lettore
`leggi_sospetti(percorso_o_byte)` → `Suspects` (`.manifest`, `.n`, `.hit(i)`, `.cerca(massa, ppm)`): finestra `|m − massa| ≤ massa·ppm·1e-6` con ricerca binaria, righe in ordine di massa e chiave. Un file senza magic o più nuovo di quanto il lettore sappia è rifiutato (`ValueError`). Il test è `tests/test_sospetti.py` (mini-insieme sintetico di chiavi inventate).
