"""Reads the interface catalogs: mzlab/web/lang/<lang>/<area>.js, each one `I18N.add("<lang>", {...});` with one key per line.

Used by the checks and tests that look at the texts (lang/it.js and en.js are only the loaders of the browser)."""
import json
import re
from pathlib import Path

LANG = Path(__file__).resolve().parent.parent / "mzlab" / "web" / "lang"


def parti(lang: str) -> list[Path]:
    return sorted((LANG / lang).glob("*.js"))


def _without_duplicates(pairs: list, path: Path) -> dict[str, str]:
    seen: set[str] = set()
    for k, _ in pairs:
        if k in seen:
            raise ValueError(f"{path.name}: key {k} is defined twice in the same file")
        seen.add(k)
    return dict(pairs)


def parte(path: Path) -> dict[str, str]:
    lang = path.parent.name
    rows = path.read_text(encoding="utf-8").strip().splitlines()
    if rows[0] != f'I18N.add("{lang}", {{' or rows[-1].strip() != "});":
        raise ValueError(f'{path.name}: the first line must be `I18N.add("{lang}", {{` and the last `}});`')
    return json.loads("{" + "\n".join(rows[1:-1]) + "}", object_pairs_hook=lambda c: _without_duplicates(c, path))


def catalog(lang: str) -> dict[str, str]:
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


def contesto_problems(it: dict[str, str], param_re: str = r"\{[A-Za-z_]") -> list[str]:
    """Descriptions live in lang/contesto/<area>.json (same stem as the area file of the key); the description of a key no catalog has
    stays in orfane.json. Reports a description in the wrong file or twice, and a key with parameters (`{name`) that has none."""
    area = {k: p.stem for p in parti("it") for k in parte(p)}
    errs: list[str] = []
    seen: dict[str, str] = {}
    for f in sorted((LANG / "contesto").glob("*.json")):
        for k in json.loads(f.read_text(encoding="utf-8")):
            if k in seen:
                errs.append(f"contesto/{f.name}: description of {k} is also in contesto/{seen[k]}")
            seen[k] = f.name
            if f.stem != area.get(k, "orfane"):
                errs.append(f"contesto/{f.name}: description of {k} belongs in contesto/{area.get(k, 'orfane')}.json")
    return errs + [f"contesto/: missing description for the key with parameters: {k}" for k in sorted(it) if re.search(param_re, it[k]) and k not in seen]
