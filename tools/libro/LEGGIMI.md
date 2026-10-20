# Libro MD dialect (tools/libro)

Converts one Markdown section per file into a Teoria-style HTML page. Pure Python, deterministic (same input, same bytes), MIT.
It reads and writes only the two folders given: `python3 tools/libro/cli.py --src <md folder> --out <html folder> [--pubblico]`.
Files ending `.appunti.md` / `.qa.md` (personal notes, Q&A) are never converted.

Supported (anything else stops the build with `file:line`):
- `#` (kicker + h1), `##`, `###` (id = slug); comments before the first heading go in the page head, `<!-- privato -->...<!-- /privato -->` is dropped with `--pubblico`.
- paragraphs, `-` and `1.` lists (an indented block inside an item is parsed as blocks), `**bold**`, `*italic*`, `` `code` ``, `$inline math$`.
- `[text](Other.md#x)` becomes `Other.html#x`; `[@key; @key]` citation; `[I]` hypothesis badge (a `--pubblico` build fails if any is left); `@eq:id`, `@fig:id` references.
- `$$ tex $$ {#eq:id}` numbered `(S1.k)` (prefix = file name up to the first `_` or `.`); TeX is kept escaped in `span.tex` for KaTeX in the browser.
- `> [!nota|attenzione|lab|formula|esercizio] optional title` boxes; `Tabella: caption` + pipe table; `![alt](img/x.svg "caption"){#fig:id}` (alt up to 125 characters); `??? question` + answer on the next line(s).
- Raw HTML is never passed through: it is escaped.
