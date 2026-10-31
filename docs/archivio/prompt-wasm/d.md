# WASM-D · isolamento tra origini (COOP/COEP) dal service worker
**Ramo**: `claude/isolamento-sw` da `origin/main`. **Dipende da**: WASM-A (`capacita.isolamento`) in `main`; decisione di Federico (§7 dello studio, punto 2). **Prima di**: E.
**Primo messaggio** (send_message a session_01NawSmneBkv8sosEHpoC8k4): «WASM-D: ricevuto».

1. **Ruolo**: DEV Sonnet, service worker e caricamento della pagina. Nessun calcolo, nessuna dipendenza (niente `coi-serviceworker` da npm: sono 15 righe nel nostro `sw.js`).
2. **Contesto**: GitHub Pages non manda `Cross-Origin-Opener-Policy`/`Cross-Origin-Embedder-Policy`, quindi `crossOriginIsolated` è falso e `SharedArrayBuffer` non esiste.
   - Misura (`docs/studi/wasm-potenza.md` §3b, `banco-browser.txt` `coi-sw.js`, server senza intestazioni): un service worker che aggiunge `COEP: require-corp`, `COOP: same-origin`, `CORP: same-origin` alle risposte della stessa origine rende la pagina isolata in Chromium 153, Firefox 157 e WebKit 27.2, con UNA ricarica alla prima visita (214-424 ms misurati); dopo, `new WebAssembly.Memory({shared: true})` e `SharedArrayBuffer` ci sono in tutti e tre.
   - Il nostro `sw.js` esiste già (rete prima, poi cache, `sw.js:20-30`): un solo service worker per scope, quindi le intestazioni vanno lì.
3. **Perimetro**: `mzlab/web/sw.js` (le tre intestazioni su ogni risposta della stessa origine, anche da cache), il punto di registrazione del service worker (ricarica una sola volta, con guardia in `sessionStorage` contro i giri infiniti), `tools/build_site.py` solo se serve; e2e `tests_e2e/e2e_isolamento.py`; una riga in `docs/agenti/dati.md` (trappole del service worker). Non toccare il motore, `capacita.js` oltre a leggere `isolamento`.
4. **Lavoro**:
   - Inventario delle risorse che `require-corp` può bloccare: Ketcher in iframe `sandbox` (stessa origine, va servito con COEP anche lui), `llm/trasporto.html` e le sue richieste ai fornitori (devono essere CORS: verificalo), immagini `blob:`/`data:`, worker `blob:`, link esterni (non toccati: sono navigazioni). Se una risorsa esterna non ha CORS/CORP, fermati e scrivi al Coordinatore (2 opzioni).
   - Prima visita: la pagina parte subito NON isolata (tutto funziona come oggi), il service worker si installa, la ricarica avviene una sola volta al primo momento senza lavoro in corso (nessun file aperto); mai durante una conversione.
   - Firefox in navigazione privata e ogni browser con service worker spenti restano non isolati: `capacita.isolamento = false`, e si disattiva solo ciò che lo richiede (oggi niente; poi WASM-E).
5. **Accettazione**:
   - `e2e_isolamento.py` (Chromium in CI): da server senza intestazioni, dopo la ricarica `crossOriginIsolated === true`, Dati apre un file d'esempio, Ketcher si apre, nessun errore COEP in console, `window.MZLAB_LR_MISS` vuoto; con `?cap-off=isolamento` nessuna ricarica.
   - Ricarica della prima visita misurata nella PR (riferimento: `python3 run.py chromium "coi=1&only=parallel"` di `banco-browser.txt`, `coi_reload_ms` = 424 ms in Chromium 153).
   - In locale anche Firefox e WebKit (Playwright in una cartella temporanea, come nello studio §2): stesso esito.
   - `e2e_tour.py`, `tools/pre_pr.py` verdi.
6. **Prove**: `python3 tools/build_site.py && python3 tests_e2e/e2e_isolamento.py`, `python3 tools/verifica.py --cambiati`.
7. **Consegna**: PR aperta su `claude/isolamento-sw`; a CI verde UN messaggio al Coordinatore (≤5 righe: PR, SHA di 40 caratteri, cosa resta). Eventi della PR: skill `steward`.
