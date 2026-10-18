# Avvio progressivo (stato attuale)

- **Pagina subito visibile**: header, `#start` e `#drop` si vedono senza aspettare il motore; `#loading` (velo, frasi, ione) è nascosto all'avvio e resta SOLO per l'apertura dei file (`loading()` in `explore.js`).
- **`AVVIO`** (`mzlab/web/avvio-progressivo.js`, caricato prima di `carica.js`): stato `pending | ready | failed`, una sola transizione fuori da `pending`. `pronto()` è il cancello di `upload()` in `shell.js` (`true` pronto, `false` fallito o annullato); i drop prima del motore restano in coda e partono in ordine. Senza `browser.js` o su telefono nasce `ready`.
- **Riga di stato** `#avvio-fase` (`role=status`) sotto la barra: fase d'avvio (`qqPhaseCur`); dopo 10 s di `pending` compare «Reset completo» (`qqHardReset`); sparisce a motore pronto o fallito.
- **Sessione salvata**: l'indizio sincrono `mzlab.sessione` (`NOMI.KEY`) mostra lo scheletro `#drop-skeleton` al posto di `#drop` finché `api/state` risponde (`segnaSessione`, `applicaIndizioSessione`, `rimuoviScheletro`); `reset.js` lo azzera.
- **Errore d'avvio**: `fail(key, params)` di `browser.js` imposta `qqFailed`, lascia `{key, params}` in `qqFailInfo` (ripreso da `AVVIO` se il modulo non c'era ancora) e chiama `AVVIO.segnaFallito`. 
  Questo toglie scheletro e riga di stato e mostra il banner `#avvio-errore` (fisso in alto, `role=alert`, non modale): messaggio, «Copia dettagli» (versione, chiave, fase, testo tecnico senza percorsi né URL, nessun nome di file) e «Reset completo». 
  `AVVIO.errore` e il testo del banner sono la stessa stringa; chiave sconosciuta o catalogo assente → «Calculation engine startup failed».
  Una stringa è una chiave solo se ha forma `a.b.c` senza spazi.
- **`#drop` dopo un fallimento** resta visibile solo per mostrare l'errore: `upload()` non accoda e non lancia (`pronto()` → `false`).
- **Casi** (messaggi `load.err.*`): browser vecchio `oldBrowser`; motore assente o WASM che non scarica `engineMissing`/`engineStart`; archivio bloccato da un'altra scheda `load.blocked` (riga di stato) poi `resumeTimeout` (banner); `?riparti=1` cancella i file salvati prima del riavvio.
- **Test**: `tests/test_avvio_progressivo.py`, `tests/test_avvio_fase3.py` (Node con DOM finto), `tests_e2e/e2e_avvio.py`, `e2e_avvio_lingua.py`.
