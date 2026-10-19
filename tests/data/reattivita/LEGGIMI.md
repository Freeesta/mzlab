# Insieme di validazione della reattività (`validazione.json`)

Molecola → sito sperimentale, per misurare il motore di `crates/mzlab-chem` (prova: `crates/mzlab-chem/tests/validazione.rs`). Solo dati già nel repository: `docs/letteratura_cc/` (`siti_sperimentali.csv`, `coppie_isomeri_rt.csv`, `tp_noti/tp_noti.csv`) e la tabella della bozza CC-05 (`docs/bozze/teoria_cc05.md`). Nessun dato di laboratorio.

- `molecules`: nome → SMILES. `atoms` di una riga = atomi pesanti da 0 nell'ordine dello SMILES (come `heavy_index` di `reatt/1`).
- `rows`: `reaction` (`OH`, `O3`, `metabolism`), `scope` (`ch` = sito C–H insaturo, deve stare in `ch_sites`; `atom` = qualunque atomo pesante), `tier` (`literatura` = DOI per riga; `bozza` = regiochimica della bozza CC-05, senza DOI), `confidence`, `source`.
- `baseline`: conteggi top-1/3/5 per `reazione/tier` misurati col motore attuale; la prova li confronta uno a uno. Se il motore cambia di proposito: `cargo test --release -p mzlab-chem --test validazione -- --nocapture`, riga `BASELINE`, e aggiorna il campo.
- Escluse (sito non ricavabile): paracetamolo di `tp_noti` (nome del prodotto incoerente), trasformazioni senza posizione (timololo, atenololo, terbutilazina), triclosan «C6» (descrizione incoerente), cloro (nessun dato di sito nel repository).
