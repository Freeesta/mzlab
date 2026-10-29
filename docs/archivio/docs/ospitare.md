# Piano B: ospitare mzLab altrove

Oggi il sito è su GitHub Pages (`pages.yml`, a ogni push su `main`). Se GitHub Pages non fosse più disponibile, il sito è una cartella di file statici che si può mettere su qualunque servizio. Non c'è uno specchio attivo: questo è il procedimento, da seguire in 1-2 ore.

## 1. Costruire la cartella `site/`
La costruzione non è solo `tools/build_site.py`: il motore Rust/WASM, le librerie spettrali e l'indice dei sospetti si aggiungono con i passi di `.github/workflows/pages.yml`. La via più semplice è **eseguire quel workflow** (`workflow_dispatch` o un push su un ramo di prova) e scaricare l'artefatto del sito; in alternativa, ripetere a mano nello stesso ordine i passi `run:` del workflow (in `tools/` ci sono `build_site.py`, `wasm_opt.sh`, `build_feat_wasm.sh`, `librerie_pacchetti.py`, `genera_sospetti.py`). Serve Python ≥ 3.11, la toolchain di `rust-toolchain.toml` e la rete.

## 2. Controllare prima di pubblicare
- Nessun file sopra **25 MiB** (limite per file di Cloudflare Pages): `find site -size +25M`. Le librerie sono già divise in pezzi.
- Il sito apre senza errori in console da `python3 -m http.server -d site` e il service worker si registra.
- I file `.wasm` sono serviti come `application/wasm`.

## 3. Pubblicare
- **Cloudflare Pages** (gratuito): crea un progetto «Direct Upload» e carica `site/`, per esempio con `npx wrangler pages deploy site --project-name mzlab`.
- Altri ospiti statici (Netlify, un server dell'università) vanno bene se servono i file così come sono, via HTTPS.

## 4. Cambiare l'indirizzo
Se l'indirizzo cambia, la prima volta aggiorna: i collegamenti nel `README.md`, in `CITATION.cff` e in `.zenodo.json`. L'**app desktop** controlla gli aggiornamenti solo da `https://freeesta.github.io/mzlab/` (`crates/mzlab-desktop/src/update.rs`): per un nuovo indirizzo serve una nuova versione dell'app che accetti **sia il nuovo sia il vecchio**. I dati degli studenti salvati nel browser sono legati all'indirizzo: cambiando indirizzo non si ritrovano.

## 5. Dopo
Annunciare il nuovo indirizzo con una issue fissata e nella pagina del corso. L'archivio permanente del codice resta su Zenodo e Software Heritage.
