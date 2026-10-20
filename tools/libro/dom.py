"""Small tolerant HTML tree (implied end tags like p/li/td, source offsets) and a canonical form used to prove that two pages are the same
document: tags and sorted attributes, whitespace collapsed, comments dropped. Used by importa.py and tests/test_libro_importa.py."""
import re
from html.parser import HTMLParser

VOID = {"meta", "link", "br", "img", "hr", "input", "source", "col", "wbr"}
BLOCK = {"p", "div", "ul", "ol", "li", "table", "tr", "td", "th", "thead", "tbody", "caption", "h1", "h2", "h3", "h4", "h5", "h6", "details", "summary",
         "figure", "figcaption", "section", "main", "nav", "body", "head", "html", "title", "br", "svg", "style", "script", "meta", "link"}
CLOSES_P = {"p", "div", "ul", "ol", "table", "h1", "h2", "h3", "h4", "h5", "h6", "details", "figure", "li", "tr", "td", "th", "thead", "tbody"}
CELL = {"td", "th"}


class Node:
    def __init__(self, tag, attrs, start):
        self.tag, self.attrs, self.start, self.end, self.kids = tag, dict(attrs), start, start, []

    def text(self):
        return "".join(k.text() if isinstance(k, Node) else k for k in self.kids)


class _Builder(HTMLParser):
    def __init__(self, src):
        super().__init__(convert_charrefs=True)
        self.src, self.root = src, Node("#root", {}, 0)
        self.stack = [self.root]
        self.lines = [0]
        for m in re.finditer("\n", src):
            self.lines.append(m.end())

    def here(self):
        line, col = self.getpos()
        return self.lines[line - 1] + col

    def close(self, node, at):
        node.end = at
        self.stack.pop()

    def pop_until(self, tags, stop, at):
        """Close open elements named in `tags` that sit above the nearest element named in `stop`."""
        for k in range(len(self.stack) - 1, 0, -1):
            t = self.stack[k].tag
            if t in stop:
                return
            if t in tags:
                while len(self.stack) > k:
                    self.close(self.stack[-1], at)
                return

    def handle_starttag(self, tag, attrs):
        at = self.here()
        if tag in CLOSES_P:
            self.pop_until({"p"}, {"div", "li", "td", "th", "details", "figure", "body", "main"}, at)
        if tag == "li":
            self.pop_until({"li"}, {"ul", "ol"}, at)
        if tag in CELL:
            self.pop_until(CELL, {"tr", "table"}, at)
        if tag == "tr":
            self.pop_until({"tr", "td", "th"}, {"table", "thead", "tbody"}, at)
        if tag in ("thead", "tbody"):
            self.pop_until({"tr", "td", "th", "thead", "tbody"}, {"table"}, at)
        node = Node(tag, attrs, at)
        self.stack[-1].kids.append(node)
        if tag in VOID:
            node.end = at + len(self.get_starttag_text())
        else:
            self.stack.append(node)

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if tag not in VOID:
            self.close(self.stack[-1], self.here() + len(self.get_starttag_text()))

    def handle_endtag(self, tag):
        at = self.here()
        for k in range(len(self.stack) - 1, 0, -1):
            if self.stack[k].tag == tag:
                end = self.src.find(">", at) + 1
                while len(self.stack) > k + 1:
                    self.close(self.stack[-1], at)
                self.close(self.stack[-1], end)
                return

    def handle_data(self, data):
        self.stack[-1].kids.append(data)


def parse(src: str) -> Node:
    b = _Builder(src)
    b.feed(src)
    while len(b.stack) > 1:
        b.close(b.stack[-1], len(src))
    return b.root


def canon(node: Node) -> str:
    """Canonical string of a parsed tree: the same document gives the same string whatever the formatting."""
    return "".join(_canon(k) for k in _strip_edges(_clean(node.kids), True))


def _clean(kids):
    out = []
    for k in kids:
        if isinstance(k, str):
            k = re.sub(r"\s+", " ", k)
            if not k.strip() and not out:
                continue
        out.append(k)
    return out


def _canon(k):
    if isinstance(k, str):
        return k
    attrs = "".join(f' {a}="{v}"' for a, v in sorted(k.attrs.items()))
    kids = _clean(k.kids)
    if k.tag in BLOCK or k.tag in ("a", "span", "b", "i"):
        kids = _strip_edges(kids, k.tag in BLOCK)
    inner = "".join(_canon(c) for c in kids)
    return f"<{k.tag}{attrs}>{inner}</{k.tag}>"


def _strip_edges(kids, block):
    kids = [k for k in kids if not (isinstance(k, str) and not k.strip() and (block and _between_blocks(kids, k)))]
    if block and kids and isinstance(kids[0], str):
        kids[0] = kids[0].lstrip()
    if block and kids and isinstance(kids[-1], str):
        kids[-1] = kids[-1].rstrip()
    return [k for k in kids if k != ""]


def _between_blocks(kids, k):
    i = next(j for j, x in enumerate(kids) if x is k)
    before = kids[i - 1] if i else None
    after = kids[i + 1] if i + 1 < len(kids) else None
    return all(x is None or (isinstance(x, Node) and x.tag in BLOCK) for x in (before, after))


def normalizza(src: str) -> str:
    return canon(parse(src))
