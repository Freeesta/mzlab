# Libro (convertitore Markdown → HTML)

- `tools/libro/` (Python puro, MIT, deterministico) converte una sezione `.md` del Libro in una pagina HTML in stile Teoria; legge e scrive solo le cartelle date (`cli.py --src --out [--pubblico]`). Dialetto e costrutti ammessi: `tools/libro/LEGGIMI.md`; ciò che non conosce si ferma con `file:riga` (`LibroError`).
- `pagina.py` assembla la pagina, `blocchi.py` i blocchi, `inline.py` il testo corrente; l'HTML grezzo nel Markdown viene sempre escapato.
- `--pubblico` toglie i blocchi `<!-- privato -->` e rifiuta i `[I]` rimasti. I file `*.appunti.md` e `*.qa.md` non si convertono.
- Prova: `tests/test_libro.py` sulla fixture sintetica CC0 `tests/data/libro/` (HTML d'oro `S1_prova.html`: si rigenera chiamando `converti()` sul `.md`). Il testo vero del Libro sta solo nel repository privato `mzlab-dati`, mai in questo.

## Editor del Libro (pagina admin)
- `mzlab/web/libro/editor.html` (+ `editor.js`, `gh.js`, `vault.js`, `norm.js`, `editor.css`): editor visuale (Milkdown/Crepe + KaTeX, bundle in `mzlab/web/vendor/libro/`, costruito da `tools/libro/editor/build.mjs`) che legge e scrive le sezioni nel repository PRIVATO `Freeesta/mzlab-dati` (`Libro_Untargeted/`) con l'API GitHub. Il codice è pubblico, il testo del Libro non entra mai nel sito: sta solo in memoria. Interfaccia in inglese (esclusa da `controlla_i18n`), non nel menu, `noindex`.
- Token: fine-grained, solo `mzlab-dati`, solo «Contents: Read and write», scadenza 90 giorni; sigillato con passphrase (AES-GCM + PBKDF2, `localStorage`) o solo per la scheda (`sessionStorage`); mai in URL né in log. CSP propria: `script-src 'self'`, `connect-src https://api.github.com`.
- Salvataggio: un commit diretto su `main` del repository privato per file (PUT con lo sha letto): se qualcuno ha cambiato il file risponde 409, l'editor non sovrascrive e mostra il testo dell'utente. «History» = `GET /commits?path=`; caricare una versione vecchia non salva finché non si preme Save.
- Tre file per sezione: `X.md`, `X.appunti.md` («My notes»), `X.qa.md` («Questions & answers»); gli ultimi due non si convertono. `norm.js` toglie `\[`, `\~`, i punti elenco `*` e le righe vuote in eccesso scritti dall'editor. Equazioni: blocco `$$…$$` con `{#eq:id}` sulla riga dopo.
- Prove: `tests_e2e/e2e_libro_editor.py` (GitHub simulato), `tests/test_libro_editor.py` (CSP, normalizzazione, nessuna riga del Libro privato nel repository pubblico).
