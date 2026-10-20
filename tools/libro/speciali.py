"""Special blocks of the Libro dialect (equations, boxes, tables, figures, questions): a mixin of the block Parser in blocchi.py."""
import re
from html import escape

BOXES = {"nota": ("key", "Concetto chiave"), "attenzione": ("warn", "Errore frequente"), "lab": ("lab", "Esempio"),
         "formula": ("math", "Formula"), "esercizio": ("gym", "Esercizio")}
FIG = re.compile(r'^!\[([^\]]*)\]\(([^)\s"]+)(?: "([^"]*)")?\)\s*\{#fig:([\w-]+)\}\s*$')
EQEND = re.compile(r"\$\$\s*(?:\{#eq:([\w-]+)\})?\s*$")
FOLLOW = re.compile(r"^\{#eq:([\w-]+)\}\s*$")
NCLS = ' class="n"'
NUMCELL = re.compile(r"^[−+-]?\d[\d.,\s·⁰¹²³⁴⁵⁶⁷⁸⁹⁻]*$")


class Speciali:
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
        nxt = next((k for k in range(j + 1, min(j + 3, len(lines))) if lines[k].strip()), None)
        if not eid and nxt is not None and FOLLOW.match(lines[nxt]):
            eid, j = FOLLOW.match(lines[nxt]).group(1), nxt
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
            cells = [f"<td{NCLS if NUMCELL.match(c) else ''}>{self.inl(c, base + i + 2 + k)}</td>" for c in r]
            body.append("<tr>" + "".join(cells) + "</tr>")
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
