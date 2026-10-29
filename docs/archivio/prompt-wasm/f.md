# WASM-F · disegno dei grafici pesanti in un worker (OffscreenCanvas)
**Ramo**: `claude/offscreen-grafici` da `origin/main`. **Dipende da**: WASM-A in `main`. **Prima di**: nulla. Priorità 2: parte solo se la misura del primo passo supera la soglia.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-F: ricevuto».

1. **Ruolo**: DEV Sonnet, grafici della pagina. Nessun calcolo nuovo (i dati restano quelli del motore).
2. **Contesto**: tutti i canvas si disegnano nel thread principale (`grafici.js`, `mappa.js`, `xic-lente.js`, `explore.js`).
   - Misura (`banco-browser.txt`, `offscreen`: 20 ridisegni di una spezzata da 200 000 punti, 1200×400): thread principale bloccato 121 / 910 / 407 ms (Chromium 153 / Firefox 157 / WebKit 27.2);
   - stesso disegno in un worker con `transferControlToOffscreen`: il thread principale resta libero (pausa massima 13 / 56 / 234 ms; in WebKit il guadagno è piccolo).
   - `OffscreenCanvas` 2D nei worker c'è nei tre browser; WebGL2 nei worker no in Firefox e WebKit di prova (la vista 3D resta nel thread principale).
3. **Perimetro**: un modulo nuovo `mzlab/web/grafico-worker.js` (`// @ts-check`) e il grafico che la misura indica come il più lento; `capacita.offscreen2d`; e2e del grafico toccato; una riga in `docs/agenti/dati.md`. Non toccare il motore, `mappa3d.js`, la tabella dei dati `TDATI` (resta nel DOM, accessibile).
4. **Lavoro**:
   - Primo passo: con `tools/bench_mappa.py` e un Performance trace di Chromium misura i compiti lunghi (> 50 ms) del thread principale durante zoom e spostamento su mappa, TIC e XIC con il file HR più grande dei dati sintetici. Se nessun disegno supera 50 ms, fermati e manda i numeri al Coordinatore.
   - Solo per i grafici sopra soglia: disegno nel worker, la pagina tiene assi, etichette, mirino e tabella dei dati (accessibilità invariata: `aria-label`, tastiera, colori da variabili CSS passati al worker al cambio di tema).
   - Senza `capacita.offscreen2d` il grafico non si disegna e mostra l'avviso comune (nessun secondo disegnatore nel thread principale): da confermare con Federico nella PR se la misura lo giustifica.
5. **Accettazione**:
   - Immagini identiche pixel per pixel prima/dopo (screenshot dell'e2e, stessi colori Okabe-Ito); `e2e_axe_grafici` verde.
   - Compito lungo massimo del thread principale durante zoom sul grafico spostato sceso sotto 50 ms in Chromium (numeri prima/dopo nella PR).
   - `e2e_tour.py`, `tools/pre_pr.py` verdi.
6. **Prove**: `python3 tools/bench_mappa.py`, `python3 tools/verifica.py --cambiati`.
7. **Consegna**: PR aperta su `claude/offscreen-grafici`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
