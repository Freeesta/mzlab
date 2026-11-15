# TIC provvisorio interattivo (zoom e spostamento dell'asse RT)

Il TIC provvisorio (`tic-provvisorio.js`, `caricamento-mappa.md` passo 6) si disegna mentre l'indice si costruisce. Decisione del proprietario richiesta sulla PR: prima era un'anteprima ferma per scelta.

- **Comandi** su `#ticprov-cv` (solo asse RT; l'altezza segue la finestra): Ctrl/Cmd + rotella (una rotella semplice scorre la pagina, come sui pannelli), trascinamento, `+` `-`, frecce, `0` e doppio clic = tutta la traccia. Un disegno per fotogramma (`FRAME.next`); disegno a 4 vertici per colonna (`decima.js` `polyline`). Una riga `role=status` dice la finestra a gesto fermo. Un clic ripete la nota di avanzamento (nessuno spettro né XIC prima dell'indice). `touch-action: pan-y pinch-zoom` solo sul canvas; nessuna animazione, quindi `prefers-reduced-motion` è rispettato.
- **Passaggio al pannello**: `TICPROV.handover()` (una volta) è chiamato da `addPanel` (`pannelli.js`) per il primo cromatogramma senza `zoom`: il pannello si apre su quella finestra e un avviso lo dice (scelta automatica registrata); gli altri pannelli della vista la seguono col collegamento dei tempi.
- **Prova**: `tests_e2e/e2e_tic_provvisorio.py` (rotella, trascinamento, doppio clic, tastiera, finestra del primo pannello); servono il sito e il `.wasm`.
- **Desktop**: l'app non ha un TIC provvisorio (lettura nativa diretta, pannello subito): nessuna parte gemella.
- **Misura**: non fatta (manca il `.wasm` nell'ambiente della PR); vedi la riga in `ottimizzazioni-tentate.md`.
