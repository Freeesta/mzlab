# Avvio rapido: interfaccia subito, il resto in sottofondo (regola per ogni pagina)

Vale per Dati, Teoria, mzFlow e Disegno (Tabui esclusa). Il dettaglio dell'avvio della vista Dati è in `avvio.md`.

- **Budget** a cache calda: primo paint ≤ 300 ms, interattivo ≤ 1 s; a cache fredda niente tetto, ma la shell compare comunque prima di qualunque `.wasm`.
- **Shell immediata**: HTML e CSS della pagina con segnaposto (scheletro dei pannelli, barra, titoli) e script classici piccoli; nessun `.wasm`, JSON di catalogo, modello o libreria pesante sul percorso che porta al primo paint e all'interattività.
- **Tutto il resto lazy**: moduli `.wasm`, cataloghi (`steps-cli-1.json`, `strumenti-1.json`), dati e script pesanti si caricano in sottofondo o al primo uso di ciò che li richiede.
  - `.wasm`: `WebAssembly.compileStreaming`/`instantiateStreaming` (risposta con `application/wasm`), istanza in un Web Worker dove il pattern esiste già (`rust-worker.js`), mai sul thread principale.
  - JSON grossi: `fetch` in parallelo, parse in worker o a idle; mai `fetch` in sequenza quando sono indipendenti.
  - Prefetch a idle (`requestIdleCallback`, con ripiego su `setTimeout`) di ciò che l'utente userà quasi di sicuro; mai prima del primo paint.
- **Indicatori sobri**: un segnaposto o una riga di stato (`role=status`) per ogni parte in arrivo, nessun velo che blocca; un controllo che dipende da una parte non ancora pronta resta disattivato con `aria-disabled` e si attiva a caricamento finito. `prefers-reduced-motion` rispettato.
- **Cache**: risorse con nome versionato o ETag, mai ricaricate se invariate; vale il contratto del service worker in `dati.md`.
- **Misura** con Playwright (Chromium già installato, mai `playwright install`), sul sito costruito (`python3 tools/build_site.py`, poi cancellare `site/`): `PerformanceObserver` per `paint` (first-contentful-paint), un marcatore `performance.mark` all'interattività della pagina, `performance.getEntriesByType("resource")` per la cascata (byte, ordine, durata); due giri, cache fredda (contesto nuovo) e calda (stesso contesto, secondo caricamento). Riporta la tabella prima/dopo nel corpo della PR.
- **Test e2e «shell prima dei wasm»**: la pagina espone i marcatori `shell` (shell visibile e interattiva) e il tempo di fine istanziazione di ogni `.wasm`; il test blocca le richieste `.wasm` (`page.route`) e verifica che la shell compaia e risponda alla tastiera, poi le sblocca e verifica che le parti lazy si attivino. Esempio: `tests_e2e/e2e_flow_avvio.py`.
