"""Block parser of the Libro dialect (see LEGGIMI.md): headings, paragraphs, lists, tables, boxes, equations, figures, questions and
private blocks. It renders straight to the HTML of the existing Teoria components; anything it does not know raises LibroError."""
import re
from html import escape

from .errori import LibroError
from .inline import render
from .liste import ITEM, Liste
from .speciali import FIG, Speciali

HEAD = re.compile(r"^(#{1,3}) +(.+?)\s*$")


def slug(s: str) -> str:
    import unicodedata
    s = unicodedata.normalize("NFD", s.lower())
    s = re.sub(r"[̀-ͯ]", "", s)
    return re.sub(r"^-|-$", "", re.sub(r"[^a-z0-9]+", "-", s))[:48]


class Parser(Speciali, Liste):
    def __init__(self, file: str, prefix: str, pubblico: bool):
        self.file, self.prefix, self.pubblico = file, prefix, pubblico
        self.ctx: dict = {"eq": {}, "fig": {}, "titoli": []}
        self.head: list[str] = []
        self.caption = None

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

    def comment(self, lines, i, base):
        ln = lines[i].strip()
        if ln == "<!-- privato -->":
            j = next((k for k in range(i + 1, len(lines)) if lines[k].strip() == "<!-- /privato -->"), None)
            if j is None:
                self.err(base + i, "<!-- privato --> without closing <!-- /privato -->")
            inner = "" if self.pubblico else f'<div class="privato" data-privato>\n{self.blocks(lines[i + 1:j], base + i + 1)}\n</div>'
            return inner, j + 1
        if not ln.endswith("-->"):
            self.err(base + i, "multi-line HTML comments are not supported")
        if not self.ctx["titoli"]:
            self.head.append(ln[4:-3].strip())
        return "", i + 1

    def heading(self, m, at):
        level, text = len(m.group(1)), m.group(2)
        self.ctx["titoli"].append((level, text))
        h = self.inl(text, at)
        if level == 1:
            return f'<div class="kicker">{escape(self.prefix)}</div>\n<h1>{h}</h1>'
        return f'<h{level} id="{slug(text)}">{h}</h{level}>'

    def paragraph(self, lines, i, base):
        j = i
        while j < len(lines) and lines[j].strip() and (j == i or not (HEAD.match(lines[j]) or ITEM.match(lines[j]) or lines[j][:2] in ("$$", "> ", "??", "<!") or lines[j][0] == "|")):
            j += 1
        return f"<p>{self.inl(' '.join(x.strip() for x in lines[i:j]), base + i)}</p>", j
