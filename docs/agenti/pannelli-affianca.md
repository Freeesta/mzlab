# Affiancare e spostare i pannelli (`affianca.js`, `AFFIANCA`)

- Maniglia `.aff-grip` nell'intestazione del pannello; trascinarla mostra l'anteprima del rettangolo finale senza ridisegnare; Esc annulla. Senza trascinare: menu della maniglia e Alt+frecce. `rifai()` applica una mossa a una pila (vera o copiata).
- Suggerimento «Trascina qui per affiancare» (`#aff-hint`, chiave `qqq.pannelli.affianca.hint`): una sola volta per browser, con almeno due pannelli e spazio, quando nessun altro fumetto è aperto (`SUGG`); × ed Esc lo chiudono, il primo affiancamento lo chiude per sempre.
- Le stesse mosse della maniglia stanno nel menu del tasto destro e nella paletta (azioni `disp.*`, `docs/agenti/azioni.md`); `disp.ripristina` rimette ogni pannello a tutta larghezza (si annulla con Ctrl+Z); `layout.js` non nomina mai un esperimento (guardia in `controlla.py`).
- Zone di rilascio a croce sul pannello bersaglio (`zona()`): 30% in alto = «Sopra» (riga propria prima del bersaglio), 30% in basso = «Sotto», fascia centrale metà sinistra / metà destra = «A sinistra» / «A destra»; «Spazio insufficiente» se non entra. Ogni zona ha la sua etichetta nell'anteprima.
- Una cromatografia trascina con sé i suoi spettri figli; rilasciata sul proprio spettro (o con Alt+freccia verso di esso) si sposta da sola oltre di lui (`solo` nella mossa).
- Divisori `.aff-sep` (`role=separator`, frecce ±16, Maiusc ±48, Invio = uguali) rifatti da `sync()` in `fitHost`. Annulla con `HSEQ` (Ctrl/⌘+Z) e barra di 6 s.
- Prove: `e2e_affianca`, `e2e_affianca_zone`.
