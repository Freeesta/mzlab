# Intestazione e tabella di caricamento

- **Schermo intero**: `#fs-btn` nell'header (Fullscreen API sull'intera app, `aria-pressed`, testi `header.fullscreen*`); nascosto dove il browser non ha l'API (iPhone).
- **Finestra «quasi a schermo intero» movibile**: sul web non esiste un modo per averla senza la barra delle schede. `window.open` con `popup`/`width`/`height` apre una finestra con la barra del browser.
  Document Picture-in-Picture accetta solo contenuto già in pagina, non l'app intera. La Window Management API (`getScreenDetails`) è solo Chromium, con permesso, e non toglie la barra. Solo la Fullscreen API la toglie.
- **Logo**: a ogni clic (`tpmine-loader.js`, `popLogo`) si ingrandisce e «esplode» con la Web Animations API, poi torna; nessuna animazione con `prefers-reduced-motion` o `data-a11y-reduce`. Cinque clic veloci accendono/spengono mzFinder.
- **Pulsante (i)** (`help.header.body`): gratuito, open source, nessun dato caricato, versione in lavorazione, indirizzo per suggerimenti. «Altro» apre PubChem e BioTransformer in un'altra scheda, senza riquadri.
- **Area di caricamento**: il testo `load.drop` dice «.mzML»; con il motore Rust (`convertRaw`) passa a `load.dropRaw` («.mzML o .raw»). L'area dei metodi ripete «clicca per sceglierlo dal computer» sottolineato (`load.method.dropHint`). «Scarica mzML» e la × stanno su una riga (`white-space:nowrap`).
- **Tipo di esperimento nella tabella** (`mixedText`, `expCell` in `index.html`): un file con più parti mostra i nomi delle parti uniti da « + », senza polarità né numero di precursori; Full Scan + MS2 = «Full Scan + DDA»; la sola polarità alternata è un «Full Scan». Prova: `tests_e2e/e2e_misti.py`.
