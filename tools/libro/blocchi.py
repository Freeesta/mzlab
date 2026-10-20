"""Block parser of the Libro dialect (see LEGGIMI.md): headings, paragraphs, lists, tables, boxes, equations, figures, questions and
private blocks. It renders straight to the HTML of the existing Teoria components; anything it does not know raises LibroError."""
import re
from html import escape

from .errori import LibroError
from .grezzo import PARA_CLASS, Grezzo
from .inline import render
from .liste import ITEM, Liste
from .speciali import FIG, Speciali

HEAD = re.compile(r"^(#{1,3}) +(.+?)\s*$")
EXPLICIT_ID = re.compile(r"\s*\{#([\w-]+)\}\s*$")
KICKER = re.compile(r"\s*\{kicker: *([^}]+)\}\s*$")


def slug(s: str) -> str:
    import unicodedata
    s = unicodedata.normalize("NFD", s.lower())
    s = re.sub(r"[̀-ͯ]", "", s)
    return re.sub(r"^-|-$", "", re.sub(r"[^a-z0-9]+", "-", s))[:48]


class Parser(Speciali, Liste, Grezzo):
    def __init__(self, file: str, prefix: str, pubblico: bool):
        self.file, self.prefix, self.pubblico = file, prefix, pubblico
        self.ctx: dict = {"eq": {}, "fig": {}, "titoli": []}
        self.head: list[str] = []
        self.caption = None
        self.scripts: list[str] = []
        self.scripts_before: list[str] = []
        self.page_title = ""
        self.no_appname = False

    def err(self, line: int, msg: str):
        raise LibroError(self.file, line, msg)

    def inl(self, text: str, line: int) -> str:
        return render(text, self.file, line, self.ctx)

    def blocks(self, lines: list[str], base: int) -> str:
        out, i, n = [], 0, len(lines)
        while i < n:
            ln = lines[i]
            if not ln.strip():
                i += 1
                continue
            html, i = self.one(lines, i, base)
            out.append(html)
        return "\n".join(h for h in out if h)

    def one(self, lines: list[str], i: int, base: int):
        ln, at = lines[i], base + i
        if ln.startswith("<!--"):
            return self.comment(lines, i, base)
        m = HEAD.match(ln)
        if m:
            return self.heading(m, at), i + 1
        if ln.rstrip() == "```html":
            return self.raw(lines, i, base)
        if ln.rstrip() == "::: duo":
            return self.duo(lines, i, base)
        if ln.startswith("$$"):
            return self.equation(lines, i, base)
        if ln.startswith(">"):
            return self.box(lines, i, base)
        if ln.startswith("|"):
            return self.table(lines, i, base)
        if FIG.match(ln):
            return self.figure(FIG.match(ln), at), i + 1
        if ln.startswith("??? "):
            return self.question(lines, i, base)
        if ln.startswith("Tabella: "):
            self.caption = (ln[9:].strip(), at)
            return "", i + 1
        if ITEM.match(ln):
            return self.listing(lines, i, base)
        return self.paragraph(lines, i, base)

    def heading(self, m, at):
        level, text = len(m.group(1)), m.group(2)
        k = KICKER.search(text) if level == 1 else None
        kicker = k.group(1) if k else self.prefix
        text = text[:k.start()] if k else text
        e = EXPLICIT_ID.search(text)
        text = text[:e.start()] if e else text
        self.ctx["titoli"].append((level, text))
        h = self.inl(text, at)
        if level == 1:
            return f'<div class="kicker">{escape(kicker)}</div>\n<h1>{h}</h1>'
        hid = e.group(1) if e else (None if self.ctx.get("teoria") else slug(text))
        return f"<h{level}{f' id={chr(34)}{hid}{chr(34)}' if hid else ''}>{h}</h{level}>"

    def paragraph(self, lines, i, base):
        j = i
        while j < len(lines) and lines[j].strip() and (j == i or not (HEAD.match(lines[j]) or ITEM.match(lines[j]) or lines[j][:2] in ("$$", "> ", "??", "<!", "``", "::") or lines[j][0] == "|")):
            j += 1
        text = " ".join(x.strip() for x in lines[i:j])
        c = PARA_CLASS.match(text)
        cls = f' class="{c.group(1)}"' if c else ""
        return f"<p{cls}>{self.inl(text[c.end():] if c else text, base + i)}</p>", j
