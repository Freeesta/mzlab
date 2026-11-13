"""Inventory of Italian left in the code of mzLab: comments, identifiers, string keys/selectors and file/directory names.
Report only (exit 0): it measures the English migration, it does not gate it. Word list: tools/italian_words.tsv
(whole words, no stemming, so English words such as `controller` or `integrate` never match).

    python3 tools/check_english.py                 # counts per area
    python3 tools/check_english.py --elenco        # also one line per file (comments / identifiers / keys / name)
    python3 tools/check_english.py --file PATH     # every hit of one file, with line numbers
    python3 tools/check_english.py --rinomine      # proposed rename map (tools/english_rename_map.py; TSV: kind, old, new, note)
"""
from __future__ import annotations

import argparse
import io
import re
import subprocess
import sys
import tokenize
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORDS = ROOT / "tools" / "italian_words.tsv"
AREAS = [("crates/", "crates/"), ("mzlab/web/", "mzlab/web/"), ("flow/", "flow/"), ("TP_Mine/", "TP_Mine/"), ("tools/", "tools/"),
         ("tests_e2e/", "tests_e2e/"), ("tests/", "tests/"), ("docs/", "docs/"), ("mzlab/*.py", "mzlab/")]
# never read: third-party, generated bulk, data fixtures (their names are still checked)
SKIP_CONTENT = ("vendor/", "mzlab/web/esempi/", "mzlab/web/elements.js", "/target/", "docs/agenti/mappa.md", "graphify-out/")
SKIP_NAME = ("vendor/", "mzlab/web/esempi/", "graphify-out/")
C_LIKE = {".rs", ".js", ".mjs", ".ts", ".css"}
HASH = {".sh", ".toml", ".yml", ".yaml", ".txt"}
# Italian words that are also English (or Latin) words: they count only next to an unambiguous Italian word
AMBIGUOUS = set("""come dove male fine solo coda piano lotto uno prima zone due vie con per del non il unite verso porta legge pari
specie genere marca pila seme parte forte uni fini posa oro velo cosa nota conti vista tempo area linea idea vario banda fila
tasso ramo corso esca ancora gira largo lente modo stato tema tocca canale ci ad ed lo un da al le di la col ai si ci""".split())
# proper nouns and names of formats/products that stay as they are
PROPER = set("attributio tabui mzlab mzfinder tpmine federico rosina freeesta okabe schymanski patroon twiggy andi".split())
ACCENTED = re.compile(r"(?<!\w)(?:è|più|perché|già|può|cioè|così|né|là|ciò|finché|poiché|affinché|sì)(?!\w)")
QUOTED = re.compile(r"«[^»]*»|“[^”]*”|\"[^\"\n]*\"|`[^`\n]*`|(?<!\w)'[^'\s]+'")
WORD = re.compile(r"[A-Za-zÀ-ÿ]+")
# names of files, identifiers and keys inside a comment are not its language (they are renamed with the code)
CODEISH = re.compile(r"[\w$#@]*[._/\-:$#@(][\w$#@./\-:()]*|\b[a-z]+[A-Z]\w*|\b[A-Z]{2,}\w*|\b\w*_\w*")
KEYLIKE = re.compile(r"^[#.]?[A-Za-z][\w.\-/:#]*$")


def load_words(path: Path = WORDS) -> dict[str, str]:
    out = {}
    for ln in path.read_text(encoding="utf-8").splitlines():
        if ln and not ln.startswith("#"):
            it, _, en = ln.partition("\t")
            out[it.strip()] = en.strip()
    return out


DICT = load_words()


def subwords(name: str) -> list[str]:
    """`tabPicchiRiga` / `tab_picchi-riga` / `TabPicchi2` -> ['tab', 'picchi', 'riga']."""
    parts = re.split(r"[^A-Za-zÀ-ÿ]+|(?<=[a-zà-ÿ])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", name)
    return [p.lower() for p in parts if p]


def italian_words(words: list[str]) -> list[str]:
    """The Italian words of a name or a sentence; ambiguous ones count only next to a clear one."""
    clear = [w for w in words if w in DICT and w not in AMBIGUOUS and w not in PROPER]
    if not clear:
        return []
    return [w for w in words if w in DICT and w not in PROPER]


def italian_comment(text: str) -> bool:
    if ACCENTED.search(text):
        return True
    plain = CODEISH.sub(" ", QUOTED.sub(" ", text))
    words = [w.lower() for w in WORD.findall(plain)]
    hits = italian_words(words)       # a ratio, so an English line naming two Italian menu items is not Italian
    k = len(set(hits))
    return bool(hits) and (len(words) <= 3 or (k >= 2 and k >= 0.25 * len(set(words))))


REGEX_BEFORE = re.compile(r"(?:^|[(,=:\[!&|?{};+\-*%<>~^]|\breturn|\btypeof|\bcase)\s*$")
REGEX_LIT = re.compile(r"/((?:[^/\\\[\n]|\\.|\[(?:[^\]\\\n]|\\.)*\])+)/[a-z]*")


# ---- lexers: each returns (comments, identifiers, strings) as lists of (line, text)
def lex_c_like(src: str, ext: str):
    comments, idents, strings = [], [], []
    i, n, line = 0, len(src), 1
    line_comments = ext != ".css"
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif line_comments and src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            comments.append((line, src[i + 2:j]))
            i = j
        elif src.startswith("/*", i):
            j = src.find("*/", i + 2)
            j = n if j < 0 else j
            comments.append((line, src[i + 2:j]))
            line += src.count("\n", i, j)
            i = j + 2
        elif ext == ".rs" and re.match(r"r#*\"", src[i:i + 8]) and not (i and (src[i - 1].isalnum() or src[i - 1] == "_")):
            hashes = re.match(r"r(#*)\"", src[i:]).group(1)
            end = src.find('"' + hashes, i + len(hashes) + 2)
            end = n if end < 0 else end
            strings.append((line, src[i + len(hashes) + 2:end]))
            line += src.count("\n", i, end)
            i = end + 1 + len(hashes)
        elif c in "\"`" or (c == "'" and ext != ".rs"):
            j = i + 1
            while j < n and src[j] != c:
                if src[j] == "\\":
                    j += 1
                elif src[j] == "\n" and c != "`":
                    break
                j += 1
            strings.append((line, src[i + 1:j]))
            line += src.count("\n", i, j)
            i = j if j < n and src[j] == "\n" else j + 1          # an unclosed quote ends at the line end
        elif c == "/" and ext != ".rs" and ext != ".css" and REGEX_BEFORE.search(src[max(0, i - 40):i]):
            m = REGEX_LIT.match(src, i)
            if m:
                strings.append((line, m.group(1)))
                i = m.end()
            else:
                i += 1
        elif c == "'":                                   # Rust: char literal or lifetime
            m = re.match(r"'(?:\\.[^']*|[^\\'\n])'", src[i:i + 12])
            i += len(m.group(0)) if m else 1
        elif c.isalpha() or c in "_$":
            m = re.match(r"[\w$]+", src[i:])
            idents.append((line, m.group(0)))
            i += len(m.group(0))
        else:
            i += 1
    return comments, idents, strings


def lex_python(src: str):
    comments, idents, strings = [], [], []
    try:
        toks = list(tokenize.generate_tokens(io.StringIO(src).readline))
    except (tokenize.TokenError, SyntaxError, IndentationError):
        return lex_hash(src)
    prev = tokenize.NEWLINE
    for t in toks:
        doc = prev in (tokenize.NEWLINE, tokenize.INDENT, tokenize.DEDENT, tokenize.NL, tokenize.ENCODING)
        if t.type not in (tokenize.COMMENT, tokenize.NL):
            prev = t.type
        if t.type == tokenize.COMMENT:
            comments.append((t.start[0], t.string[1:]))
        elif t.type == tokenize.NAME:
            idents.append((t.start[0], t.string))
        elif t.type == tokenize.STRING:
            body = re.sub(r"^[rbuRBUfF]*('''|\"\"\"|'|\")", "", t.string)[:-1]
            (comments if doc and t.string.lstrip("rbuRBUfF").startswith(('"""', "'''")) else strings).append((t.start[0], body.strip("'\"")))
    return comments, idents, strings


def lex_hash(src: str):
    comments, idents, strings = [], [], []
    for k, ln in enumerate(src.splitlines(), 1):
        m = re.search(r"(?:^|\s)#(?!!)(.*)$", ln)
        code = ln[:m.start()] if m else ln
        if m:
            comments.append((k, m.group(1)))
        for s in re.findall(r"\"([^\"]*)\"|'([^']*)'", code):
            strings.append((k, s[0] or s[1]))
        for w in re.findall(r"[A-Za-z_][\w\-]*", re.sub(r"\"[^\"]*\"|'[^']*'", " ", code)):
            idents.append((k, w))
    return comments, idents, strings


def lex_html(src: str):
    comments, idents, strings = [], [], []
    for m in re.finditer(r"<!--(.*?)-->", src, re.DOTALL):
        comments.append((src.count("\n", 0, m.start()) + 1, m.group(1)))
    for m in re.finditer(r"<(script|style)\b[^>]*>(.*?)</\1>", src, re.DOTALL):
        base = src.count("\n", 0, m.start(2))
        for kind, out in zip(lex_c_like(m.group(2), ".js" if m.group(1) == "script" else ".css"), (comments, idents, strings)):
            out.extend((ln + base, t) for ln, t in kind)
    for m in re.finditer(r"\s((?:data-[\w-]+)|id|class|for|name|aria-controls)=\"([^\"]*)\"", src):
        ln = src.count("\n", 0, m.start()) + 1
        if m.group(1).startswith("data-"):
            idents.append((ln, m.group(1)))
        if m.group(1) != "name" and not m.group(1).startswith("data-i18n-"):
            idents.extend((ln, v) for v in m.group(2).split())
        elif m.group(1).startswith("data-i18n"):
            strings.append((ln, m.group(2)))
    return comments, idents, strings


def lex_lang(src: str):
    """lang/*.js: only the keys are code; the texts are the translations and stay."""
    return [], [], [(src.count("\n", 0, m.start()) + 1, m.group(1)) for m in re.finditer(r"^\s*\"([^\"]+)\"\s*:", src, re.M)]


def lex_json(src: str):
    return [], [], [(src.count("\n", 0, m.start()) + 1, m.group(1)) for m in re.finditer(r"\"([^\"\n]+)\"\s*:", src)]


def lex_markdown(src: str):
    """Docs: the prose (outside code fences) is counted apart; identifiers are not looked for."""
    out, fence = [], False
    for k, ln in enumerate(src.splitlines(), 1):
        if ln.lstrip().startswith("```"):
            fence = not fence
        elif not fence and ln.strip():
            out.append((k, re.sub(r"`[^`]*`", " ", ln)))
    return out, [], []


def lex(rel: str, src: str):
    ext = Path(rel).suffix.lower()
    if "/lang/" in rel and ext == ".js":
        return lex_lang(src)
    if ext in C_LIKE:
        return lex_c_like(src, ext)
    if ext == ".py":
        return lex_python(src)
    if ext in (".html", ".svg"):
        return lex_html(src)
    if ext == ".json" and not rel.startswith(("tests/", "tests_e2e/")) and "/tests/" not in rel:
        return lex_json(src)
    if ext == ".md":
        return lex_markdown(src)
    if ext in HASH or Path(rel).name in ("Dockerfile", ".gitignore", ".gitattributes"):
        return lex_hash(src)
    return [], [], []


JSDOC_BRACES = re.compile(r"\{[^{}\n]*\}")
JSDOC_NAME = re.compile(r"@(?:typedef|callback|property|param|prop)\s+\[?([\w$.]+)")


def split_jsdoc(comments: list, idents: list) -> list:
    """JSDoc types and names are identifiers: they move to `idents` and leave the comment text."""
    out = []
    for ln, t in comments:
        while True:                                       # innermost braces first: {{ a: Pannello }} nests
            m = JSDOC_BRACES.search(t)
            if not m:
                break
            idents.extend((ln, w) for w in re.findall(r"[A-Za-z_$][\w$]*", m.group(0)))
            t = t[:m.start()] + " " + t[m.end():]
        idents.extend((ln, w) for w in JSDOC_NAME.findall(t))
        out.append((ln, JSDOC_NAME.sub(" ", t)))
    return out


def scan_file(rel: str, src: str) -> dict:
    comments, idents, strings = lex(rel, src)
    if Path(rel).suffix in (".js", ".mjs", ".ts", ".html"):
        comments = split_jsdoc(comments, idents)
    prose = rel.endswith(".md")
    com = sorted({ln for ln, t in comments if italian_comment(t)})
    ids = {}
    for ln, name in idents:
        if name not in ids and italian_words(subwords(name)):
            ids[name] = ln
    keys = {}
    for ln, s in strings:
        s = s.strip()
        if s and s not in keys and KEYLIKE.match(s) and italian_words(subwords(s)):
            keys[s] = ln
    return {"prose" if prose else "comments": com, "idents": ids, "keys": keys}


def name_hits(rel: str) -> list[str]:
    """Italian words in the file name and in each directory of its path (ambiguous words count here: paths are code)."""
    out = []
    for comp in rel.split("/"):
        stem = comp if "." not in comp[1:] else comp[:comp.rfind(".")] if comp.count(".") == 1 else comp.split(".")[0]
        out += [w for w in subwords(stem) if w in DICT and w not in PROPER]
    return out


def area_of(rel: str) -> str:
    for label, pre in AREAS:
        if rel.startswith(pre) and (label != "mzlab/*.py" or "/" not in rel[len(pre):]):
            return label
    return "other"


def tracked() -> list[str]:
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.splitlines()
    return [f for f in out if not f.startswith(SKIP_NAME) and "/vendor/" not in f]


def scan(files: list[str]) -> dict[str, dict]:
    res = {}
    for rel in files:
        r = {"comments": [], "idents": {}, "keys": {}, "prose": [], "name": name_hits(rel)}
        p = ROOT / rel
        if not any(s in rel or rel.startswith(s) for s in SKIP_CONTENT) and p.is_file() and p.stat().st_size < 2_000_000:
            try:
                r.update(scan_file(rel, p.read_text(encoding="utf-8")))
            except UnicodeDecodeError:
                pass
        res[rel] = r
    return res


def summary(res: dict[str, dict]) -> dict[str, Counter]:
    per = defaultdict(Counter)
    for rel, r in res.items():
        a = per[area_of(rel)]
        a["files"] += 1
        a["comment_lines"] += len(r["comments"])
        a["prose_lines"] += len(r["prose"])
        a["identifiers"] += len(r["idents"])
        a["keys"] += len(r["keys"])
        a["it_names"] += bool(r["name"])
        a["files_with_hits"] += bool(r["comments"] or r["idents"] or r["keys"] or r["name"])
    return per


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--elenco", action="store_true", help="one line per file with hits")
    ap.add_argument("--file", help="every hit of one tracked file")
    ap.add_argument("--rinomine", action="store_true", help="print the proposed rename map (TSV)")
    a = ap.parse_args(argv)
    files = tracked()
    if a.rinomine:
        import english_rename_map               # here: it imports this module
        for row in english_rename_map.rename_map(files):
            print("\t".join(row))
        return 0
    if a.file:
        r = scan([a.file])[a.file]
        print(f"name: {' '.join(r['name']) or '-'}")
        for ln in r["comments"] + r["prose"]:
            print(f"{a.file}:{ln}: comment/prose")
        for kind in ("idents", "keys"):
            for name, ln in sorted(r[kind].items(), key=lambda x: x[1]):
                print(f"{a.file}:{ln}: {kind[:-1]} {name}")
        return 0
    res = scan(files)
    cols = ("files", "files_with_hits", "comment_lines", "identifiers", "keys", "it_names", "prose_lines")
    per = summary(res)
    print("area\t" + "\t".join(cols))
    for label in [x for x, _ in AREAS] + ["other"]:
        print(label + "\t" + "\t".join(str(per[label][c]) for c in cols))
    if a.elenco:
        for rel, r in res.items():
            if r["comments"] or r["idents"] or r["keys"] or r["name"] or r["prose"]:
                print(f"{rel}\tc={len(r['comments'])}\tid={len(r['idents'])}\tk={len(r['keys'])}\tp={len(r['prose'])}\tname={','.join(r['name'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
