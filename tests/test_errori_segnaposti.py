"""Every engine error says what its text names: an `Error::new("key")` of the Rust code (tests aside) carries, in the `.with(...)` chain written after it,
each `{placeholder}` of that key's text in the Italian and English catalogs, so a reader never sees a literal «{name}». Keys whose text has no placeholder,
keys built at run time and keys without a catalog text are not checked here (`tools/controlla_i18n.py` covers the catalogs)."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANG = ROOT / "mzlab" / "web" / "lang"
ENTRY = re.compile(r'^\s*"([\w.\-]+)":\s*"((?:[^"\\]|\\.)*)"', re.M)
RAISE = re.compile(r'Error::new\(\s*"([\w.\-]+)"\s*\)')
WITH = re.compile(r'\s*\.with\(\s*"(\w+)"\s*,')


def placeholders() -> dict[str, set[str]]:
    need: dict[str, set[str]] = {}
    for lang in ("it", "en"):
        for f in sorted((LANG / lang).glob("*.js")):
            for key, text in ENTRY.findall(f.read_text(encoding="utf-8")):
                need.setdefault(key, set()).update(re.findall(r"\{(\w+)\}", text))
    return need


def chain(src: str, at: int) -> set[str]:
    """The parameter names of the `.with("name", ...)` calls that follow position `at`, each argument skipped up to its closing parenthesis."""
    given, pos = set(), at
    while (m := WITH.match(src, pos)) is not None:
        given.add(m.group(1))
        depth, q = 0, src.index("(", m.start())
        for q in range(q, len(src)):
            depth += {"(": 1, ")": -1}.get(src[q], 0)
            if depth == 0:
                break
        pos = q + 1
    return given


def sources():
    for f in sorted((ROOT / "crates").rglob("*.rs")):
        rel = f.relative_to(ROOT).as_posix()
        if "test" not in rel and "/target/" not in rel:
            yield rel, f.read_text(encoding="utf-8")


def test_every_raised_key_fills_the_placeholders_of_its_text():
    need = placeholders()
    assert "err.file.notAccepted" in need, "catalogs not found"
    missing = []
    for rel, src in sources():
        for m in RAISE.finditer(src):
            lack = need.get(m.group(1), set()) - chain(src, m.end())
            if lack:
                line = src.count("\n", 0, m.start()) + 1
                missing.append(f"{rel}:{line} {m.group(1)} lacks {sorted(lack)}")
    assert not missing, "errors raised without the values their text names:\n" + "\n".join(missing)


def test_the_chain_reader_follows_nested_arguments():
    src = 'Error::new("k").with("a", f(x, (1, 2))).with("b", y).map(z).with("c", 1)'
    m = RAISE.search(src)
    assert m and chain(src, m.end()) == {"a", "b"}
