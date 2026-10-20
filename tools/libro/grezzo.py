"""Comments (page head, `script:` list, private blocks) and import-only constructs of the Libro dialect: a fenced ```html block copied verbatim (svg, simulators, the existing equation markup),
the ::: duo side-by-side boxes, and the {.lead}/{.note} paragraph classes. A mixin of the block Parser in blocchi.py."""
import re
from pathlib import Path

UNSAFE = re.compile(r"<\s*(script|iframe|object|embed|link|meta|base|form)\b|\son\w+\s*=|javascript:", re.I)
SCRIPT = re.compile(r"^script: *([\w-]+\.js)$")
IMPORTATI = Path(__file__).with_name("importati.txt")
PARA_CLASS = re.compile(r"^\{\.(lead|note)\} +")


def ammesso(file: str) -> bool:
    """True if `file` is listed in importati.txt (chapters imported from the old HTML); the list only shrinks."""
    nomi = {x.split("#")[0].strip() for x in IMPORTATI.read_text(encoding="utf-8").splitlines()}
    return file in nomi


class Grezzo:
    def raw(self, lines, i, base):
        if not ammesso(self.file):
            self.err(base + i, "a raw ```html block is only allowed in the imported chapters listed in tools/libro/importati.txt")
        j = next((k for k in range(i + 1, len(lines)) if lines[k].rstrip() == "```"), None)
        if j is None:
            self.err(base + i, "```html block without closing ```")
        body = "\n".join(lines[i + 1:j])
        bad = UNSAFE.search(body)
        if bad:
            self.err(base + i, f"raw HTML block contains {bad.group(0).strip()!r}, which is not allowed")
        return body, j + 1

    def duo(self, lines, i, base):
        j = next((k for k in range(i + 1, len(lines)) if lines[k].rstrip() == ":::"), None)
        if j is None:
            self.err(base + i, "::: duo without closing :::")
        return f'<div class="duo">{self.blocks(lines[i + 1:j], base + i + 1)}\n</div>', j + 1

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
            m = SCRIPT.match(ln[4:-3].strip())
            if m:
                self.scripts.append(m.group(1))
            else:
                self.head.append(ln[4:-3].strip())
        return "", i + 1
