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
- `<!-- html: data-esempio=atrazina -->` puts a `data-*` attribute on the `<html>` tag (`--teoria`).
- `<!-- full-title: Formulario · Teoria mzLab -->` sets the whole `<title>` of a chapter whose title does not end with "· Teoria QqQ lab" (`--teoria`).
- `<!-- esteso -->` makes a hidden draft chapter (`data-esteso`, noindex, header comment `chapter: N Title` plus the other head comments such as `source-card:`, `verification: draft`, `exam-level:`).
- `<!-- inline: modi -->` / `<!-- inline: formulario-quiz -->` add a fixed script snippet from `tools/libro/snippets/` (names listed in `snippet.py`; `before` ones go ahead of `../appname.js`, `after` ones at the end); any other name is an error and the Markdown never holds JavaScript.
- Inline for imported chapters: `_em_`, `~sub~`, `^sup^` (no spaces inside); external `https` links carry `target="_blank" rel="noopener"` in `--teoria`; `## Title {#id}` sets an id (in `--teoria` headings get none otherwise); `<!-- title: X -->` overrides the page title; `<!-- script-before: ../elements.js -->` adds a script before `teoria.js` (`script:` after it); `<!-- no-appname -->` leaves out `../appname.js` (a chapter that never loaded it).
- `python3 tools/libro/importa.py --html <chapter.html> --out mzlab/web/teoria/src/NN-slug.md` imports a published chapter once: each block becomes dialect Markdown only if the canonical DOM (`dom.py`) is unchanged, else a raw `html` block; it refuses to write a page that is not equivalent. `tests/data/libro/equivalenza.txt` keeps the digest of each imported chapter as it was.
