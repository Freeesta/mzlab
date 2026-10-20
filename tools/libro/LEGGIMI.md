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
- Raw HTML is never passed through: it is escaped. The only exception is the import-only fenced block ```` ```html ... ``` ````, copied verbatim and only in the imported chapters listed in `importati.txt` (any other file fails the build; the list only shrinks) (svg, simulators, existing equation markup); `<script>`, `<iframe>`, `on*=`, `javascript:` and similar stop the build.
- Import constructs: `::: duo` ... `:::` (side-by-side boxes; extra box kinds `[!lr]`, `[!hr]`, `[!oral]`), `{.lead}` / `{.note}` at the start of a paragraph, `# Title {kicker: Capitolo 8}`, `<!-- script: sim-misc.js -->` before the first heading.
- `--teoria` writes published chapters (Teoria head and scripts, no noindex); `tests/test_libro_generato.py` checks every `mzlab/web/teoria/src/NN-*.md` against the HTML next to it.
