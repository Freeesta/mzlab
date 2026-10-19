"""The theme snippet that opens the <head> of every page of the site (app and Teoria): inline, synchronous, before any script or stylesheet,
so the very first frame already has the right background (no white flash in the dark theme, no dark flash in the light one).
`python3 tools/tema_testa.py` rewrites the block between the markers in every page; `--check` only verifies (used by tests/test_tema_testa.py)."""
import re
import sys
from pathlib import Path

WEB = Path(__file__).resolve().parent.parent / "mzlab" / "web"
OPEN, CLOSE = "<!--tema-->", "<!--/tema-->"
LIGHT = {"app": "#f7f8fa", "teoria": "#fbfbf9"}   # the --bg of each page in the light theme (the dark one is #14171b in both)
DARK = "#14171b"

# data-theme comes from the parent page (Teoria inside the app) or from the saved preference «mzlab.prefs» (the old «qqq.prefs» until migrated); «auto» sets
# nothing and the prefers-color-scheme rules decide. «no-tr» switches the colour transitions off until the second frame after load.
JS = ('(function(){var R=document.documentElement,t=null;try{if(parent!==window)t=parent.document.documentElement.getAttribute("data-theme")}catch(e){}'
      'if(!t)try{t=JSON.parse(localStorage.getItem("mzlab.prefs")||localStorage.getItem("qqq.prefs")||"{}").theme}catch(e){}'
      'if(t==="dark"||t==="light"){R.setAttribute("data-theme",t);R.style.colorScheme=t}'
      'R.classList.add("no-tr");addEventListener("load",function(){requestAnimationFrame(function(){requestAnimationFrame(function(){R.classList.remove("no-tr")})})})})()')


def snippet(kind: str) -> str:
    light = LIGHT[kind]
    css = (f"html{{background:{light}}}@media(prefers-color-scheme:dark){{html:not([data-theme=light]){{background:{DARK}}}}}"
           f"html[data-theme=dark]{{background:{DARK}}}html[data-theme=light]{{background:{light}}}"
           "html.no-tr *,html.no-tr *::before,html.no-tr *::after{transition:none!important}")
    return f'{OPEN}<meta name="color-scheme" content="light dark"><style>{css}</style><script>{JS}</script>{CLOSE}'


def pages():
    yield WEB / "index.html", "app"
    for p in sorted((WEB / "teoria").glob("*.html")):
        yield p, "teoria"


def inject(text: str, kind: str) -> str:
    text = re.sub(re.escape(OPEN) + r".*?" + re.escape(CLOSE), "", text, flags=re.S)
    m = re.search(r'<meta charset="utf-8">', text)
    if not m:
        raise SystemExit("no <meta charset> in page")
    return text[:m.end()] + snippet(kind) + text[m.end():]


def main() -> int:
    bad = 0
    for p, kind in pages():
        old = p.read_text(encoding="utf-8")
        new = inject(old, kind)
        if new != old:
            bad += 1
            if "--check" not in sys.argv:
                p.write_text(new, encoding="utf-8")
    return 1 if bad and "--check" in sys.argv else 0


if __name__ == "__main__":
    sys.exit(main())
