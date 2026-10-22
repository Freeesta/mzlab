# Manuale d'uso MSn: dall'infusione `.raw` all'albero, alla libreria e alla tabella

Per chi lancia il lavoro dalla shell o da un piano Flow; il progetto del grafo è in `msn-grafo.md`, la libreria in `msnlib.md`. Il programma non dà verdetti: le sottoformule sono candidati con ppm e prove.

## 1. Riga di comando (`mzlab-cli`)
- **Grafo**: `mzlab-cli msn-graph INFUSIONE.raw --formula "C13H24N4O3S [M+H]+" --out grafo.json`. Il file può essere un `.raw` Thermo o un mzML; `--formula` accetta lo ione (`C13H25N4O3S+`, `C13H24N4O3S+H`, `"FORMULA [M+H]+"`); senza, il grafo ha solo le masse. Opzioni utili: `--attivazione cid|hcd` (un solo tipo), `--ppm-formula 10`, `--isotopi`, `--scelta MZ=ION` (fissa la formula di un nodo), `--smiles` (scritto nel JSON). `--riassunto` stampa una riga di numeri.
- **Libreria**: `mzlab-cli msn-lib add lib.json INFUSIONE.raw --id ID --formula ION` aggiunge la voce (la stessa voce due volte resta com'è); `msn-lib build MANIFEST.tsv` ne scrive una per riga (`id`, `formula_neutra`, `addotto_atteso`, `file`).
- **Esportazione come libreria**: `mzlab-cli msn-lib export lib.json --formato msp|massbank [--pulito] [--tutti] [--annota]` (`--pulito` tiene i picchi spiegati da un nodo; `--tutti` ogni livello MS; `--annota` scrive formula e perdita neutra di ogni picco).
- **Esportazione come tabella**: `mzlab-cli msn-graph INFUSIONE.raw --formula ION --export csv|mgf|msp [--pulito] [--nodo nK|tutto]`.
- **Cerca**: `mzlab-cli msn-lib search lib.json grafo.json [--ppm 10] [--tol 0.01] [--min-prove 1] [--top 10]` dà `msncerca/1`: tre punteggi separati (entropia MS2, albero, perdite condivise) con le prove, senza verdetto.

## 2. Passi del Flow
Piano `mzflow/2` con l'ingresso `.raw` (o mzML) e, in ordine:
- `spectra.msn_graph@1`: porta `source` (l'infusione), uscita `graph` (`msngrafo/1`). Il composto si dà con `formula` + `adduct`, oppure con `molecule` (nome) e la porta `molecules` (elenco CSV delle molecole); gli altri parametri sono quelli di `msn-graph` (`ppm_formula`, `attivazione`, `isotopi`, `scelte`, `smiles`).
- `msn.library_export@1` (`graph` → libreria): `format` msp|massbank, `levels` all|ms2, `clean`, `annotate`, `id` (nome del record).
- `msn.table_export@1` (`graph` → tabella CSV): `view` modifications|nodes|peaks, `clean`.
Le perdite neutre stanno nella tabella `modifications`, le sottoformule in `nodes`, i picchi in `peaks`; la vista «Grafo MSn» della corsa le mostra senza aprire il CSV. Ogni parametro ha il suo `--set ISTANZA.NOME=VALORE` in `mzlab-cli flow run`.

## 3. Come si legge il risultato
- Ogni nodo è uno ione con formula proposta e ppm; l'arco tra due nodi è la perdita neutra (formula e massa a 5 decimali).
- Un frammento con più formule possibili resta con tutti i candidati: la scelta è dell'utente (`--scelta`).
- A risoluzione unitaria un m/z è un candidato, non un'identificazione.
