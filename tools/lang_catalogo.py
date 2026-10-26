"""Reads the interface catalogs: mzlab/web/lang/<lang>/<area>.js, each one `I18N.add("<lang>", {...});` with one key per line.

Used by the checks and tests that look at the texts (lang/it.js and en.js are only the loaders of the browser)."""
import json
from pathlib import Path

LANG = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "lang"


def parti(lang: str) -> list[Path]:
    return sorted((LANG / lang).glob("*.js"))


def parte(path: Path) -> dict[str, str]:
    lang = path.parent.name
    righe = path.read_text(encoding="utf-8").strip().splitlines()
    if righe[0] != f'I18N.add("{lang}", {{' or righe[-1].strip() != "});":
        raise ValueError(f'{path.name}: the first line must be `I18N.add("{lang}", {{` and the last `}});`')
    return json.loads("{" + "\n".join(righe[1:-1]) + "}")


def catalogo(lang: str) -> dict[str, str]:
    """All keys of a language; a key in two areas is an error."""
    out: dict[str, str] = {}
    for p in parti(lang):
        for k, v in parte(p).items():
            if k in out:
                raise ValueError(f"{p.name}: key {k} is also in another area file")
            out[k] = v
    return out


def testo(lang: str) -> str:
    """The catalog as one text (for the checks that look for a quoted key)."""
    return "\n".join(p.read_text(encoding="utf-8") for p in parti(lang))
