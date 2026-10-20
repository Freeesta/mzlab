"""Fixed script snippets a chapter can name with `<!-- inline: NAME -->`: the Markdown never carries free JavaScript.
`before` snippets go ahead of ../appname.js, `after` ones at the end of the page. Used by pagina.py and importa.py."""
from pathlib import Path

POSITION = {"modi": "before", "formulario-quiz": "after"}
DIR = Path(__file__).with_name("snippets")


def testo(name: str) -> str:
    return (DIR / f"{name}.html").read_text(encoding="utf-8").rstrip("\n")
