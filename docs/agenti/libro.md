# Libro (convertitore Markdown → HTML)

- `tools/libro/` (Python puro, MIT, deterministico) converte una sezione `.md` del Libro in una pagina HTML in stile Teoria; legge e scrive solo le cartelle date (`cli.py --src --out [--pubblico]`). Dialetto e costrutti ammessi: `tools/libro/LEGGIMI.md`; ciò che non conosce si ferma con `file:riga` (`LibroError`).
- `pagina.py` assembla la pagina, `blocchi.py` i blocchi, `inline.py` il testo corrente; l'HTML grezzo nel Markdown viene sempre escapato.
- `--pubblico` toglie i blocchi `<!-- privato -->` e rifiuta i `[I]` rimasti. I file `*.appunti.md` e `*.qa.md` non si convertono.
- Prova: `tests/test_libro.py` sulla fixture sintetica CC0 `tests/data/libro/` (HTML d'oro `S1_prova.html`: si rigenera chiamando `converti()` sul `.md`). Il testo vero del Libro sta solo nel repository privato `mzlab-dati`, mai in questo.
