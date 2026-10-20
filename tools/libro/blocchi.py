"""Block parser of the Libro dialect (see LEGGIMI.md): headings, paragraphs, lists, tables, boxes, equations, figures, questions and
private blocks. It renders straight to the HTML of the existing Teoria components; anything it does not know raises LibroError."""
import re
from html import escape

from .errori import LibroError
from .inline import render

BOXES = {"nota": ("key", "Concetto chiave"), "attenzione": ("warn", "Errore frequente"), "lab": ("lab", "Esempio"),
         "formula": ("math", "Formula"), "esercizio": ("gym", "Esercizio")}
HEAD = re.compile(r"^(#{1,3}) +(.+?)\s*$")
ITEM = re.compile(r"^([-*]|\d+\.) +(.*)$")
FIG = re.compile(r'^!\[([^\]]*)\]\(([^)\s"]+)(?: "([^"]*)")?\)\s*\{#fig:([\w-]+)\}\s*$')
EQEND = re.compile(r"\$\$\s*(?:\{#eq:([\w-]+)\})?\s*$")
NUMCELL = re.compile(r"^[−+-]?\d[\d.,\s·⁰¹²³⁴⁵⁶⁷⁸⁹⁻]*$")


def slug(s: str) -> str:
    import unicodedata
    s = unicodedata.normalize("NFD", s.lower())
    s = re.sub(r"[̀-ͯ]", "", s)
    return re.sub(r"^-|-$", "", re.sub(r"[^a-z0-9]+", "-", s))[:48]


class Parser:
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

    def equation(self, lines, i, base):
        j = i
        while j < len(lines) and not (EQEND.search(lines[j]) and (j > i or len(lines[j].strip()) > 2)):
            j += 1
        if j >= len(lines):
            self.err(base + i, "equation $$ without closing $$")
        body = " ".join(lines[i:j + 1])
        m = EQEND.search(body)
        tex = body[2:m.start()].strip()
        eid = m.group(1)
        label = ""
        if eid:
            if eid in self.ctx["eq"]:
                self.err(base + i, f"duplicate equation id {eid}")
            self.ctx["eq"][eid] = f"{self.prefix}.{len(self.ctx['eq']) + 1}"
            label = f'<span class="no">({self.ctx["eq"][eid]})</span>'
        idattr = f' id="eq-{eid}"' if eid else ""
        return f'<span class="eq"{idattr}><span class="tex">{escape(tex)}</span>{label}</span>', j + 1

    def box(self, lines, i, base):
        j = i
        while j < len(lines) and lines[j].startswith(">"):
            j += 1
        inner = [re.sub(r"^> ?", "", x) for x in lines[i:j]]
        m = re.match(r"^\[!(\w+)\]\s*(.*)$", inner[0])
        if not m or m.group(1) not in BOXES:
            self.err(base + i, "a quote must start with [!nota], [!attenzione], [!lab], [!formula] or [!esercizio]")
        cls, default = BOXES[m.group(1)]
        title = self.inl(m.group(2), base + i) if m.group(2) else default
        return f'<div class="box {cls}"><b>{title}</b>\n{self.blocks(inner[1:], base + i + 1)}\n</div>', j

    def table(self, lines, i, base):
        j = i
        while j < len(lines) and lines[j].startswith("|"):
            j += 1
        rows = [[c.strip() for c in x.strip().strip("|").split("|")] for x in lines[i:j]]
        if len(rows) < 3 or not all(re.fullmatch(r":?-+:?", c) for c in rows[1]):
            self.err(base + i, "a table needs a header row and a |---| separator row")
        if not self.caption:
            self.err(base + i, "a table needs a caption: put 'Tabella: ...' on the line before")
        cap, _ = self.caption
        self.caption = None
        head = "".join(f'<th scope="col">{self.inl(c, base + i)}</th>' for c in rows[0])
        body = []
        for k, r in enumerate(rows[2:]):
            if len(r) != len(rows[0]):
                self.err(base + i + 2 + k, f"row has {len(r)} cells, header has {len(rows[0])}")
            body.append("<tr>" + "".join(f'<td{" class=\"n\"" if NUMCELL.match(c) else ""}>{self.inl(c, base + i + 2 + k)}</td>' for c in r) + "</tr>")
        return f'<table>\n<caption>{self.inl(cap, base + i)}</caption>\n<tr>{head}</tr>\n' + "\n".join(body) + "\n</table>", j

    def figure(self, m, at):
        alt, src, cap, fid = m.groups()
        if not alt or len(alt) > 125:
            self.err(at, "figure alt text is required and must be at most 125 characters")
        if not re.fullmatch(r"[\w./-]+", src):
            self.err(at, f"unsupported figure path {src!r}")
        num = f"{self.prefix}.{len(self.ctx['fig']) + 1}"
        self.ctx["fig"][fid] = num
        return (f'<figure id="fig-{fid}">\n<img src="{src}" alt="{escape(alt)}">\n'
                f'<figcaption><b>Figura {num}</b> {self.inl(cap or "", at)}</figcaption>\n</figure>')

    def question(self, lines, i, base):
        j = i + 1
        while j < len(lines) and lines[j].strip():
            j += 1
        if j == i + 1:
            self.err(base + i, "a question '??? ...' needs the answer on the next line")
        ans = self.inl(" ".join(x.strip() for x in lines[i + 1:j]), base + i + 1)
        return f'<details class="q"><summary>{self.inl(lines[i][4:].strip(), base + i)}</summary><p>{ans}</p></details>', j

    def listing(self, lines, i, base):
        tag = "ol" if lines[i][0].isdigit() else "ul"
        items, j = [], i
        while j < len(lines):
            m = ITEM.match(lines[j])
            if not m or m.group(1)[0].isdigit() != (tag == "ol"):
                break
            body, k = [m.group(2)], j + 1
            while k < len(lines) and (lines[k].startswith("  ") or (not lines[k].strip() and self.indented_next(lines, k))):
                body.append(lines[k][3:] if lines[k].startswith("   ") else lines[k].strip())
                k += 1
            items.append((body, j))
            j = k
            while j < len(lines) and not lines[j].strip() and j + 1 < len(lines) and ITEM.match(lines[j + 1]):
                j += 1
        return f"<{tag}>\n" + "\n".join(self.item(b, base + s) for b, s in items) + f"\n</{tag}>", j

    @staticmethod
    def indented_next(lines, k):
        nxt = next((x for x in lines[k + 1:] if x.strip()), "")
        return nxt.startswith("  ")

    def item(self, body, at):
        inner = self.blocks(body, at)
        simple = inner.startswith("<p>") and inner.count("<p>") == 1
        return f"<li>{inner[3:-4] if simple else inner}</li>"

    def paragraph(self, lines, i, base):
        j = i
        while j < len(lines) and lines[j].strip() and (j == i or not (HEAD.match(lines[j]) or ITEM.match(lines[j]) or lines[j][:2] in ("$$", "> ", "??", "<!") or lines[j][0] == "|")):
            j += 1
        return f"<p>{self.inl(' '.join(x.strip() for x in lines[i:j]), base + i)}</p>", j
