# Il record `mol/1` (`crates/mzlab-mol`)

Struttura chimica come dato: una molecola (un componente) con atomi, legami in forma di Kekulé, elementi stereo con stato, gruppi di stereo avanzata e strati facoltativi. Studio e motivi: `docs/studi/STRUTTURA_DATO.md` (+ `_PROVE`, `_TRASFORMAZIONI`). Il crate dipende solo da `serde`, `serde_json`, `sha2`; non è collegato a nessun `.wasm` del sito; `mzlab-feat::chem::grafo` non lo usa ancora. Uscite e formati esistenti non cambiano.

## Schema JSON
Compatto, ordine dei campi fisso (quello dei tipi di `model.rs`), campi vuoti omessi; stessa molecola e stesso ordine degli atomi = stessi byte.
- `format`: sempre `"mol/1"`. `atoms[i]`: `{el, iso, chg, rad, h}` (simbolo H-Og; isotopo 0 = naturale; `h` = H impliciti, un H con isotopo è un atomo). `bonds`: `{a, b, o}` con `a < b`, `o` ∈ 1,2,3 (Kekulé, l'aromaticità non si scrive), senza duplicati.
- `stereo`: `{t:"tetra", c, refs, implicitH, par, status}` o `{t:"db", a, b, refs, cfg, status}`; `status` `defined` o `unknown`. Nessun elemento = il centro non è stereogenico; `unknown` = c'è ma non si sa (`{"t":"tetra","c":6,"status":"unknown"}`, senza `refs`/`par`/`cfg`).
- `tetra` definito: `refs` = vicini del centro in ordine crescente (3 o 4); `par` `cw`/`ccw` = senso in cui gli altri `refs` appaiono guardando dal primo; con 3 `refs` il quarto posto implicito (H se `h` = 1, `implicitH` true; coppia solitaria se `h` = 0, `implicitH` false) viene per ultimo. È la convenzione `CHI_TETRAHEDRAL_CW/CCW` di RDKit sull'ordine dei `refs`.
- `db` definito: `refs[0]` vicino di `a`, `refs[1]` vicino di `b` (non il legame stesso), `cfg` `cis`/`trans` tra i due (mai E/Z: le etichette CIP si calcolano). Il legame `a`-`b` è doppio.
- `groups`: `{kind: abs|and|or, atoms}` (`and` racemo, `or` relativa), atomi crescenti, solo centri `tetra` definiti, ognuno in un solo gruppo.
- Strati facoltativi, fuori dall'hash: `coords {dim 2|3, xy (righe di dim numeri), from}`, `id {can, inchikey, skeleton}` (segnaposto), `props [{name, from, can, v}]` (un valore per atomo), `regions [{atoms, delta, kind zone|multicenter|whole, support}]`.
- Un record per componente: i sali (due componenti nello stesso record) passano la validazione, ma l'identità per l'MS è quella del genitore.

## Funzioni pubbliche (`src/`)
- `Mol::new(atoms, bonds, stereo, groups)`: record col solo nucleo, nell'ordine dato (l'ordine canonico arriva con il pacchetto P2 dello studio).
- `Mol::from_json(text)`: legge e valida; `MolError::Json` se non è JSON della forma, `MolError::Invalid` se rompe una regola.
- `Mol::validate()`: indici nei limiti, simboli, ordini 1-3, `a < b`, refs davvero vicini, stato coerente con i campi presenti, gruppi su centri definiti, lunghezze degli strati.
- `Mol::to_json()`: JSON compatto e deterministico di tutto il record.
- `Mol::core_json()`: JSON di `{atoms, bonds, stereo, groups}`, i byte che entrano nell'hash.
- `Mol::core_hash()` / `core_hash_hex()`: SHA-256 di `core_json()`; coordinate, id, proprietà, regioni non lo cambiano; segue l'ordine degli atomi dato.
- `hex(bytes)`: esadecimale minuscolo. `MolError`: errore con `Display`.

## Prove (`tests/record.rs`, `tests/data/golden.json`)
- Dati d'oro da RDKit: `python3 tools/struttura_dato/confronto.py --golden crates/mzlab-mol/tests/data/golden.json` (serve `pip install rdkit`, solo per rigenerarli; il test Rust non lo richiede). Per le 20 molecole di `STRUTTURA_DATO_PROVE.md`: record con atomi nell'ordine scritto e nell'ordine canonico di RDKit, CXSMILES, InChIKey, SHA-256 del nucleo calcolato in Python.
- Il test rilegge ogni record, lo riscrive (stesso JSON), confronta l'hash con quello di Python (20/20 × 2 ordini). Trappole provate: stereo sconosciuta ≠ nessuna stereo (hash diversi), S ≠ R ≠ sconosciuta ≠ racemo, strati fuori dal nucleo non cambiano l'hash, record rotti respinti.
- Regole di dimensione: file ≤ 500 righe, funzioni ≤ 80 (`tools/controlla.py`).
