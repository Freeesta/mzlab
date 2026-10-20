"""Turns one section file into a Teoria-style HTML page (same head, CSS and nav as the chapters). Deterministic: no clock, no random ids."""
import re
import sys
from html import escape
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import tema_testa  # noqa: E402

from .blocchi import Parser  # noqa: E402
from .errori import LibroError  # noqa: E402

SKELETON = """<!doctype html>
<html lang="it" data-esteso><head><meta charset="utf-8">
<!--
@HEAD@
-->
<meta name="robots" content="noindex">
<title>@TITLE@ · Libro</title><link rel="stylesheet" href="teoria.css"><script src="tema.js"></script></head>
<body>
<div class="wrap"><nav id="side"></nav><main>
@BODY@
</main></div>
</body></html>
"""


TEORIA = """<!doctype html>
<html lang="it"><head><meta charset="utf-8">
@HEAD@<meta name="viewport" content="width=device-width,initial-scale=1">
<title>@TITLE@ · Teoria QqQ lab</title><link rel="icon" href="../favicon-32.png"><link rel="stylesheet" href="teoria.css"><script src="tema.js"></script></head>
<body>
<div class="wrap"><nav id="side"></nav><main>
@BODY@
</main></div>
@SCRIPTS@
</body></html>
"""


def converti(text: str, nome: str, pubblico: bool = False, teoria: bool = False) -> str:
    """Convert the Markdown of section `nome` (a file name such as A2_tracce.md) to a full HTML page.
    With `teoria` the page is a published Teoria chapter (own head, scripts from `<!-- script: x.js -->`) instead of a hidden Libro draft."""
    prefix = nome.split("_")[0].split(".")[0]
    p = Parser(nome, prefix, pubblico)
    p.ctx["teoria"] = teoria
    body = p.blocks(text.splitlines(), 1)
    if pubblico and p.ctx.get("ipotesi"):
        raise LibroError(nome, 1, f"{p.ctx['ipotesi']} unresolved [I] tag(s): not allowed in a public build")
    body = re.sub(r"\{\{(eq|fig):([\w-]+)\}\}", lambda m: _ref(p, m, nome), body)
    if not p.ctx["titoli"]:
        raise LibroError(nome, 1, "the section has no heading")
    title = p.page_title or re.sub(r"[`*$]", "", p.ctx["titoli"][0][1])
    if teoria:
        head = "".join(f"<!--{h}-->\n" for h in p.head)
        scripts = "".join(f'<script src="{x}"></script>' for x in ["../appname.js", "teoria.js", *p.scripts])
        page = TEORIA.replace("@HEAD@", head).replace("@TITLE@", escape(title)).replace("@BODY@", body).replace("@SCRIPTS@", scripts)
        return tema_testa.inject(page, "teoria")
    page = SKELETON.replace("@HEAD@", "\n".join(h.replace("--", "-") for h in p.head)).replace("@TITLE@", escape(title)).replace("@BODY@", body)
    return tema_testa.inject(page, "teoria")


def _ref(p: Parser, m: re.Match, nome: str) -> str:
    kind, rid = m.groups()
    label = p.ctx[kind].get(rid)
    if label is None:
        raise LibroError(nome, 1, f"reference @{kind}:{rid} points to nothing")
    return f"eq. {label}" if kind == "eq" else f"Figura {label}"
