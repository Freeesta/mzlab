"""One-off importer: turns a published Teoria chapter (HTML) into the Libro Markdown source in mzlab/web/teoria/src/.
Python tools/libro/importa.py --html <chapter.html> --out <src/NN-slug.md> [--verifica]. Every block is converted to the dialect only if
the result is the same document (same canonical DOM); otherwise the block is kept as a verbatim ```html block. Prints the raw-block count."""
import argparse
import re
import sys
from html import unescape
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))
from tools.libro.blocchi import Parser  # noqa: E402
from tools.libro.dom import Node, normalizza, parse  # noqa: E402
from tools.libro.errori import LibroError  # noqa: E402
from tools.libro.pagina import converti  # noqa: E402

SPECIAL = re.compile(r"[*`$\[\]@\\<]")
LINE_START = re.compile(r"^([-*+>#|]|\d+[.)]|\$\$|`|:::|\?\?\?|<)")
INLINE = {"i": "*", "b": "**", "em": "_", "sub": "~", "sup": "^"}


def inline_md(node: Node):
    """Markdown of the inline content of `node`, or None if it holds something the dialect cannot say."""
    out = []
    for k in node.kids:
        if isinstance(k, str):
            t = re.sub(r"\s+", " ", k)
            if SPECIAL.search(t):
                return None
            out.append(t)
        elif k.tag in INLINE and not k.attrs:
            inner = inline_md(k)
            if inner is None or not inner.strip() or inner != inner.strip():
                return None
            out.append(f"{INLINE[k.tag]}{inner}{INLINE[k.tag]}")
        elif k.tag == "code" and not k.attrs and "`" not in k.text():
            out.append(f"`{k.text()}`")
        elif k.tag == "a":
            link = link_md(k)
            if link is None:
                return None
            out.append(link)
        else:
            return None
    return "".join(out)


def link_md(a: Node):
    href = a.attrs.get("href", "")
    inner = inline_md(a)
    if inner is None or not inner.strip():
        return None
    if re.fullmatch(r"https?://[^\s\"<>()]+", href) and a.attrs == {"href": href, "target": "_blank", "rel": "noopener"}:
        return f"[{inner}]({href})"
    local = re.fullmatch(r"([\w./-]+)\.html(#[\w-]+)?", href)
    if local and set(a.attrs) == {"href"}:
        return f"[{inner}]({local.group(1)}.md{local.group(2) or ''})"
    return None


def native(node: Node):
    """Markdown candidate for one top-level element, or None."""
    t, cls = node.tag, node.attrs.get("class")
    if t in ("h2", "h3") and not node.attrs:
        s = inline_md(node)
        return None if s is None else f"{'#' * int(t[1])} {s.strip()}"
    if t == "p" and (not node.attrs or node.attrs in ({"class": "lead"}, {"class": "note"})):
        s = inline_md(node)
        if s is None or not s.strip() or LINE_START.match(s.strip()):
            return None
        return (f"{{.{cls}}} " if cls else "") + s.strip()
    if t in ("ul", "ol") and not node.attrs:
        items = []
        for li in (k for k in node.kids if isinstance(k, Node)):
            s = inline_md(li) if li.tag == "li" and not li.attrs else None
            if s is None or not s.strip() or LINE_START.match(s.strip()):
                return None
            items.append(f"{'-' if t == 'ul' else '1.'} {s.strip()}")
        return "\n".join(items)
    if t == "details" and node.attrs == {"class": "q"}:
        return question_md(node)
    return None


def question_md(node: Node):
    kids = [k for k in node.kids if isinstance(k, Node)]
    if len(kids) != 2 or kids[0].tag != "summary" or kids[1].tag != "p" or kids[1].attrs:
        return None
    q, a = inline_md(kids[0]), inline_md(kids[1])
    if q is None or a is None or LINE_START.match(q.strip()):
        return None
    return f"??? {q.strip()}\n{a.strip()}"


def same(md: str, node: Node, src: str) -> bool:
    try:
        parser = Parser("x.md", "x", True)
        parser.ctx["teoria"] = True
        html = parser.blocks(md.splitlines(), 1)
    except LibroError:
        return False
    return normalizza(html) == normalizza(src[node.start:node.end])


def block_md(node: Node, src: str, state: dict) -> str:
    md = native(node)
    if md is not None and same(md, node, src):
        return md
    state["raw"] += 1
    return "```html\n" + src[node.start:node.end].strip() + "\n```"


def find(node: Node, tag: str, cls=None):
    for k in node.kids:
        if isinstance(k, Node):
            if k.tag == tag and (cls is None or k.attrs.get("class") == cls):
                return k
            r = find(k, tag, cls)
            if r:
                return r
    return None


def importa(src: str, nome: str):
    """Return (markdown, number of raw blocks) for the chapter HTML `src`."""
    root = parse(src)
    main = find(root, "main")
    head = find(root, "head")
    title = unescape(re.sub(r"\s*·\s*Teoria QqQ lab\s*$", "", find(head, "title").text()))
    state = {"raw": 0}
    parts, kids = [], [k for k in main.kids if isinstance(k, Node)]
    if not kids or kids[0].attrs != {"class": "kicker"} or kids[1].tag != "h1":
        raise LibroError(nome, 1, "the chapter does not start with kicker + h1")
    h1 = inline_md(kids[1])
    if h1 is None:
        raise LibroError(nome, 1, "the h1 holds markup the dialect cannot say")
    scripts = [Path(s.attrs["src"]).name for s in root_scripts(root) if s.attrs.get("src") not in ("../appname.js", "teoria.js")]
    meta = ["<!-- no-appname -->"] if not any(s.attrs.get("src") == "../appname.js" for s in root_scripts(root)) else []
    meta += [f"<!-- script: {s} -->" for s in scripts]
    if h1.strip() != title:
        meta.append(f"<!-- title: {title} -->")
    parts.append("\n".join(meta + [f"# {h1.strip()} {{kicker: {kids[0].text().strip()}}}"]))
    for k in kids[2:]:
        parts.append(block_md(k, src, state))
    return "\n\n".join(parts) + "\n", state["raw"]


def root_scripts(root: Node):
    body = find(root, "body")
    return [k for k in body.kids if isinstance(k, Node) and k.tag == "script"]


def con_viewport(src: str) -> str:
    """The generated chapters always carry the viewport meta; a chapter that lacks it is compared as if it had it."""
    if "viewport" in src:
        return src
    return src.replace("<title>", '<meta name="viewport" content="width=device-width,initial-scale=1">\n<title>', 1)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--html", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    a = ap.parse_args(argv)
    src = a.html.read_text(encoding="utf-8")
    md, raw = importa(src, a.out.name)
    gen = converti(md, a.out.name, pubblico=True, teoria=True)
    if normalizza(gen) != normalizza(con_viewport(src)):
        print(f"{a.html.name}: the generated page is not equivalent to the original", file=sys.stderr)
        return 1
    a.out.parent.mkdir(parents=True, exist_ok=True)
    a.out.write_text(md, encoding="utf-8", newline="\n")
    print(f"{a.html.name}: {md.count(chr(10))} lines, {raw} raw block(s); equivalent")
    return 0


if __name__ == "__main__":
    sys.exit(main())
