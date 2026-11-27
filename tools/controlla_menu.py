"""Menu-label style check, called by tools/controlla_i18n.py: menu labels are short imperatives without articles or demonstratives.

The rule is in docs/agenti/languages.md. A label that legitimately has a determiner goes in MENU_ARTICLE_OK with the reason.
"""
from __future__ import annotations

import re
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"

# House style (docs/agenti/languages.md): menu items, palette commands and button labels are short imperative phrases, verb + object,
# without articles or demonstratives («Reset zoom», not «Reset the zoom»). Menu keys: these families, plus every key a script
# passes as `label:` / `name:` / `menuLabel:` of an entry.
MENU_KEY_RE = re.compile(r"^(ctx\.(?!.*\.(aria|prompt|tip|where|fail|number)$)|cmd\..*\.name$|files\.ctx\.|.*\.menu(\.|$)|.*\.menu[A-Z])")
MENU_LABEL_RE = re.compile(r"""\b(?:label|name|menuLabel)\s*[:=(]\s*[^"'`\n]{0,50}?(?:I18N\.t|\bT|\bt)\(\s*["']([A-Za-z0-9_.]+)["']""")
MENU_ARTICLE_RE = re.compile(r"(?<![\w'’-])(the|this|that|these|those|a|an)(?![\w'’-])", re.I)
# key -> why a determiner is right there (the text is not a menu label, or a fixed section name shared with tabs)
MENU_ARTICLE_OK = {
    "cmd.idn.name": "section name, same text as the tab bn.i9.idn",
    "cmd.hdr.name": "section name, same text as the tab bn.i9.hdr",
    "cmd.lst.name": "section name, same text as the tab bn.i9.lst",
    "load.exp.why.name": "explains where a value came from, not a command",
    "msngrafo.str.ev.label": "describes an event in the MSn graph, not a command",
    "ctx.ask.sug.label": "heading of a suggestion list inside a dialog, not an entry",
    "ctx.spec.noPeak": "information row of the menu (a hint), not a command",
    "draw.anal.stick": "text of a table cell with a value, not a command",
    "draw.quick.label": "label of an input field, a sentence",
    "draw.sub.ev.label": "describes what the evidence shows, not a command",
    "draw.sub.source.cat": "name of a source in a dialog, not a command",
}


def menu_keys(cat: dict) -> set[str]:
    keys = {k for k in cat if MENU_KEY_RE.match(k)}
    for p in WEB.glob("*.js"):
        if p.name != "elements.js":
            keys |= set(MENU_LABEL_RE.findall(p.read_text(encoding="utf-8", errors="replace")))
    return {k for k in keys & set(cat) if not k.endswith((".aria", ".tip", ".title", ".prompt"))}   # sentences, not labels


def check_menu_labels(cat: dict, errs: list) -> None:
    for k in sorted(menu_keys(cat)):
        plain = re.sub(r"\{[^{}]*\}|<[^>]+>", " ", cat[k])
        m = MENU_ARTICLE_RE.search(plain)
        if m and k not in MENU_ARTICLE_OK and len(cat[k]) <= 80:
            errs.append(f"menu label with «{m.group(1)}»: {k}: «{cat[k][:60]}» (short imperative, no articles or demonstratives; or list the key in MENU_ARTICLE_OK with the reason)")
