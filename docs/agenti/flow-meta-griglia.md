# Griglia dei metadati dei campioni (Flow, `flow/lazy/flow-meta-grid.js`)

Solo amministratore e solo inglese. Una griglia ARIA a righe virtuali (`role=grid`, `aria-rowcount`, `aria-colindex`; righe di 28 px, finestra di circa 13 righe più 4 di margine, la riga attiva esiste sempre) sopra un documento `campioni/3` (`campioni.md`). Tutta la logica è Rust, raggiunto con `CDOC.post("<op>")` (`api/campioni/<op>`): il file dispone le celle e muove il fuoco.

## Documento e salvataggio
- `F.metaGrid(S, input)` (`F.samplesEditor` lo chiama): il documento sta in `S.samples[input.name]`; un `/2` si migra con `migra`. Senza documento e con file aperti in Dati si apre da sola: `F.samplesFromData` → `migra` → `dal_file` con polarità, intervalli, scansioni e strumento letti da `E.files`.
- Ogni modifica si vede subito; 120 ms dopo `F.samplesSet` calcola lo SHA-256 canonico (`CDOC.canonical`, in `input.sha256`) e `valida` ridisegna gli avvisi. Il documento salvato ha quindi lo stesso SHA di `CDOC.canonical`.
- `F.metaImport(S, input, testo)`: `csv_leggi` (`,` `;` tab; `F.META_HEADERS` rinomina «Time (min)», «Series»…) e fusione per file+campione o SHA-256: le righe note si sovrascrivono tenendo i dati del file, le nuove si aggiungono, le colonne libere si uniscono.

## Colonne
- Fisse e modificabili: `file`, `campione`, `campione_id`, `tipo`, `matrice`, `serie`, `gruppo`, `tempo`, `unita_tempo`, `replica`, `lotto`, `concentrazione`, `unita`, `note`; poi le colonne libere di `colonne` (testo, numero, categoria), poi in sola lettura polarità, re-iniezione, acquisito, strumento, intervallo m/z e RT, scansioni.
- «Columns…»: aggiungi, rinomina (le `extra` seguono), cambia tipo e valori ammessi, obbligatoria, elimina. Un valore che non rispetta il tipo non si rifiuta: `valida` lo segna (`typeMismatch`).

## Tastiera e mouse
- Frecce, Home/End (Ctrl = prima/ultima riga), PagSu/PagGiù spostano; Maiusc estende l'intervallo; Invio, F2 o un carattere aprono la cella (un elenco prende la prima voce che comincia con la lettera); Invio conferma e scende, Tab conferma e va a destra, Esc annulla e torna alla cella; Ctrl+D riempie in giù (riga sopra, o la prima dell'intervallo); Canc svuota; Ctrl+C copia in TSV; Ctrl+V incolla un blocco TSV da Excel dalla cella attiva (celle in sola lettura, fuori dalla griglia o non valide si contano e si dicono). Tab da fuori entra nella cella attiva; nessuna trappola.
- Ordina con l'intestazione (`aria-sort`), filtra con colonna + testo (chip rimovibili), raggruppa per matrice, gruppo o condizione (bordo e fondo alternato sui gruppi). Bersagli di almeno 28 px, colori solo da variabili CSS.

## Avvisi e Rust
- `valida` → avvisi per cella (`colonna`, `file` = indice di riga): segno `✖`/`⚠` più bordo, `aria-invalid` sugli errori, riga con coppia mancante (`pairMissing`) con bordo tratteggiato; la cella attiva scrive il suo avviso in un `role=status` e un secondo `role=status` riassume errori, avvisi e avvisi di serie.
- «Fill from name pattern…» (`modello`, predefinito `{campione_id}`): anteprima delle prime 8 righe con le parti catturate (`posizioni`) e i nomi senza corrispondenza; «Apply» scrive. «Read file data» (`dal_file`) e «Generate metadata» da Dati; «Export CSV» (`csv_scrivi`, separatore a scelta) e «Download JSON».

## Prove
`tests_e2e/e2e_flow_meta_grid.py` (240 righe, incolla 240×6 in meno di 300 ms, SHA, tastiera, ordine, filtro, colonna libera, modello, CSV, axe), `e2e_flow_import`, tipi `tools/tipi/pagina.json`. Il sito con il motore: `python3 tools/prepara_sito.py` (non guarda `flow/`: cancella `site/static/wasm/mzlab_wasm_bg.wasm` per riprepararlo).
