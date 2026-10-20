"""Inline Markdown of the Libro dialect: `code`, $math$, **bold**, *italic*, _em_, ~sub~, ^sup^, [text](link.md#x), [@key; @key], @eq:id / @fig:id, [I].
Used by blocchi.py; raw HTML is never passed through (everything is escaped), so a page cannot carry script from the text."""
import re
from html import escape

from .errori import LibroError

CODE = re.compile(r"`([^`\n]+)`")
MATH = re.compile(r"(?<![\\$])\$([^$\n]+)\$")
CITE = re.compile(r"\[(@[\w:.-]+(?:;\s*@[\w:.-]+)*)\]")
LINK = re.compile(r"\[([^\]\n]+)\]\(([^)\s]+)\)")
REF = re.compile(r"(?<![\w\[;])@(eq|fig):([\w-]+)")
BOLD = re.compile(r"\*\*(.+?)\*\*")
ITAL = re.compile(r"(?<![\w*])\*(?!\s)([^*\n]+?)(?<!\s)\*(?![\w*])")
SUB = re.compile(r"~([^\s~]+)~")
SUP = re.compile(r"\^([^\s^]+)\^")
EM = re.compile(r"(?<![\w_])_(?!\s)([^_\n]+?)(?<!\s)_(?![\w_])")
SAFE_HREF = re.compile(r"^(https?://[^\s\"<>]+|#[\w-]+|[\w./-]+\.md(#[\w-]+)?)$")


def render(text: str, file: str, line: int, ctx: dict) -> str:
    """Return the HTML of one run of inline text; ctx collects citation keys, [I] count and eq/fig references."""
    hold: list[str] = []

    def keep(html: str) -> str:
        hold.append(html)
        return f"\x00{len(hold) - 1}\x00"

    text = CODE.sub(lambda m: keep(f"<code>{escape(m.group(1))}</code>"), text)
    text = MATH.sub(lambda m: keep(f'<span class="tex">{escape(m.group(1))}</span>'), text)
    text = escape(text, quote=False)
    text = CITE.sub(lambda m: keep(_cite(m.group(1), ctx)), text)
    text = LINK.sub(lambda m: keep(_link(m, file, line, ctx)), text)
    ctx["ipotesi"] = ctx.get("ipotesi", 0) + text.count("[I]")
    text = text.replace("[I]", keep('<span class="ipotesi" title="hypothesis, to be verified">[I]</span>'))
    text = REF.sub(lambda m: keep(f'<a class="ref" href="#{m.group(1)}-{m.group(2)}">{{{{{m.group(1)}:{m.group(2)}}}}}</a>'), text)
    text = BOLD.sub(r"<b>\1</b>", text)
    text = ITAL.sub(r"<i>\1</i>", text)
    text = EM.sub(r"<em>\1</em>", text)
    text = SUB.sub(r"<sub>\1</sub>", text)
    text = SUP.sub(r"<sup>\1</sup>", text)
    return re.sub(r"\x00(\d+)\x00", lambda m: hold[int(m.group(1))], text)


def _cite(body: str, ctx: dict) -> str:
    keys = [k.strip()[1:] for k in body.split(";")]
    ctx.setdefault("citazioni", []).extend(keys)
    return f'<span class="cit">[{escape("; ".join(keys))}]</span>'


def _link(m: re.Match, file: str, line: int, ctx: dict) -> str:
    label, href = m.group(1), m.group(2).replace("&amp;", "&")
    if not SAFE_HREF.match(href):
        raise LibroError(file, line, f"unsupported link target {href!r}")
    href = re.sub(r"\.md(#|$)", r".html\1", href)
    ext = ' target="_blank" rel="noopener"' if href.startswith("http") and ctx.get("teoria") else ""
    return f'<a href="{escape(href)}"{ext}>{label}</a>'
