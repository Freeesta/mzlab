# «Il file è troppo grande»: quando succede (misure)

Macchina: contenitore Linux 4 vCPU, 16 GB, Chromium 1243 senza interfaccia, profilo persistente (OPFS pieno); budget Automatico 1024 MB sicuro / 3072 alto, sonda 3072. `--js-flags=--wasm-max-mem-pages` imita un browser che concede meno a ogni istanza. File sintetici (`tools/dati_sintetici.py` e generatori del `scratchpad`, nessun dato vero).

| Ingresso | Esito | Tempo | Renderer al picco | Motore (memoria lineare) |
|---|---|---|---|---|
| 1 mzML DDA 1,0 GB, 150 000 scansioni | caricato | 23 s | 606 MB | 117 MB |
| 1 mzML a profilo 1,4 GB, 5000 scansioni | caricato | 33 s | 776 MB | 32 MB |
| 8 mzML da 204 MB, 240 000 scansioni | caricato | 18 s | 1,36-1,6 GB | 266 MB |
| gli stessi con 1024 / 512 / 256 MB per istanza | caricati (budget 768 / 256 / 192) | | | |
| 9 file (8 × 204 MB + 1 GB, 374 000 scansioni, 2,6 GB), 256 MB per istanza | prima della correzione: l'avviso «Il file è troppo grande…» senza nome, la riga «indice incompleto o danneggiato», l'indice cancellato | | | |
| gli stessi, 192 MB per istanza | prima: il motore finisce la memoria (trappola, «Il motore ha finito la memoria…»), 3 file non si aprono | | | |
| 9 file, 256 MB per istanza, dopo la correzione | caricati tutti e 9 | 32 s | 1,46 GB | 226 MB |
| 9 file, 192 MB per istanza, dopo la correzione | 8 caricati; il file da 1 GB ha «…accanto agli 8 già aperti (240 000 scansioni)…», indice tenuto, nessuna trappola | 32 s | 1,44 GB | 178 MB |

- Costruire l'indice non è il limite: costruttore a budget + scratch in OPFS reggono 1,4 GB con il motore a 32 MB.
- Il limite è il motore della pagina: tiene la tabella delle scansioni di OGNI file aperto (~1,07 KB per scansione DDA, fuori dal budget della cache) più la cache (`min(5/6 budget, 512 MB)`). Quando una prenotazione di `Index::open` fallisce, prima era un `error.memoryBudget` nudo (nessuna fase, nessun file): `lrIndexOwn` lo prendeva per un indice guasto e lo cancellava, `rust-bridge.js` ripiegava sull'mzML intero e l'avviso diceva «file troppo grande».
- Ogni file stava in 3 slot del motore (lista, elemento della sessione, parte MS1/MS2): tre copie della stessa tabella. Ora `Index.scans` è un `Arc` e `Engine.share_index` apre gli altri slot con `Index::over` sullo stesso `.mzidx` (stesso timbro dimensione+data di `createOpenGate`): una tabella per file (stima per 240 000 scansioni DDA: da ~770 a ~260 MB; il rifiuto di prima contava 24 slot e 720 000 scansioni per 8 file).
- Ora (`routes_open::open_beside`, motore del sito e app desktop): se fallisce, la cache degli altri file si svuota e l'apertura si riprova una volta; se fallisce ancora, il rifiuto è `error.memoryBudget` fase `openFiles` con `aperti` e `scansioni` («Rimuovi alcuni file o caricane meno insieme, oppure usa l'app desktop»). L'indice resta, non si ricostruisce e non si riapre l'mzML intero.
- Formati letti per intero (limite fisso `Limits::sito().byte` 192 MB, `desktop()` 1024 MB, non segue il budget): contenitore `.wiff` e lettura del metodo (`WIFF_MAX`), Shimadzu `.lcd`/`.qgd` (contenitore + flusso: oltre ~96 MB), Agilent `MSScan.bin`/`data.ms` (somma dei membri), indici Waters, record Bruker. Oltre il limite: `error.memoryBudget` fase `whole` (`Limits::fits`, `fits_in`) con i MB, «aprilo con l'app desktop oppure convertilo prima in mzML»; prima era nudo e il pool degli indici dimezzava e riprovava inutilmente.
- Un `error.memoryBudget` nudo resta solo per una prenotazione fallita altrove (`try_vec`): testo «il browser non ha concesso altra memoria: chiudi le altre schede o alcuni file aperti, oppure usa l'app desktop».
- Decisioni aperte (proprietario, nessuna cambiata qui): far seguire `Limits::sito().byte` al budget scelto; contare nel budget le tabelle delle scansioni dei file aperti; la trappola del motore con 192 MB concessi.
